use super::*;
impl PreparedCodedRefusal {
    pub fn transition(&self) -> CodedCaptureTransition {
        CodedCaptureTransition::updates(vec![CodedCaptureUpdate {
            expected: Some(self.prior.clone()),
            proposed: self.proposed.clone(),
        }])
    }
}

/// Coordinator-issued proof that one exact durable capture is retirable.
///
/// This capability is intentionally not deserializable or reconstructible from
/// a persisted snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CodedCaptureRemoval {
    pub(super) expected: CodedCaptureSnapshot,
    pub(super) clean_closure: Option<CodedCleanClosureEvidence>,
}

impl CodedCaptureRemoval {
    pub(crate) fn expected(&self) -> &CodedCaptureSnapshot {
        &self.expected
    }

    pub(crate) fn validates_clean_closure(&self, recovery: &RecoverySnapshot) -> bool {
        self.clean_closure
            .as_ref()
            .is_none_or(|evidence| evidence.validates(recovery, &self.expected))
    }
}

/// One opaque owner-issued complete coded-capture semantic transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedCaptureTransition {
    pub(super) kind: CodedCaptureTransitionKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum CodedCaptureTransitionKind {
    Updates(Vec<CodedCaptureUpdate>),
    Clean {
        update: CodedCaptureUpdate,
        fence: FenceCertificate,
        dirty_regions: BTreeSet<RegionId>,
        checksum_extents: BTreeSet<IntegrityExtentId>,
    },
    LaterCut {
        update: CodedCaptureUpdate,
        target: InvalidationTarget,
        invalidation_generation: RecoveryGeneration,
    },
    InvalidatingRemoval {
        removal: CodedCaptureRemoval,
        dirty_regions: BTreeSet<RegionId>,
        checksum_extents: BTreeSet<IntegrityExtentId>,
        invalidation_generation: RecoveryGeneration,
        conservative_state: Option<CodedConservativeStateProof>,
    },
    CleanRemoval(CodedCaptureRemoval),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CodedSelectedPostcondition {
    Clean,
    Dirty,
}

impl CodedCaptureTransition {
    pub(super) fn updates(updates: Vec<CodedCaptureUpdate>) -> Self {
        Self {
            kind: CodedCaptureTransitionKind::Updates(updates),
        }
    }

    fn apply_update(
        snapshot: &mut RecoverySnapshot,
        update: CodedCaptureUpdate,
        expected_topology_epoch: TopologyEpoch,
    ) -> Result<(), RecoveryError> {
        let capture_snapshot = update.proposed().clone();
        if capture_snapshot.topology.topology_epoch() != expected_topology_epoch
            || !capture_snapshot.validates_internal_state()
        {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::CodedCaptureTopologyMismatch,
            ));
        }
        let existing_index = snapshot
            .coded_captures
            .iter()
            .position(|existing| existing.capture == capture_snapshot.capture);
        match (existing_index, update.expected()) {
            (Some(index), Some(expected))
                if snapshot.coded_captures.get(index) == Some(expected) =>
            {
                snapshot.coded_captures[index] = capture_snapshot;
            }
            (None, None) if capture_snapshot.capture.0 == snapshot.next_coded_capture_id => {
                snapshot.next_coded_capture_id = snapshot
                    .next_coded_capture_id
                    .checked_add(1)
                    .ok_or(RecoveryError::GenerationExhausted)?;
                snapshot.coded_captures.push(capture_snapshot);
            }
            _ => {
                return Err(RecoveryError::InvalidTransition(
                    TransitionError::CodedCaptureIdentityMismatch,
                ));
            }
        }
        Ok(())
    }

    fn apply_removal(
        snapshot: &mut RecoverySnapshot,
        removal: CodedCaptureRemoval,
    ) -> Result<(), RecoveryError> {
        let expected = removal.expected();
        let index = snapshot
            .coded_captures
            .iter()
            .position(|existing| existing == expected)
            .ok_or(RecoveryError::InvalidTransition(
                TransitionError::CodedCaptureMissing,
            ))?;
        if !removal.validates_clean_closure(snapshot) {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::CodedCaptureClosureMismatch,
            ));
        }
        snapshot.coded_captures.remove(index);
        Ok(())
    }

    fn contains_capture_id(&self, capture: CodedCaptureId) -> bool {
        match &self.kind {
            CodedCaptureTransitionKind::Updates(updates) => updates
                .iter()
                .any(|update| update.proposed.capture == capture),
            CodedCaptureTransitionKind::Clean { update, .. }
            | CodedCaptureTransitionKind::LaterCut { update, .. } => {
                update.proposed.capture == capture
            }
            CodedCaptureTransitionKind::InvalidatingRemoval { removal, .. }
            | CodedCaptureTransitionKind::CleanRemoval(removal) => {
                removal.expected.capture == capture
            }
        }
    }
    pub(crate) fn matches_invalidation(
        &self,
        target: &InvalidationTarget,
        generation: RecoveryGeneration,
    ) -> bool {
        matches!(
            &self.kind,
            CodedCaptureTransitionKind::LaterCut {
                target: selected,
                invalidation_generation,
                ..
            } if selected == target && *invalidation_generation == generation
        )
    }

    fn selected_postcondition(&self) -> Option<CodedSelectedPostcondition> {
        match self.kind {
            CodedCaptureTransitionKind::Updates(_) => None,
            CodedCaptureTransitionKind::Clean { .. }
            | CodedCaptureTransitionKind::CleanRemoval(_) => {
                Some(CodedSelectedPostcondition::Clean)
            }
            CodedCaptureTransitionKind::LaterCut { .. }
            | CodedCaptureTransitionKind::InvalidatingRemoval { .. } => {
                Some(CodedSelectedPostcondition::Dirty)
            }
        }
    }

    fn contains_selected_region(&self, selected: RegionId) -> bool {
        match &self.kind {
            CodedCaptureTransitionKind::Updates(_) => false,
            CodedCaptureTransitionKind::Clean { dirty_regions, .. }
            | CodedCaptureTransitionKind::InvalidatingRemoval { dirty_regions, .. } => {
                dirty_regions.contains(&selected)
            }
            CodedCaptureTransitionKind::LaterCut { target, .. } => {
                target.regions.contains(&selected)
            }
            CodedCaptureTransitionKind::CleanRemoval(removal) => {
                removal.expected.dirty_regions.contains(&selected)
            }
        }
    }

    fn contains_selected_extent(&self, selected: IntegrityExtentId) -> bool {
        match &self.kind {
            CodedCaptureTransitionKind::Updates(_) => false,
            CodedCaptureTransitionKind::Clean {
                checksum_extents, ..
            }
            | CodedCaptureTransitionKind::InvalidatingRemoval {
                checksum_extents, ..
            } => checksum_extents.contains(&selected),
            CodedCaptureTransitionKind::LaterCut { target, .. } => {
                target.checksum_extents.contains(&selected)
            }
            CodedCaptureTransitionKind::CleanRemoval(removal) => {
                removal.expected.checksum_extents.contains(&selected)
            }
        }
    }

    fn selected_scope_overlaps(&self, other: &Self) -> bool {
        match &self.kind {
            CodedCaptureTransitionKind::Updates(_) => false,
            CodedCaptureTransitionKind::Clean {
                dirty_regions,
                checksum_extents,
                ..
            }
            | CodedCaptureTransitionKind::InvalidatingRemoval {
                dirty_regions,
                checksum_extents,
                ..
            } => {
                dirty_regions
                    .iter()
                    .any(|region| other.contains_selected_region(*region))
                    || checksum_extents
                        .iter()
                        .any(|extent| other.contains_selected_extent(*extent))
            }
            CodedCaptureTransitionKind::LaterCut { target, .. } => {
                target
                    .regions
                    .iter()
                    .any(|region| other.contains_selected_region(*region))
                    || target
                        .checksum_extents
                        .iter()
                        .any(|extent| other.contains_selected_extent(*extent))
            }
            CodedCaptureTransitionKind::CleanRemoval(removal) => {
                removal
                    .expected
                    .dirty_regions
                    .iter()
                    .any(|region| other.contains_selected_region(*region))
                    || removal
                        .expected
                        .checksum_extents
                        .iter()
                        .any(|extent| other.contains_selected_extent(*extent))
            }
        }
    }

    pub(crate) fn conflicts_with(&self, other: &Self) -> bool {
        let same_capture = match &self.kind {
            CodedCaptureTransitionKind::Updates(updates) => updates
                .iter()
                .any(|update| other.contains_capture_id(update.proposed.capture)),
            CodedCaptureTransitionKind::Clean { update, .. }
            | CodedCaptureTransitionKind::LaterCut { update, .. } => {
                other.contains_capture_id(update.proposed.capture)
            }
            CodedCaptureTransitionKind::InvalidatingRemoval { removal, .. }
            | CodedCaptureTransitionKind::CleanRemoval(removal) => {
                other.contains_capture_id(removal.expected.capture)
            }
        };
        if same_capture {
            return true;
        }
        matches!(
            (self.selected_postcondition(), other.selected_postcondition()),
            (Some(left), Some(right)) if left != right
        ) && self.selected_scope_overlaps(other)
    }

    pub(crate) fn apply(
        self,
        snapshot: &mut RecoverySnapshot,
        expected_topology_epoch: TopologyEpoch,
    ) -> Result<(), RecoveryError> {
        match self.kind {
            CodedCaptureTransitionKind::Updates(updates) => {
                let identities = updates
                    .iter()
                    .map(|update| update.proposed.capture)
                    .collect::<BTreeSet<_>>();
                if updates.is_empty() || identities.len() != updates.len() {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::CodedSemanticTransitionIncomplete,
                    ));
                }
                for update in updates {
                    Self::apply_update(snapshot, update, expected_topology_epoch)?;
                }
            }
            CodedCaptureTransitionKind::Clean {
                update,
                fence,
                dirty_regions,
                checksum_extents,
            } => {
                let expected = update.expected().ok_or(RecoveryError::InvalidTransition(
                    TransitionError::CodedSemanticTransitionIncomplete,
                ))?;
                if expected.dirty_regions != dirty_regions
                    || expected.checksum_extents != checksum_extents
                    || update.proposed.dirty_regions != dirty_regions
                    || update.proposed.checksum_extents != checksum_extents
                    || update.proposed.phase != CodedCapturePhase::CleanKnown
                    || fence.topology_epoch != expected_topology_epoch
                    || !dirty_regions
                        .iter()
                        .all(|region| fence.covers_region(*region, snapshot.generation))
                    || !checksum_extents
                        .iter()
                        .all(|extent| fence.covers_integrity_extent(*extent, snapshot.generation))
                {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::CodedSemanticTransitionIncomplete,
                    ));
                }
                crate::MemoryRecoveryStore::apply_mutation(
                    snapshot,
                    RecoveryMutation::RecordDataParityFence { fence },
                    expected_topology_epoch,
                )?;
                for region in dirty_regions {
                    crate::MemoryRecoveryStore::apply_mutation(
                        snapshot,
                        RecoveryMutation::MarkRegionDirty {
                            region,
                            mutation_generation: snapshot.generation,
                        },
                        expected_topology_epoch,
                    )?;
                    crate::MemoryRecoveryStore::apply_mutation(
                        snapshot,
                        RecoveryMutation::MarkRegionClean {
                            region,
                            through_generation: snapshot.generation,
                        },
                        expected_topology_epoch,
                    )?;
                }
                for extent in checksum_extents {
                    crate::MemoryRecoveryStore::apply_mutation(
                        snapshot,
                        RecoveryMutation::MarkIntegrityStale {
                            extent,
                            stale_generation: snapshot.generation,
                        },
                        expected_topology_epoch,
                    )?;
                }
                Self::apply_update(snapshot, update, expected_topology_epoch)?;
            }
            CodedCaptureTransitionKind::LaterCut {
                update,
                target,
                invalidation_generation,
            } => {
                if invalidation_generation != snapshot.generation
                    || target.regions.is_empty()
                    || update.expected().is_none()
                {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::CodedSemanticTransitionIncomplete,
                    ));
                }
                for region in target.regions {
                    crate::MemoryRecoveryStore::apply_mutation(
                        snapshot,
                        RecoveryMutation::MarkRegionDirty {
                            region,
                            mutation_generation: invalidation_generation,
                        },
                        expected_topology_epoch,
                    )?;
                }
                for extent in target.checksum_extents {
                    crate::MemoryRecoveryStore::apply_mutation(
                        snapshot,
                        RecoveryMutation::MarkIntegrityStale {
                            extent,
                            stale_generation: invalidation_generation,
                        },
                        expected_topology_epoch,
                    )?;
                }
                Self::apply_update(snapshot, update, expected_topology_epoch)?;
            }
            CodedCaptureTransitionKind::InvalidatingRemoval {
                removal,
                dirty_regions,
                checksum_extents,
                invalidation_generation,
                conservative_state,
            } => {
                let expected = removal.expected();
                if expected.dirty_regions != dirty_regions
                    || expected.checksum_extents != checksum_extents
                    || invalidation_generation != snapshot.generation
                {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::CodedSemanticTransitionIncomplete,
                    ));
                }
                if let Some(proof) = conservative_state {
                    if !proof.validates(snapshot, expected) {
                        return Err(RecoveryError::InvalidTransition(
                            TransitionError::CodedSemanticTransitionIncomplete,
                        ));
                    }
                } else {
                    for region in dirty_regions {
                        crate::MemoryRecoveryStore::apply_mutation(
                            snapshot,
                            RecoveryMutation::MarkRegionDirty {
                                region,
                                mutation_generation: invalidation_generation,
                            },
                            expected_topology_epoch,
                        )?;
                    }
                    for extent in checksum_extents {
                        crate::MemoryRecoveryStore::apply_mutation(
                            snapshot,
                            RecoveryMutation::MarkIntegrityStale {
                                extent,
                                stale_generation: invalidation_generation,
                            },
                            expected_topology_epoch,
                        )?;
                    }
                }
                Self::apply_removal(snapshot, removal)?;
            }
            CodedCaptureTransitionKind::CleanRemoval(removal) => {
                Self::apply_removal(snapshot, removal)?;
            }
        }
        Ok(())
    }

    pub(crate) fn contains_capture(&self, snapshot: &CodedCaptureSnapshot) -> bool {
        match &self.kind {
            CodedCaptureTransitionKind::Updates(updates) => {
                updates.iter().any(|update| update.proposed() == snapshot)
            }
            CodedCaptureTransitionKind::Clean { update, .. }
            | CodedCaptureTransitionKind::LaterCut { update, .. } => update.proposed() == snapshot,
            CodedCaptureTransitionKind::InvalidatingRemoval { .. }
            | CodedCaptureTransitionKind::CleanRemoval(_) => false,
        }
    }

    pub(crate) fn contains_removal(&self, expected: &CodedCaptureRemoval) -> bool {
        match &self.kind {
            CodedCaptureTransitionKind::InvalidatingRemoval { removal, .. }
            | CodedCaptureTransitionKind::CleanRemoval(removal) => removal == expected,
            CodedCaptureTransitionKind::Updates(_)
            | CodedCaptureTransitionKind::Clean { .. }
            | CodedCaptureTransitionKind::LaterCut { .. } => false,
        }
    }

    pub(crate) fn contains_region_dirty(
        &self,
        region: RegionId,
        generation: RecoveryGeneration,
    ) -> bool {
        match &self.kind {
            CodedCaptureTransitionKind::LaterCut {
                target,
                invalidation_generation,
                ..
            } => *invalidation_generation == generation && target.regions.contains(&region),
            CodedCaptureTransitionKind::InvalidatingRemoval {
                dirty_regions,
                invalidation_generation,
                conservative_state,
                ..
            } => {
                conservative_state.is_none()
                    && *invalidation_generation == generation
                    && dirty_regions.contains(&region)
            }
            _ => false,
        }
    }

    pub(crate) fn contains_integrity_stale(
        &self,
        extent: IntegrityExtentId,
        generation: RecoveryGeneration,
    ) -> bool {
        match &self.kind {
            CodedCaptureTransitionKind::Clean {
                fence,
                checksum_extents,
                ..
            } => {
                checksum_extents.contains(&extent)
                    && fence.covers_integrity_extent(extent, generation)
            }
            CodedCaptureTransitionKind::LaterCut {
                target,
                invalidation_generation,
                ..
            } => {
                *invalidation_generation == generation && target.checksum_extents.contains(&extent)
            }
            CodedCaptureTransitionKind::InvalidatingRemoval {
                checksum_extents,
                invalidation_generation,
                conservative_state,
                ..
            } => {
                conservative_state.is_none()
                    && *invalidation_generation == generation
                    && checksum_extents.contains(&extent)
            }
            _ => false,
        }
    }
}

impl PreparedCodedCleanCommit {
    pub fn snapshot(&self) -> &CodedCaptureSnapshot {
        &self.proposed
    }

    pub fn transition(&self) -> CodedCaptureTransition {
        CodedCaptureTransition {
            kind: CodedCaptureTransitionKind::Clean {
                update: CodedCaptureUpdate {
                    expected: Some(self.durable_prior.clone()),
                    proposed: self.proposed.clone(),
                },
                fence: self.fence.clone(),
                dirty_regions: self.proposed.dirty_regions.clone(),
                checksum_extents: self.proposed.checksum_extents.clone(),
            },
        }
    }
}
impl PreparedCodedLaterCut {
    pub fn snapshot(&self) -> &CodedCaptureSnapshot {
        &self.proposed
    }

    pub fn invalidation_target(&self) -> &InvalidationTarget {
        &self.target
    }

    pub fn transition(&self) -> CodedCaptureTransition {
        CodedCaptureTransition {
            kind: CodedCaptureTransitionKind::LaterCut {
                update: CodedCaptureUpdate {
                    expected: Some(self.prior.clone()),
                    proposed: self.proposed.clone(),
                },
                target: self.target.clone(),
                invalidation_generation: self.expected_generation,
            },
        }
    }
}
impl PreparedCodedMembershipCompaction {
    pub fn transition(&self) -> CodedCaptureTransition {
        CodedCaptureTransition::updates(vec![CodedCaptureUpdate {
            expected: Some(self.prior.clone()),
            proposed: self.proposed.clone(),
        }])
    }
}
impl PreparedCodedRefusedCleanup {
    pub fn transition(&self) -> CodedCaptureTransition {
        self.0.invalidating_transition()
    }
}

impl PreparedCodedCleanKnownCleanup {
    pub fn transition(&self) -> CodedCaptureTransition {
        CodedCaptureTransition {
            kind: CodedCaptureTransitionKind::CleanRemoval(self.0.removal()),
        }
    }
}

impl PreparedCodedInheritedCaptureAbandonment {
    pub fn transition(&self) -> CodedCaptureTransition {
        self.0.invalidating_transition()
    }
}

impl PreparedCodedCaptureRemoval {
    pub(super) fn removal(&self) -> CodedCaptureRemoval {
        CodedCaptureRemoval {
            expected: self.prior.clone(),
            clean_closure: self.clean_closure.clone(),
        }
    }

    fn invalidating_transition(&self) -> CodedCaptureTransition {
        CodedCaptureTransition {
            kind: CodedCaptureTransitionKind::InvalidatingRemoval {
                removal: self.removal(),
                dirty_regions: self.prior.dirty_regions.clone(),
                checksum_extents: self.prior.checksum_extents.clone(),
                invalidation_generation: self.expected_generation,
                conservative_state: self.conservative_state.clone(),
            },
        }
    }
}
impl PreparedCodedCaptureUpdates {
    pub fn transition(&self) -> CodedCaptureTransition {
        CodedCaptureTransition::updates(self.updates.clone())
    }
}
