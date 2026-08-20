use dwv_core::{ByteRange, FenceDomain, TopologyEpoch};
use dwv_recovery::{IntegrityExtentId, RecoveryGeneration, RegionId};
use dwv_store::StoreId;
use dwv_transaction_proc::{compare_schedule, measure_schedule, semantic_actions};
use dwv_transaction_ref::{
    ActionResult, CommittedRecoveryGeneration, ComputationResult, ParityComputationPlan,
    ParityRange, PlannedRead, PlannedWrite, RangeGuardToken, SemanticIoResult, StoreWatermark,
    TransactionAction, TransactionPersistenceEvidence, TransactionPlan,
};

fn plan() -> TransactionPlan {
    TransactionPlan::new(TopologyEpoch(1), RecoveryGeneration(2), FenceDomain(3))
        .with_ranges(vec![ParityRange::new(
            RegionId(1),
            StoreId(1),
            ByteRange::new(0, 512).unwrap(),
        )])
        .with_dirty_regions(vec![RegionId(1)])
        .with_checksum_extents(vec![IntegrityExtentId(1)])
        .with_reads(vec![PlannedRead::new(
            StoreId(1),
            ByteRange::new(0, 512).unwrap(),
        )])
        .with_parity(ParityComputationPlan::new(
            TopologyEpoch(1),
            ByteRange::new(0, 512).unwrap(),
            1,
        ))
        .with_writes(vec![PlannedWrite::new(
            StoreId(1),
            ByteRange::new(0, 512).unwrap(),
        )])
        .with_stores(vec![StoreId(1)])
        .with_watermarks(vec![StoreWatermark::new(
            StoreId(1),
            dwv_store::StoreWriteWatermark(1),
        )])
}

fn results(plan: &TransactionPlan) -> Vec<ActionResult> {
    let certificate = semantic_actions(plan)
        .into_iter()
        .find_map(|action| match action {
            TransactionAction::CommitRecoveryClean { certificate } => Some(certificate),
            _ => None,
        })
        .unwrap();
    let generation = CommittedRecoveryGeneration::new(RecoveryGeneration(2), TopologyEpoch(1));
    vec![
        ActionResult::RangeAcquired(RangeGuardToken::new(1)),
        ActionResult::WriteRecoveryRecordDurable(generation),
        ActionResult::ReadSetComplete(SemanticIoResult::Complete),
        ActionResult::ParityComputed(ComputationResult::Complete),
        ActionResult::WriteSetComplete(SemanticIoResult::Complete),
        ActionResult::FlushSetComplete(TransactionPersistenceEvidence::durable(certificate)),
        ActionResult::RecoveryCleanCommitted(generation),
        ActionResult::RangeReleased,
    ]
}

fn main() {
    let repetitions = std::env::args()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(1_000);
    let plan = plan();
    let results = results(&plan);
    let report = compare_schedule(plan.clone(), &results);
    assert!(
        report.equivalent,
        "comparison failed: {:?}",
        report.mismatch
    );
    let semantic_steps = report.reference_events.len();
    assert_eq!(
        report.candidate.as_ref().unwrap().events.len(),
        semantic_steps
    );
    let measurement = measure_schedule(plan, &results, repetitions).unwrap();
    let candidate = measurement.median_candidate_elapsed_nanos as f64;
    let reference = measurement.median_reference_elapsed_nanos as f64;
    println!(
        "equivalent=true semantic_steps={} repetitions={} candidate_median_ns={} reference_median_ns={} candidate/reference={:.3} sync_ops={} candidate_io_bytes={} reference_machine_bytes={}",
        semantic_steps,
        measurement.repetitions,
        measurement.median_candidate_elapsed_nanos,
        measurement.median_reference_elapsed_nanos,
        candidate / reference.max(1.0),
        measurement.synchronization_operations,
        measurement.candidate_io_size_bytes,
        measurement.reference_machine_size_bytes,
    );
}
