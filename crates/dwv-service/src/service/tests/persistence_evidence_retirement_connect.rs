use super::*;
use anyhow::{Context, bail};
use dwv_recovery::{
    FenceCertificate, FenceOccurrenceId, RecoveryCleanDecision, RecoveryCleanRequest,
    RecoveryMutation, RecoveryRetirementPlan, RecoveryRootFact, RecoverySnapshot,
    RecoveryStateStore, RecoveryTxn, RegionState, evaluate_recovery_clean,
};
use quint_connect::{Config, Driver, Result, State, Step, quint_run};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(tag = "tag", content = "value")]
enum ModelRootKind {
    CleanRegion,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(tag = "tag", content = "value")]
enum ModelOwnerFact {
    Clean(ModelCleanFact),
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
struct ModelCleanFact {
    region: i64,
    #[serde(rename = "cleanGeneration")]
    clean_generation: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
struct ModelStoreFact {
    store: i64,
    incarnation: i64,
    capability: i64,
    watermark: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
struct ModelFenceShape {
    topology: i64,
    domain: i64,
    stores: BTreeSet<ModelStoreFact>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
struct ModelCertificateBinding {
    shape: ModelFenceShape,
    target: i64,
    coverage: BTreeSet<i64>,
    generation: i64,
    watermark: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
struct ModelClaimBinding {
    kind: ModelRootKind,
    fact: ModelOwnerFact,
    certificate: ModelCertificateBinding,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
struct ModelFence {
    occurrence: i64,
    shape: ModelFenceShape,
    claims: BTreeSet<ModelCertificateBinding>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
struct ModelRoot {
    id: i64,
    occurrence: i64,
    binding: ModelClaimBinding,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct ModelSnapshot {
    generation: i64,
    topology: i64,
    #[serde(rename = "allocatedOccurrences")]
    allocated_occurrences: BTreeSet<i64>,
    #[serde(rename = "allocatedRoots")]
    allocated_roots: BTreeSet<i64>,
    fences: BTreeSet<ModelFence>,
    roots: BTreeSet<ModelRoot>,
}

fn model_shape(certificate: &FenceCertificate) -> ModelFenceShape {
    ModelFenceShape {
        topology: certificate.topology_epoch.0 as i64,
        domain: certificate.fence_domain.0 as i64,
        stores: certificate
            .stores
            .iter()
            .map(|store| ModelStoreFact {
                store: store.store_id.0 as i64,
                incarnation: store.store_incarnation.0 as i64,
                capability: store.capability_evidence_id.0 as i64,
                watermark: store.through.0 as i64,
            })
            .collect(),
    }
}

fn model_certificate(
    certificate: &FenceCertificate,
    fact: &RecoveryRootFact,
) -> Result<ModelCertificateBinding> {
    let RecoveryRootFact::CleanRegion {
        region,
        clean_generation,
    } = fact
    else {
        bail!("retention Connect profile received an unsupported root fact");
    };
    Ok(ModelCertificateBinding {
        shape: model_shape(certificate),
        target: region.0 as i64,
        coverage: BTreeSet::from([region.0 as i64]),
        generation: clean_generation.0 as i64,
        watermark: certificate
            .stores
            .iter()
            .map(|store| store.through.0 as i64)
            .max()
            .unwrap_or_default(),
    })
}

fn model_root(root: &dwv_recovery::RecoveryClaimRoot) -> Result<ModelRoot> {
    let RecoveryRootFact::CleanRegion {
        region,
        clean_generation,
    } = root.fact
    else {
        bail!("retention Connect profile received an unsupported root fact");
    };
    Ok(ModelRoot {
        id: root.id.0 as i64,
        occurrence: root.occurrence.0 as i64,
        binding: ModelClaimBinding {
            kind: ModelRootKind::CleanRegion,
            fact: ModelOwnerFact::Clean(ModelCleanFact {
                region: region.0 as i64,
                clean_generation: clean_generation.0 as i64,
            }),
            certificate: model_certificate(&root.certificate, &root.fact)?,
        },
    })
}

fn model_snapshot(snapshot: &RecoverySnapshot) -> Result<ModelSnapshot> {
    let roots = snapshot
        .roots
        .iter()
        .map(model_root)
        .collect::<Result<BTreeSet<_>>>()?;
    let fences = snapshot
        .fences
        .iter()
        .map(|fence| {
            let claims = snapshot
                .roots
                .iter()
                .filter(|root| root.occurrence == fence.occurrence)
                .map(|root| model_certificate(&root.certificate, &root.fact))
                .collect::<Result<BTreeSet<_>>>()?;
            let capture_claims = fence
                .captured_region_generations
                .iter()
                .map(|(region, generation)| ModelCertificateBinding {
                    shape: model_shape(fence),
                    target: region.0 as i64,
                    coverage: BTreeSet::from([region.0 as i64]),
                    generation: generation.0 as i64,
                    watermark: fence
                        .stores
                        .iter()
                        .map(|store| store.through.0 as i64)
                        .max()
                        .unwrap_or_default(),
                })
                .collect::<BTreeSet<_>>();
            Ok(ModelFence {
                occurrence: fence.occurrence.0 as i64,
                shape: model_shape(fence),
                claims: claims.union(&capture_claims).cloned().collect(),
            })
        })
        .collect::<Result<BTreeSet<_>>>()?;
    Ok(ModelSnapshot {
        generation: snapshot.generation.0 as i64,
        topology: snapshot.topology_epoch.0 as i64,
        allocated_occurrences: (1..snapshot.next_fence_occurrence_id.0)
            .map(|id| id as i64)
            .collect(),
        allocated_roots: (1..snapshot.next_root_id.0).map(|id| id as i64).collect(),
        fences,
        roots,
    })
}

fn clean_record_is_exact(snapshot: &RecoverySnapshot) -> bool {
    let Some(record) = snapshot
        .dirty_regions
        .iter()
        .find(|record| record.region == RegionId(1))
    else {
        return false;
    };
    let Some(fence) = snapshot.fence(FenceOccurrenceId::FIRST) else {
        return false;
    };
    record.state == RegionState::Clean
        && record.clean_generation == Some(RecoveryGeneration(1))
        && record.last_clean_fence.as_ref() == Some(fence)
}

#[derive(Default)]
struct BridgeDriver {
    recovery: Option<MemoryRecoveryStore>,
    // Process-local recovery validation only; Connect state projects durable snapshots.
    pending_plan: Option<RecoveryRetirementPlan>,
}

impl State<BridgeDriver> for ModelSnapshot {
    fn from_driver(driver: &BridgeDriver) -> Result<Self> {
        let recovery = driver
            .recovery
            .as_ref()
            .context("retention Connect driver has no recovery owner")?;
        model_snapshot(recovery.snapshot())
    }
}

impl Driver for BridgeDriver {
    type State = ModelSnapshot;

    fn config() -> Config {
        Config {
            state: &[
                "PersistenceEvidenceRetirementConnect::retention::state",
                "snapshot",
            ],
            ..Config::default()
        }
    }
    fn step(&mut self, step: &Step) -> Result {
        quint_connect::switch!(step {
            connectInit => self.init(),
            appendFirstFence => self.append_first_fence()?,
            admitCleanRoot => self.admit_clean_root()?,
            appendSecondFence => self.append_second_fence()?,
            prepareSecondRetirement => self.prepare_second_retirement()?,
            commitRetirement => self.commit_retirement()?,
        })
    }
}

impl BridgeDriver {
    fn init(&mut self) {
        self.recovery = Some(MemoryRecoveryStore::new(TopologyEpoch(1)));
        self.pending_plan = None;
    }

    fn recovery(&self) -> Result<&MemoryRecoveryStore> {
        self.recovery
            .as_ref()
            .context("retention Connect driver has no recovery owner")
    }

    fn recovery_mut(&mut self) -> Result<&mut MemoryRecoveryStore> {
        self.recovery
            .as_mut()
            .context("retention Connect driver has no mutable recovery owner")
    }

    fn append_first_fence(&mut self) -> Result {
        let current = self.recovery()?.snapshot().clone();
        if current.generation != RecoveryGeneration::ZERO
            || !current.fences.is_empty()
            || !current.roots.is_empty()
        {
            bail!("appendFirstFence was selected outside the empty production snapshot");
        }
        let fence = FenceCertificate::new(
            current.topology_epoch,
            FenceDomain(1),
            vec![StoreFenceRef {
                fence_id: FenceId(1),
                store_id: StoreId(1),
                store_incarnation: StoreIncarnationId(1),
                topology_epoch: current.topology_epoch,
                through: StoreWriteWatermark(0),
                capability_evidence_id: CapabilityEvidenceId(1),
            }],
            vec![(RegionId(1), RecoveryGeneration(1))],
        );
        let mut transaction = RecoveryTxn::new(current.generation, current.topology_epoch);
        transaction.push(RecoveryMutation::RecordDataParityFence { fence });
        let receipt = self.recovery_mut()?.commit_durable_receipt(transaction)?;
        let snapshot = self.recovery()?.snapshot();
        if receipt.generation() != RecoveryGeneration(1)
            || snapshot.fences.len() != 1
            || snapshot.fence(FenceOccurrenceId::FIRST).is_none()
            || !snapshot.dirty_regions.is_empty()
        {
            bail!("production fence commit did not publish occurrence one");
        }
        Ok(())
    }

    fn admit_clean_root(&mut self) -> Result {
        let current = self.recovery()?.snapshot().clone();
        let fence = current
            .fence(FenceOccurrenceId::FIRST)
            .context("clean root has no first fence")?
            .clone();
        if current.generation != RecoveryGeneration(1)
            || !current.roots.is_empty()
            || !current.dirty_regions.is_empty()
        {
            bail!("admitCleanRoot was selected outside the mapped CLEAN path");
        }
        let request =
            RecoveryCleanRequest::new(current.topology_epoch, FenceDomain(1), current.generation)
                .with_region(RegionId(1));
        let permit = match evaluate_recovery_clean(&current, &request, &fence)? {
            RecoveryCleanDecision::Clear(permit) => permit,
            RecoveryCleanDecision::Refused { .. } => {
                bail!("production CLEAN owner refused complete evidence")
            }
        };
        if permit.generation() != current.generation
            || permit.topology_epoch() != current.topology_epoch
        {
            bail!("CLEAN permit does not match the loaded predecessor");
        }
        let mut transaction = RecoveryTxn::new(current.generation, current.topology_epoch);
        transaction
            .push(RecoveryMutation::MarkRegionDirty {
                region: RegionId(1),
                mutation_generation: RecoveryGeneration(1),
            })
            .push(RecoveryMutation::MarkRegionClean {
                region: RegionId(1),
                through_generation: RecoveryGeneration(1),
                fence_occurrence: FenceOccurrenceId::FIRST,
            });
        let receipt = self.recovery_mut()?.commit_durable_receipt(transaction)?;
        let snapshot = self.recovery()?.snapshot();
        let root = snapshot
            .roots
            .first()
            .context("CLEAN owner did not publish a root")?;
        let root_matches_fence = snapshot
            .fence(FenceOccurrenceId::FIRST)
            .is_some_and(|fence| fence == &root.certificate);
        if receipt.generation() != RecoveryGeneration(2)
            || snapshot.roots.len() != 1
            || !clean_record_is_exact(snapshot)
            || !root_matches_fence
            || root.occurrence != FenceOccurrenceId::FIRST
            || !matches!(
                root.fact,
                RecoveryRootFact::CleanRegion {
                    region: RegionId(1),
                    clean_generation: RecoveryGeneration(1),
                }
            )
        {
            bail!("production CLEAN owner published an incorrect root");
        }
        Ok(())
    }

    fn append_second_fence(&mut self) -> Result {
        let current = self.recovery()?.snapshot().clone();
        if current.generation != RecoveryGeneration(2)
            || current.fences.len() != 1
            || current.roots.len() != 1
        {
            bail!("appendSecondFence was selected outside the root-preserving path");
        }
        let fence = FenceCertificate::new(
            current.topology_epoch,
            FenceDomain(1),
            vec![StoreFenceRef {
                fence_id: FenceId(2),
                store_id: StoreId(1),
                store_incarnation: StoreIncarnationId(1),
                topology_epoch: current.topology_epoch,
                through: StoreWriteWatermark(0),
                capability_evidence_id: CapabilityEvidenceId(1),
            }],
            Vec::new(),
        );
        let mut transaction = RecoveryTxn::new(current.generation, current.topology_epoch);
        transaction.push(RecoveryMutation::RecordDataParityFence { fence });
        let receipt = self.recovery_mut()?.commit_durable_receipt(transaction)?;
        let snapshot = self.recovery()?.snapshot();
        if receipt.generation() != RecoveryGeneration(3)
            || snapshot.fences.len() != 2
            || snapshot.fence(FenceOccurrenceId(2)).is_none()
            || snapshot.roots.len() != 1
        {
            bail!("second fence commit did not preserve the CLEAN root");
        }
        Ok(())
    }

    fn prepare_second_retirement(&mut self) -> Result {
        let current = self.recovery()?.snapshot().clone();
        if current.generation != RecoveryGeneration(3)
            || current.fences.len() != 2
            || current.roots.len() != 1
            || self.pending_plan.is_some()
        {
            bail!("prepareSecondRetirement was selected outside the mapped path");
        }
        let mut successor = current.clone();
        successor
            .fences
            .retain(|fence| fence.occurrence != FenceOccurrenceId(2));
        let mut plan = RecoveryRetirementPlan::new(current, successor);
        plan.retire(FenceOccurrenceId(2));
        self.pending_plan = Some(plan);
        Ok(())
    }

    fn commit_retirement(&mut self) -> Result {
        let plan = self
            .pending_plan
            .as_ref()
            .context("commitRetirement has no prepared plan")?
            .clone();
        let current = self.recovery()?.snapshot().clone();
        if current != plan.predecessor {
            bail!("production snapshot changed during the mapped pending interval");
        }
        let mut transaction = RecoveryTxn::new(current.generation, current.topology_epoch);
        transaction.push(RecoveryMutation::RetireFences { plan });
        let receipt = self.recovery_mut()?.commit_durable_receipt(transaction)?;
        let snapshot = self.recovery()?.snapshot();
        let root = snapshot
            .roots
            .first()
            .context("production retirement removed the CLEAN root")?;
        let root_matches_fence = snapshot
            .fence(FenceOccurrenceId::FIRST)
            .is_some_and(|fence| fence == &root.certificate);
        if receipt.generation() != RecoveryGeneration(4)
            || snapshot.fences.len() != 1
            || snapshot.fence(FenceOccurrenceId::FIRST).is_none()
            || snapshot.fence(FenceOccurrenceId(2)).is_some()
            || snapshot.roots.len() != 1
            || !clean_record_is_exact(snapshot)
            || !root_matches_fence
            || root.occurrence != FenceOccurrenceId::FIRST
            || root.topology_epoch != snapshot.topology_epoch
            || root.generation != RecoveryGeneration(2)
        {
            bail!("production retirement did not preserve the root-bound predecessor");
        }
        self.pending_plan = None;
        Ok(())
    }
}

#[quint_run(
    spec = "../../verification/quint/PersistenceEvidenceRetirementConnect.qnt",
    main = "PersistenceEvidenceRetirementConnect",
    init = "connectInit",
    step = "connectStep",
    max_samples = 1,
    max_steps = 8,
    seed = "22082026"
)]
fn persistence_evidence_retirement_connect() -> impl Driver {
    BridgeDriver::default()
}

#[quint_run(
    spec = "../../verification/quint/PersistenceEvidenceRetirementConnect.qnt",
    main = "PersistenceEvidenceRetirementConnect",
    init = "connectInit",
    step = "connectStep",
    max_samples = 1,
    max_steps = 8,
    seed = "1"
)]
fn persistence_evidence_retirement_connect_second_seed() -> impl Driver {
    BridgeDriver::default()
}
