use crate::{
    AdapterError, Fixture, KernelCompletion, KernelOperation, KernelRequest, Lifecycle,
    LifecycleState, MAX_TRACE_RECORDS, MAX_TRANSFER, NormalizedTraceRequest, PHYSICAL_BLOCK_SIZE,
    ProbeDisposition, ProbeReport, QUEUE_DEPTH, TagTable, TerminalResult, TraceLog, TraceRecord,
    borrowed_write_payload, classify_control_access, completion_was_delivered,
    map_kernel_completion, translate_request, validate_shutdown_evidence,
};
use dwv_core::{SubmissionSequence, TopologyEpoch};
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

pub fn serve(root: &Path, device_id: i32) -> Result<serde_json::Value, AdapterError> {
    STOP_REQUESTED.store(false, Ordering::SeqCst);
    QUEUE_FAILED.store(false, Ordering::SeqCst);
    install_signal_handlers()?;
    let mut lifecycle = Lifecycle::default();
    lifecycle.transition(LifecycleState::Assembling)?;
    let fixture = Fixture::load(root)?;
    let digest = fixture.manifest().digest()?;
    let capacity = fixture.manifest().protected_length;
    let epoch = TopologyEpoch(fixture.manifest().topology_epoch);
    let root = fs::canonicalize(root).map_err(|error| AdapterError::Io(error.to_string()))?;
    if root.join("ready.json").exists() {
        return Err(AdapterError::ReconciliationRequired(
            "readiness record already exists; inspect and clean the owned endpoint".into(),
        ));
    }
    let conflicts = owned_devices(&digest);
    if !conflicts.is_empty() {
        return Err(AdapterError::ReconciliationRequired(format!(
            "owned ublk endpoint already exists: {conflicts:?}"
        )));
    }
    let opened = Arc::new(Mutex::new(fixture.open()?));
    let tags = Arc::new(Mutex::new(TagTable::default()));
    let trace = Arc::new(Mutex::new(TraceLog::default()));
    let sequence = Arc::new(AtomicU64::new(0));

    let ctrl = libublk::ctrl::UblkCtrlBuilder::default()
        .name("diskweave-demo")
        .id(device_id)
        .nr_queues(1)
        .depth(QUEUE_DEPTH as u16)
        .io_buf_bytes(MAX_TRANSFER as u32)
        .dev_flags(UblkFlags::UBLK_DEV_F_ADD_DEV)
        .build()
        .map_err(ublk_io)?;

    let target_digest = digest.clone();
    let target_root = root.clone();
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
            "contract": "dwv.ublk.target.v1",
            "fixture_digest": target_digest,
            "fixture_root_digest": blake3::hash(target_root.as_os_str().as_encoded_bytes()).to_hex().to_string(),
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
        );
    };

    let ready_root = root.clone();
    let ready_digest = digest.clone();
    lifecycle.transition(LifecycleState::Published)?;
    let run_result = ctrl.run_target(target_init, queue_fn, move |running| {
        let id = running.dev_info().dev_id;
        let ready = json!({
            "schema": "dwv.ublk.ready.v1",
            "device_id": id,
            "device_path": format!("/dev/ublkb{id}"),
            "fixture_digest": ready_digest,
            "queue_count": 1,
            "queue_depth": QUEUE_DEPTH,
            "maximum_transfer": MAX_TRANSFER,
        });
        if let Ok(bytes) = serde_json::to_vec_pretty(&ready) {
            let _ = fs::write(ready_root.join("ready.json"), bytes);
        }
        while !STOP_REQUESTED.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(25));
        }
        let _ = running.stop_dev();
    });
    lifecycle.transition(LifecycleState::AdmissionClosed)?;
    lifecycle.transition(LifecycleState::Draining)?;
    let trace_document = trace
        .lock()
        .map_err(|_| AdapterError::Io("trace lock poisoned".into()))?
        .document(digest.clone(), capacity, epoch.0)?;
    let replay = trace_document.replay()?;
    let trace_bytes = trace_document.to_json()?;
    fs::write(root.join("trace.json"), &trace_bytes)
        .map_err(|error| AdapterError::Io(error.to_string()))?;
    let trace_digest = blake3::hash(&trace_bytes).to_hex().to_string();
    let trace_count = replay.record_count;
    if let Err(error) = run_result {
        lifecycle.transition(LifecycleState::ReconciliationRequired)?;
        return Err(ublk_io(error));
    }
    if QUEUE_FAILED.load(Ordering::SeqCst) {
        lifecycle.transition(LifecycleState::ReconciliationRequired)?;
        return Err(AdapterError::ReconciliationRequired(
            "ublk queue stopped after an I/O or completion failure".into(),
        ));
    }
    if !replay.clean {
        lifecycle.transition(LifecycleState::ReconciliationRequired)?;
        return Err(AdapterError::ReconciliationRequired(
            "frontend trace contains exhausted or abandoned work".into(),
        ));
    }
    let drained = tags
        .lock()
        .map_err(|_| AdapterError::ReconciliationRequired("tag table lock poisoned".into()))?
        .is_empty();
    if !drained {
        lifecycle.transition(LifecycleState::ReconciliationRequired)?;
        return Err(AdapterError::ReconciliationRequired(
            "queue stopped with active tags".into(),
        ));
    }
    opened
        .lock()
        .map_err(|_| AdapterError::ReconciliationRequired("service lock poisoned".into()))?
        .flush(sequence.fetch_add(1, Ordering::SeqCst) + 1)?;
    let checkpointed = true;
    ctrl.del_dev().map_err(ublk_reconcile)?;
    let endpoint_removed = true;
    validate_shutdown_evidence(drained, checkpointed, endpoint_removed)?;
    fs::remove_file(root.join("ready.json")).map_err(|error| {
        AdapterError::ReconciliationRequired(format!(
            "endpoint removed but readiness cleanup failed: {error}"
        ))
    })?;
    drop(opened);
    lifecycle.transition(LifecycleState::Stopped)?;
    Ok(json!({
        "shutdown": "clean",
        "state": lifecycle.state(),
        "fixture_digest": digest,
        "trace_count": trace_count,
        "trace_bound": MAX_TRACE_RECORDS,
        "trace_digest": trace_digest,
    }))
}

pub fn cleanup(root: &Path, device_id: u32) -> Result<serde_json::Value, AdapterError> {
    let fixture = Fixture::load(root)?;
    let expected = fixture.manifest().digest()?;
    let ctrl = libublk::ctrl::UblkCtrl::new_simple(device_id as i32).map_err(ublk_io)?;
    let target = ctrl
        .get_target_data_from_json()
        .ok_or_else(|| AdapterError::Conflict("device has no exported target metadata".into()))?;
    if target.get("contract").and_then(serde_json::Value::as_str) != Some("dwv.ublk.target.v1")
        || target
            .get("fixture_digest")
            .and_then(serde_json::Value::as_str)
            != Some(expected.as_str())
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

fn run_queue(
    qid: u16,
    dev: &UblkDev,
    opened: Arc<Mutex<crate::OpenFixture>>,
    tags: Arc<Mutex<TagTable>>,
    trace: Arc<Mutex<TraceLog>>,
    sequence: Arc<AtomicU64>,
    capacity: u64,
    epoch: TopologyEpoch,
) {
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
            match io_task(&queue, tag, opened, tags, trace, sequence, capacity, epoch).await {
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

async fn io_task(
    queue: &UblkQueue<'_>,
    tag: u16,
    opened: Arc<Mutex<crate::OpenFixture>>,
    tags: Arc<Mutex<TagTable>>,
    trace: Arc<Mutex<TraceLog>>,
    sequence: Arc<AtomicU64>,
    capacity: u64,
    epoch: TopologyEpoch,
) -> Result<(), UblkError> {
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
                match translate_request(
                    raw,
                    capacity,
                    epoch,
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
                            Ok(mut service) => service.execute(translated.normalized, write),
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
        }
        submit?;
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

fn errno(error: &AdapterError) -> i32 {
    terminal_errno(error.terminal())
}

fn owned_devices(digest: &str) -> Vec<u32> {
    let matches = Arc::new(Mutex::new(Vec::new()));
    let found = Arc::clone(&matches);
    let expected = digest.to_owned();
    libublk::ctrl::UblkCtrl::for_each_dev_id(move |id| {
        let Ok(ctrl) = libublk::ctrl::UblkCtrl::new_simple(id as i32) else {
            return;
        };
        if ctrl
            .get_target_data_from_json()
            .and_then(|value| value.get("fixture_digest").cloned())
            .and_then(|value| value.as_str().map(str::to_owned))
            .as_deref()
            == Some(expected.as_str())
        {
            if let Ok(mut ids) = found.lock() {
                ids.push(id);
            }
        }
    });
    Arc::try_unwrap(matches)
        .ok()
        .and_then(|mutex| mutex.into_inner().ok())
        .unwrap_or_default()
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
