use crate::{
    AdapterError, Fixture, KernelCompletion, KernelOperation, KernelRequest, Lifecycle,
    LifecycleState, MAX_TRACE_RECORDS, MAX_TRANSFER, NormalizedTraceRequest, PHYSICAL_BLOCK_SIZE,
    ProbeDisposition, ProbeReport, PublicationMetadataMatch, QUEUE_DEPTH, TagTable, TerminalResult,
    TraceLog, TraceRecord, borrowed_write_payload, classify_control_access,
    classify_publication_metadata, completion_was_delivered, map_kernel_completion,
    translate_request_for_slot, validate_shutdown_evidence,
};
use dwv_core::{
    ArrayId, BlockOp, BlockRequest, ByteRange, DurabilityIntent, FenceDomain, FrontendId,
    OrderingIntent, RequestId, SlotId, SubmissionSequence, TopologyEpoch,
};
use dwv_recovery::RecoveryStateStore;
use dwv_service::{HealthyPortableService, PublicationIdentity};
use dwv_store::RandomAccessStore;
use libublk::helpers::IoBuf;
use libublk::io::{UblkDev, UblkQueue};
use libublk::{BufDesc, UblkError, UblkFlags};
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);
static QUEUE_FAILED: AtomicBool = AtomicBool::new(false);

pub fn probe() -> ProbeReport {
    let kernel_release = command_output("uname", &["-r"]);
    let module_installed = kernel_release.as_deref().is_some_and(module_is_installed);
    let control = classify_control_access(
        OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/ublk-control")
            .map(|_| ()),
        module_installed,
    );
    ProbeReport {
        platform: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
        kernel_release,
        control,
        ext4_tools: tool_disposition("mkfs.ext4"),
        mount_tools: if executable_in_path("mount") && executable_in_path("umount") {
            ProbeDisposition::Present
        } else {
            ProbeDisposition::Unsupported("mount and umount are required".into())
        },
    }
}

fn execute_service<S: RandomAccessStore, R: RecoveryStateStore>(
    service: &mut HealthyPortableService<S, R>,
    request: BlockRequest,
    write_bytes: Option<&[u8]>,
) -> Result<Vec<u8>, AdapterError> {
    match request.op {
        BlockOp::Read => service
            .read(request)
            .map(|(bytes, _)| bytes)
            .map_err(|error| AdapterError::Io(error.to_string())),
        BlockOp::Write => {
            let bytes = write_bytes.ok_or(AdapterError::Invalid("write buffer is missing"))?;
            if bytes.len() as u64 != request.range.length {
                return Err(AdapterError::Invalid(
                    "write buffer length differs from request",
                ));
            }
            service
                .write(request, bytes)
                .map(|_| Vec::new())
                .map_err(|error| AdapterError::Io(error.to_string()))
        }
        BlockOp::Flush => service
            .flush(request)
            .map(|_| Vec::new())
            .map_err(|error| AdapterError::Io(error.to_string())),
        _ => Err(AdapterError::Unsupported("normalized operation")),
    }
}

fn flush_service<S: RandomAccessStore, R: RecoveryStateStore>(
    service: &mut HealthyPortableService<S, R>,
    request_id: u64,
    slot: SlotId,
    epoch: TopologyEpoch,
) -> Result<(), AdapterError> {
    let request = BlockRequest::new(
        RequestId(request_id),
        FrontendId(1),
        slot,
        epoch,
        BlockOp::Flush,
        ByteRange::empty(),
        None,
        OrderingIntent {
            submission_sequence: SubmissionSequence(request_id),
            preflush: false,
            fence_domain: FenceDomain(1),
        },
        DurabilityIntent::ExplicitFlush,
    );
    execute_service(service, request, None)
        .map(|_| ())
        .map_err(|error| AdapterError::ReconciliationRequired(error.to_string()))
}

struct ServeConfig {
    capacity: u64,
    epoch: TopologyEpoch,
    data_slot: SlotId,
    array_identity: Option<String>,
    publication_identity: String,
    fixture_digest: Option<String>,
    root: Option<PathBuf>,
    name: &'static str,
}

pub fn serve_with_publication<F>(
    root: &Path,
    device_id: i32,
    on_published: F,
) -> Result<serde_json::Value, AdapterError>
where
    F: FnOnce(&serde_json::Value) + Send + Sync + 'static,
{
    let fixture = Fixture::load(root)?;
    let digest = fixture.manifest().digest()?;
    let config = ServeConfig {
        capacity: fixture.manifest().protected_length,
        epoch: TopologyEpoch(fixture.manifest().topology_epoch),
        data_slot: crate::DATA_SLOT,
        array_identity: None,
        publication_identity: format!("fixture:{digest}"),
        fixture_digest: Some(digest),
        root: Some(root.to_path_buf()),
        name: "diskweave-demo",
    };
    let service = fixture.open()?.into_service();
    serve_backend(service, config, device_id, on_published)
}

pub fn serve_admitted_with_publication<S, R, F>(
    service: HealthyPortableService<S, R>,
    device_id: i32,
    on_published: F,
) -> Result<serde_json::Value, AdapterError>
where
    S: RandomAccessStore + Send + 'static,
    R: RecoveryStateStore + Send + 'static,
    F: FnOnce(&serde_json::Value) + Send + Sync + 'static,
{
    let topology = service.topology();
    if topology.profile().data_slots() != 1
        || topology.profile().parity_slots() != 1
        || topology.geometry().logical_block_size() != 512
    {
        return Err(AdapterError::Unsupported(
            "initial ublk publication requires one data member, one parity member, and 512-byte logical blocks",
        ));
    }
    let data_slot = topology
        .assignments()
        .iter()
        .find(|assignment| assignment.role() == dwv_core::MemberRole::Data)
        .map(dwv_core::TopologyAssignment::slot_id)
        .ok_or(AdapterError::Invalid("admitted topology has no data slot"))?;
    let config = ServeConfig {
        capacity: topology.geometry().protected_length(),
        epoch: topology.topology_epoch(),
        data_slot,
        array_identity: Some(array_identity_hex(topology.array_id())),
        publication_identity: service
            .publication_identity()
            .map_err(|error| AdapterError::Conflict(error.to_string()))?
            .hex(),
        fixture_digest: None,
        root: None,
        name: "diskweave",
    };
    serve_backend(service, config, device_id, on_published)
}

fn serve_backend<S, R, F>(
    service: HealthyPortableService<S, R>,
    config: ServeConfig,
    device_id: i32,
    on_published: F,
) -> Result<serde_json::Value, AdapterError>
where
    S: RandomAccessStore + Send + 'static,
    R: RecoveryStateStore + Send + 'static,
    F: FnOnce(&serde_json::Value) + Send + Sync + 'static,
{
    STOP_REQUESTED.store(false, Ordering::SeqCst);
    QUEUE_FAILED.store(false, Ordering::SeqCst);
    install_signal_handlers()?;
    let mut lifecycle = Lifecycle::default();
    lifecycle.transition(LifecycleState::Assembling)?;
    if config
        .root
        .as_ref()
        .is_some_and(|root| root.join("ready.json").exists())
    {
        return Err(AdapterError::ReconciliationRequired(
            "readiness record already exists; inspect and clean the owned endpoint".into(),
        ));
    }
    let owned = owned_devices(
        config.array_identity.as_deref(),
        &config.publication_identity,
    )?;
    if !owned.exact.is_empty() || !owned.conflicting.is_empty() {
        return Err(AdapterError::ReconciliationRequired(format!(
            "array publication ownership is not clean: exact={:?}, conflicting={:?}",
            owned.exact, owned.conflicting
        )));
    }
    let opened = Arc::new(Mutex::new(service));
    let tags = Arc::new(Mutex::new(TagTable::default()));
    let trace = Arc::new(Mutex::new(TraceLog::default()));
    let sequence = Arc::new(AtomicU64::new(0));
    let capacity = config.capacity;
    let epoch = config.epoch;
    let data_slot = config.data_slot;

    let ctrl = libublk::ctrl::UblkCtrlBuilder::default()
        .name(config.name)
        .id(device_id)
        .nr_queues(1)
        .depth(QUEUE_DEPTH as u16)
        .io_buf_bytes(MAX_TRANSFER as u32)
        .dev_flags(UblkFlags::UBLK_DEV_F_ADD_DEV)
        .build()
        .map_err(ublk_io)?;

    let target_identity = config.publication_identity.clone();
    let target_fixture_digest = config.fixture_digest.clone();
    let target_array_identity = config.array_identity.clone();
    let target_init = move |dev: &mut UblkDev| {
        dev.tgt.dev_size = capacity;
        dev.tgt.params = libublk::sys::ublk_params {
            types: libublk::sys::UBLK_PARAM_TYPE_BASIC,
            basic: libublk::sys::ublk_param_basic {
                attrs: libublk::sys::UBLK_ATTR_VOLATILE_CACHE,
                logical_bs_shift: 9,
                physical_bs_shift: PHYSICAL_BLOCK_SIZE.ilog2() as u8,
                io_opt_shift: PHYSICAL_BLOCK_SIZE.ilog2() as u8,
                io_min_shift: 9,
                max_sectors: (MAX_TRANSFER / 512) as u32,
                dev_sectors: capacity / 512,
                ..Default::default()
            },
            ..Default::default()
        };
        dev.set_target_json(json!({
            "contract": "dwv.ublk.target.v2",
            "array_identity": target_array_identity,
            "publication_identity": target_identity,
            "fixture_digest": target_fixture_digest,
        }));
        Ok(())
    };

    let queue_opened = Arc::clone(&opened);
    let queue_tags = Arc::clone(&tags);
    let queue_trace = Arc::clone(&trace);
    let queue_sequence = Arc::clone(&sequence);
    let queue_fn = move |qid: u16, dev: &UblkDev| {
        run_queue(
            qid,
            dev,
            Arc::clone(&queue_opened),
            Arc::clone(&queue_tags),
            Arc::clone(&queue_trace),
            Arc::clone(&queue_sequence),
            capacity,
            epoch,
            data_slot,
        );
    };

    let ready_root = config.root.clone();
    let ready_identity = config.publication_identity.clone();
    let ready_fixture_digest = config.fixture_digest.clone();
    lifecycle.transition(LifecycleState::Published)?;
    let run_result = ctrl.run_target(target_init, queue_fn, move |running| {
        let id = running.dev_info().dev_id;
        let ready = json!({
            "schema": "dwv.ublk.ready.v2",
            "device_id": id,
            "device_path": format!("/dev/ublkb{id}"),
            "publication_identity": ready_identity,
            "fixture_digest": ready_fixture_digest,
            "queue_count": 1,
            "queue_depth": QUEUE_DEPTH,
            "maximum_transfer": MAX_TRANSFER,
        });
        if let Some(root) = &ready_root
            && let Ok(bytes) = serde_json::to_vec_pretty(&ready)
        {
            let _ = fs::write(root.join("ready.json"), bytes);
        }
        on_published(&ready);
        while !STOP_REQUESTED.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(25));
        }
        let _ = running.stop_dev();
    });
    lifecycle.transition(LifecycleState::AdmissionClosed)?;
    lifecycle.transition(LifecycleState::Draining)?;
    let trace_identity = config
        .fixture_digest
        .clone()
        .unwrap_or_else(|| config.publication_identity.clone());
    let trace_document = trace
        .lock()
        .map_err(|_| AdapterError::Io("trace lock poisoned".into()))?
        .document(trace_identity, capacity, epoch.0)?;
    let replay = trace_document.replay()?;
    let trace_bytes = trace_document.to_json()?;
    if let Some(root) = &config.root {
        fs::write(root.join("trace.json"), &trace_bytes)
            .map_err(|error| AdapterError::Io(error.to_string()))?;
    }
    let trace_digest = blake3::hash(&trace_bytes).to_hex().to_string();
    let trace_count = replay.record_count;
    if let Err(error) = run_result {
        lifecycle.transition(LifecycleState::AwaitingReconciliation)?;
        return Err(ublk_io(error));
    }
    if QUEUE_FAILED.load(Ordering::SeqCst) {
        lifecycle.transition(LifecycleState::AwaitingReconciliation)?;
        return Err(AdapterError::ReconciliationRequired(
            "ublk queue stopped after an I/O or completion failure".into(),
        ));
    }
    if !replay.clean {
        lifecycle.transition(LifecycleState::AwaitingReconciliation)?;
        return Err(AdapterError::ReconciliationRequired(
            "frontend trace contains exhausted or abandoned work".into(),
        ));
    }
    let drained = tags
        .lock()
        .map_err(|_| AdapterError::ReconciliationRequired("tag table lock poisoned".into()))?
        .is_empty();
    if !drained {
        lifecycle.transition(LifecycleState::AwaitingReconciliation)?;
        return Err(AdapterError::ReconciliationRequired(
            "queue stopped with active tags".into(),
        ));
    }
    let mut service = opened
        .lock()
        .map_err(|_| AdapterError::ReconciliationRequired("service lock poisoned".into()))?;
    flush_service(
        &mut service,
        sequence.fetch_add(1, Ordering::SeqCst) + 1,
        config.data_slot,
        config.epoch,
    )?;
    drop(service);
    ctrl.del_dev().map_err(ublk_reconcile)?;
    validate_shutdown_evidence(drained, true, true)?;
    if let Some(root) = &config.root {
        fs::remove_file(root.join("ready.json")).map_err(|error| {
            AdapterError::ReconciliationRequired(format!(
                "endpoint removed but readiness cleanup failed: {error}"
            ))
        })?;
    }
    drop(opened);
    lifecycle.transition(LifecycleState::Stopped)?;
    Ok(json!({
        "shutdown": "clean",
        "state": lifecycle.state(),
        "publication_identity": config.publication_identity,
        "fixture_digest": config.fixture_digest,
        "trace_count": trace_count,
        "trace_bound": MAX_TRACE_RECORDS,
        "trace_digest": trace_digest,
    }))
}

pub fn live_publication(root: &Path) -> Result<Option<serde_json::Value>, AdapterError> {
    let fixture = Fixture::load(root)?;
    live_publication_identity(None, &format!("fixture:{}", fixture.manifest().digest()?))
}

/// dwv:req req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow
pub fn live_admitted_publication(
    array_identity: ArrayId,
    identity: PublicationIdentity,
) -> Result<Option<serde_json::Value>, AdapterError> {
    let array_identity = array_identity_hex(array_identity);
    live_publication_identity(Some(&array_identity), &identity.hex())
}

pub fn cleanup(root: &Path, device_id: u32) -> Result<serde_json::Value, AdapterError> {
    let fixture = Fixture::load(root)?;
    let expected = fixture.manifest().digest()?;
    let publication_identity = format!("fixture:{expected}");
    let ctrl = libublk::ctrl::UblkCtrl::new_simple(device_id as i32).map_err(ublk_io)?;
    let target = ctrl
        .get_target_data_from_json()
        .ok_or_else(|| AdapterError::Conflict("device has no exported target metadata".into()))?;
    if target.get("contract").and_then(serde_json::Value::as_str) != Some("dwv.ublk.target.v2")
        || target
            .get("publication_identity")
            .and_then(serde_json::Value::as_str)
            != Some(publication_identity.as_str())
    {
        return Err(AdapterError::Conflict(
            "device target metadata does not match the fixture".into(),
        ));
    }
    let _ = ctrl.stop_dev();
    ctrl.del_dev().map_err(ublk_reconcile)?;
    let ready = fs::canonicalize(root)
        .map_err(|error| AdapterError::Io(error.to_string()))?
        .join("ready.json");
    if ready.exists() {
        fs::remove_file(ready).map_err(|error| AdapterError::Io(error.to_string()))?;
    }
    Ok(json!({
        "cleanup": "removed-owned-endpoint",
        "device_id": device_id,
        "fixture_digest": expected,
    }))
}

fn run_queue<S, R>(
    qid: u16,
    dev: &UblkDev,
    opened: Arc<Mutex<HealthyPortableService<S, R>>>,
    tags: Arc<Mutex<TagTable>>,
    trace: Arc<Mutex<TraceLog>>,
    sequence: Arc<AtomicU64>,
    capacity: u64,
    epoch: TopologyEpoch,
    data_slot: SlotId,
) where
    S: RandomAccessStore + Send + 'static,
    R: RecoveryStateStore + Send + 'static,
{
    let queue = match UblkQueue::new(qid, dev) {
        Ok(queue) => Rc::new(queue),
        Err(_) => return,
    };
    let executor = Rc::new(smol::LocalExecutor::new());
    let mut tasks = Vec::with_capacity(QUEUE_DEPTH);
    for tag in 0..QUEUE_DEPTH as u16 {
        let queue = Rc::clone(&queue);
        let opened = Arc::clone(&opened);
        let tags = Arc::clone(&tags);
        let trace = Arc::clone(&trace);
        let sequence = Arc::clone(&sequence);
        tasks.push(executor.spawn(async move {
            match io_task(
                &queue, tag, opened, tags, trace, sequence, capacity, epoch, data_slot,
            )
            .await
            {
                Ok(()) | Err(UblkError::QueueIsDown) => {}
                Err(_) => {
                    QUEUE_FAILED.store(true, Ordering::SeqCst);
                    STOP_REQUESTED.store(true, Ordering::SeqCst);
                }
            }
        }));
    }
    let run_executor = Rc::clone(&executor);
    smol::block_on(executor.run(async move {
        let run_tasks = || while run_executor.try_tick() {};
        let done = || tasks.iter().all(smol::Task::is_finished);
        if libublk::wait_and_handle_io_events(&queue, Some(20), run_tasks, done)
            .await
            .is_err()
        {
            QUEUE_FAILED.store(true, Ordering::SeqCst);
            STOP_REQUESTED.store(true, Ordering::SeqCst);
        }
    }));
}

async fn io_task<S, R>(
    queue: &UblkQueue<'_>,
    tag: u16,
    opened: Arc<Mutex<HealthyPortableService<S, R>>>,
    tags: Arc<Mutex<TagTable>>,
    trace: Arc<Mutex<TraceLog>>,
    sequence: Arc<AtomicU64>,
    capacity: u64,
    epoch: TopologyEpoch,
    data_slot: SlotId,
) -> Result<(), UblkError>
where
    S: RandomAccessStore + Send + 'static,
    R: RecoveryStateStore + Send + 'static,
{
    let mut buffer = IoBuf::<u8>::new(MAX_TRANSFER as usize);
    queue
        .submit_io_prep_cmd(tag, BufDesc::Slice(buffer.as_slice()), 0, Some(&buffer))
        .await?;
    loop {
        let descriptor = queue.get_iod(tag);
        let operation = kernel_operation(descriptor.op_flags & 0xff);
        let raw = KernelRequest {
            operation,
            flags: descriptor.op_flags & !0xff,
            start_sector: descriptor.start_sector,
            sectors: descriptor.nr_sectors,
            tag,
        };
        let sequence_number = sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let reservation = match trace
            .lock()
            .map_err(|_| UblkError::OtherError(-libc::EIO))?
            .reserve()
        {
            Ok(reservation) => reservation,
            Err(error) => {
                QUEUE_FAILED.store(true, Ordering::SeqCst);
                STOP_REQUESTED.store(true, Ordering::SeqCst);
                queue
                    .submit_io_commit_cmd(tag, BufDesc::Slice(buffer.as_slice()), errno(&error))
                    .await?;
                continue;
            }
        };
        let token = tags
            .lock()
            .map_err(|_| UblkError::OtherError(-libc::EIO))?
            .reserve(tag);
        let (token, generation, normalized_request, semantic_result, completed_length) = match token
        {
            Ok(token) => {
                match translate_request_for_slot(
                    raw,
                    capacity,
                    epoch,
                    data_slot,
                    SubmissionSequence(sequence_number),
                    token.generation,
                ) {
                    Ok(translated) => {
                        let normalized = NormalizedTraceRequest::from_translated(translated)
                            .map_err(|_| UblkError::OtherError(-libc::EIO))?;
                        let execution = borrowed_write_payload(
                            operation,
                            translated.data_length,
                            buffer.as_slice(),
                        )
                        .and_then(|write| match opened.lock() {
                            Ok(mut service) => {
                                execute_service(&mut service, translated.normalized, write)
                            }
                            Err(_) => Err(AdapterError::Io("service lock poisoned".into())),
                        });
                        match execution {
                            Ok(bytes) => {
                                if operation == KernelOperation::Read {
                                    buffer.as_mut_slice()[..bytes.len()].copy_from_slice(&bytes);
                                }
                                (
                                    Some(token),
                                    token.generation,
                                    Some(normalized),
                                    TerminalResult::Success,
                                    Some(translated.data_length as u64),
                                )
                            }
                            Err(error) => (
                                Some(token),
                                token.generation,
                                Some(normalized),
                                error.terminal(),
                                None,
                            ),
                        }
                    }
                    Err(error) => (Some(token), token.generation, None, error.terminal(), None),
                }
            }
            Err(error) => (None, 0, None, error.terminal(), None),
        };
        let planned_completion = map_kernel_completion(semantic_result, completed_length)
            .unwrap_or(KernelCompletion::MappingRefused);
        if planned_completion == KernelCompletion::MappingRefused {
            QUEUE_FAILED.store(true, Ordering::SeqCst);
            STOP_REQUESTED.store(true, Ordering::SeqCst);
        }
        let submit = queue
            .submit_io_commit_cmd(
                tag,
                BufDesc::Slice(buffer.as_slice()),
                completion_code(planned_completion),
            )
            .await;
        // libublk combines completion with the next fetch. After consumers unmount, a requested
        // stop may abort that next fetch even though the preceding completion was observed.
        let delivered = completion_was_delivered(
            submit.is_ok(),
            matches!(&submit, Err(UblkError::QueueIsDown)),
            STOP_REQUESTED.load(Ordering::SeqCst),
            QUEUE_FAILED.load(Ordering::SeqCst),
        );
        trace
            .lock()
            .map_err(|_| UblkError::OtherError(-libc::EIO))?
            .complete(
                reservation,
                TraceRecord {
                    sequence: sequence_number,
                    tag,
                    generation,
                    kernel_submission: raw,
                    normalized_request,
                    semantic_result,
                    kernel_completion: if delivered {
                        planned_completion
                    } else {
                        KernelCompletion::Abandoned
                    },
                },
            )
            .map_err(|_| UblkError::OtherError(-libc::EIO))?;
        if delivered {
            if let Some(token) = token {
                tags.lock()
                    .map_err(|_| UblkError::OtherError(-libc::EIO))?
                    .complete(token)
                    .map_err(|_| UblkError::OtherError(-libc::EIO))?;
            }
            // Trace retention is diagnostic only; release its slot after the operation/tag
            // owner completes so a terminal record cannot race live-resource reuse.
            trace
                .lock()
                .map_err(|_| UblkError::OtherError(-libc::EIO))?
                .mark_reclaimable(reservation)
                .map_err(|_| UblkError::OtherError(-libc::EIO))?;
        }
    }
}

fn kernel_operation(operation: u32) -> KernelOperation {
    match operation {
        libublk::sys::UBLK_IO_OP_READ => KernelOperation::Read,
        libublk::sys::UBLK_IO_OP_WRITE => KernelOperation::Write,
        libublk::sys::UBLK_IO_OP_FLUSH => KernelOperation::Flush,
        libublk::sys::UBLK_IO_OP_DISCARD => KernelOperation::Discard,
        libublk::sys::UBLK_IO_OP_WRITE_ZEROES | libublk::sys::UBLK_IO_OP_WRITE_SAME => {
            KernelOperation::WriteZeroes
        }
        libublk::sys::UBLK_IO_OP_ZONE_OPEN
        | libublk::sys::UBLK_IO_OP_ZONE_CLOSE
        | libublk::sys::UBLK_IO_OP_ZONE_FINISH
        | libublk::sys::UBLK_IO_OP_ZONE_APPEND
        | libublk::sys::UBLK_IO_OP_ZONE_RESET_ALL
        | libublk::sys::UBLK_IO_OP_ZONE_RESET
        | libublk::sys::UBLK_IO_OP_REPORT_ZONES => KernelOperation::Zoned,
        _ => KernelOperation::Unknown,
    }
}

fn errno(error: &AdapterError) -> i32 {
    terminal_errno(error.terminal())
}

fn completion_code(completion: KernelCompletion) -> i32 {
    match completion {
        KernelCompletion::Success { bytes } => i32::try_from(bytes).unwrap_or(-libc::EIO),
        KernelCompletion::Error { terminal } => terminal_errno(terminal),
        KernelCompletion::Abandoned | KernelCompletion::MappingRefused => -libc::EIO,
    }
}

fn terminal_errno(terminal: TerminalResult) -> i32 {
    match terminal {
        TerminalResult::Unsupported => -libc::EOPNOTSUPP,
        TerminalResult::Invalid => -libc::EINVAL,
        TerminalResult::ResourceExhausted => -libc::EAGAIN,
        TerminalResult::Io | TerminalResult::ReconciliationRequired => -libc::EIO,
        TerminalResult::Success => 0,
    }
}
#[derive(Default)]
struct OwnedDevices {
    exact: Vec<u32>,
    conflicting: Vec<u32>,
}

fn owned_devices(
    array_identity: Option<&str>,
    publication_identity: &str,
) -> Result<OwnedDevices, AdapterError> {
    let array_identity = array_identity.map(str::to_owned);
    let publication_identity = publication_identity.to_owned();
    let owned = Arc::new(Mutex::new(OwnedDevices::default()));
    let observed = Arc::clone(&owned);
    libublk::ctrl::UblkCtrl::for_each_dev_id(move |id| {
        let Ok(ctrl) = libublk::ctrl::UblkCtrl::new_simple(id as i32) else {
            return;
        };
        let Some(target) = ctrl
            .get_target_data_from_json()
            .and_then(|value| value.get("target").cloned())
        else {
            return;
        };
        let Ok(mut observed) = observed.lock() else {
            return;
        };
        match classify_publication_metadata(
            &target,
            array_identity.as_deref(),
            &publication_identity,
        ) {
            PublicationMetadataMatch::Exact => observed.exact.push(id),
            PublicationMetadataMatch::Conflict => observed.conflicting.push(id),
            PublicationMetadataMatch::Unrelated => {}
        }
    });
    Arc::try_unwrap(owned)
        .map_err(|_| AdapterError::Io("publication discovery ownership did not close".into()))?
        .into_inner()
        .map_err(|_| AdapterError::Io("publication discovery lock poisoned".into()))
}

fn live_publication_identity(
    array_identity: Option<&str>,
    publication_identity: &str,
) -> Result<Option<serde_json::Value>, AdapterError> {
    let owned = owned_devices(array_identity, publication_identity)?;
    if !owned.conflicting.is_empty() {
        return Err(AdapterError::ReconciliationRequired(format!(
            "array has conflicting live publication metadata on devices {:?}",
            owned.conflicting
        )));
    }
    match owned.exact.as_slice() {
        [] => Ok(None),
        [device_id] => Ok(Some(json!({
            "device_id": device_id,
            "device_path": format!("/dev/ublkb{device_id}"),
            "publication_identity": publication_identity,
            "array_identity": array_identity,
        }))),
        _ => Err(AdapterError::ReconciliationRequired(format!(
            "publication identity is live on multiple devices {:?}",
            owned.exact
        ))),
    }
}

fn array_identity_hex(identity: ArrayId) -> String {
    identity
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

extern "C" fn request_stop(_signal: libc::c_int) {
    STOP_REQUESTED.store(true, Ordering::SeqCst);
}

fn install_signal_handlers() -> Result<(), AdapterError> {
    // The handler performs only one lock-free atomic store; cleanup remains in normal context.
    unsafe {
        if libc::signal(
            libc::SIGINT,
            request_stop as *const () as libc::sighandler_t,
        ) == libc::SIG_ERR
            || libc::signal(
                libc::SIGTERM,
                request_stop as *const () as libc::sighandler_t,
            ) == libc::SIG_ERR
        {
            return Err(AdapterError::Io("failed to install signal handlers".into()));
        }
    }
    Ok(())
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    std::process::Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn module_is_installed(release: &str) -> bool {
    let base = PathBuf::from("/lib/modules").join(release);
    [
        "kernel/drivers/block/ublk_drv.ko",
        "kernel/drivers/block/ublk_drv.ko.xz",
        "kernel/drivers/block/ublk_drv.ko.zst",
    ]
    .iter()
    .any(|relative| base.join(relative).exists())
}

fn tool_disposition(name: &str) -> ProbeDisposition {
    if executable_in_path(name) {
        ProbeDisposition::Present
    } else {
        ProbeDisposition::Unsupported(format!("{name} is not installed"))
    }
}

fn executable_in_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| {
            let path = directory.join(name);
            path.metadata().is_ok_and(|metadata| {
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            })
        })
    })
}

fn ublk_io(error: UblkError) -> AdapterError {
    AdapterError::Io(error.to_string())
}

fn ublk_reconcile(error: UblkError) -> AdapterError {
    AdapterError::ReconciliationRequired(error.to_string())
}
