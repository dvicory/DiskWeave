use super::*;

impl<S: RandomAccessStore, R: RecoveryStateStore> HealthyPortableService<S, R> {
    /// Accept one physical read child and retain its continuation for later delivery.
    ///
    /// The blocking/file path calls this method and then immediately delivers the
    /// store result. Other adapters may retain the returned identity and call
    /// `complete_read` after this method returns.
    pub fn submit_read(
        &mut self,
        request: BlockRequest,
    ) -> Result<PortableOperationSubmission, ServiceError> {
        self.state.require_reads()?;
        let (member_index, role, _) = validate_request(
            &self.topology,
            &self.members,
            request,
            self.maximum_transfer(),
        )?;
        if request.op != BlockOp::Read || role != MemberRole::Data {
            return Err(ServiceError::invalid(
                FailureClass::InvalidRequest,
                "read endpoint requires a data-member read request",
            ));
        }
        self.ensure_identities()?;
        let plan = split_range(
            request.range,
            self.topology.geometry(),
            self.maximum_transfer(),
        )?;
        let [range] = plan.ranges.as_slice() else {
            return Err(ServiceError::invalid(
                FailureClass::Range,
                "deferred read requires one physical child",
            ));
        };
        let token = self.reserve(request)?;
        let result = (|| {
            let child = self.admission.child(token, *range).map_err(slot_error)?;
            let store = &self.members[member_index].store;
            let identity = self
                .admission
                .accept(
                    token,
                    child,
                    store.store_id(),
                    store.incarnation(),
                    store.topology_epoch(),
                )
                .map_err(slot_error)?;
            Ok(PortableOperationSubmission {
                operation: token,
                child,
                identity,
                request,
            })
        })();
        match result {
            Ok(submission) => Ok(submission),
            Err(error) => {
                Err(self.finish_error(token, request, error, ReleaseRequirement::NotApplicable))
            }
        }
    }

    /// Deliver a previously accepted read and resume normal service completion.
    ///
    /// Returns `None` after the slot owner terminalizes a completion whose
    /// frontend delivery interest was abandoned.
    pub fn complete_read(
        &mut self,
        submission: PortableOperationSubmission,
        payload: PortableReadPayload,
        delivery: StoreCompletionDelivery,
    ) -> Result<Option<(Vec<u8>, OperationEvidence)>, ServiceError> {
        let (request, abandoned) =
            self.read_continuation_request(&submission, &delivery.completion)?;
        let expected_length = usize::try_from(request.range.length)
            .map_err(|_| ServiceError::io(FailureClass::Range, "read range does not fit memory"))?;
        if Some(payload.buffer) != request.buffer
            || payload.bytes.len() != expected_length
            || delivery.identity != submission.identity
        {
            return Err(ServiceError::rejected_completion(
                FailureClass::Admission,
                delivery.completion,
                "read result does not match its accepted physical work",
            )
            .with_request(request));
        }
        let bytes = payload.bytes;
        let reported = delivery.completion.clone();
        self.admission.deliver(delivery).map_err(|error| {
            ServiceError::rejected_completion(
                FailureClass::Admission,
                reported.clone(),
                error.to_string(),
            )
            .with_request(request)
        })?;
        let completion = CompletionEvidence {
            requested: request.range,
            completed: reported.completed.clone(),
            disposition: reported.disposition.clone(),
            persistence: if reported.persistence.is_durable() {
                PersistenceClaim::HostFenceOnly
            } else {
                PersistenceClaim::VolatileOrUnknown
            },
        };
        if matches!(reported.disposition, CompletionDisposition::Success) {
            let generation = self
                .recovery_generation()
                .map_err(|error| error.with_request(request))?;
            let trace = empty_trace(self.topology.topology_epoch(), generation)
                .map_err(|error| error.with_request(request))?;
            let release_authorization = self
                .finish(
                    submission.operation,
                    false,
                    ReleaseRequirement::NotApplicable,
                )
                .map_err(|cleanup| self.cleanup_error(request, cleanup))?;
            let evidence = OperationEvidence {
                request,
                completion,
                trace,
                release_authorization,
            };
            if abandoned {
                Ok(None)
            } else {
                Ok(Some((bytes, evidence)))
            }
        } else {
            let primary = ServiceError::incomplete_read(request, bytes, completion);
            let result = self.finish_error(
                submission.operation,
                request,
                primary,
                ReleaseRequirement::NotApplicable,
            );
            if abandoned {
                match result {
                    ServiceError::IncompleteRead { .. } => Ok(None),
                    error => Err(error),
                }
            } else {
                Err(result)
            }
        }
    }
    fn read_continuation_request(
        &self,
        submission: &PortableOperationSubmission,
        completion: &StoreCompletion,
    ) -> Result<(BlockRequest, bool), ServiceError> {
        let snapshot = self.admission.snapshot(submission.operation).map_err(|_| {
            ServiceError::io(
                FailureClass::Admission,
                "stale or unknown read continuation",
            )
        })?;
        let request = snapshot.request;
        if request != submission.request || submission.child.slot != submission.operation {
            return Err(ServiceError::io(
                FailureClass::Admission,
                "stale or unknown read continuation",
            ));
        }
        let Some(child) = snapshot
            .children
            .iter()
            .find(|child| child.operation_id == submission.child)
        else {
            return Err(ServiceError::io(
                FailureClass::Admission,
                "stale or unknown read continuation",
            ));
        };
        if child.submission != Some(submission.identity) {
            return Err(ServiceError::io(
                FailureClass::Admission,
                "stale or unknown read continuation",
            ));
        }
        if child.terminal && child.completion.as_ref() != Some(completion) {
            return Err(ServiceError::rejected_completion(
                FailureClass::Admission,
                completion.clone(),
                "read completion does not match its retained terminal result",
            )
            .with_request(request));
        }
        Ok((request, snapshot.abandoned))
    }

    /// dwv:req req.healthy-portable-io.healthy-reads-preserve-exact-range-evidence
    pub fn read(
        &mut self,
        request: BlockRequest,
    ) -> Result<(Vec<u8>, OperationEvidence), ServiceError> {
        self.state.require_reads()?;
        let (member_index, role, _) = validate_request(
            &self.topology,
            &self.members,
            request,
            self.maximum_transfer(),
        )?;
        if request.op != BlockOp::Read || role != MemberRole::Data {
            return Err(ServiceError::invalid(
                FailureClass::InvalidRequest,
                "read endpoint requires a data-member read request",
            ));
        }
        self.ensure_identities()?;
        let plan = split_range(
            request.range,
            self.topology.geometry(),
            self.maximum_transfer(),
        )?;
        if plan.ranges.len() == 1 {
            let submission = self.submit_read(request)?;
            let length = usize::try_from(request.range.length).map_err(|_| {
                ServiceError::io(FailureClass::Range, "read range does not fit memory")
            })?;
            let mut bytes = vec![0; length];
            let completion = self.members[member_index].store.read_at(
                submission.child,
                request.range,
                &mut bytes,
            );
            return self
                .complete_read(
                    submission,
                    PortableReadPayload::new(
                        request.buffer.expect("validated read request has a buffer"),
                        bytes,
                    ),
                    StoreCompletionDelivery {
                        identity: submission.identity,
                        completion,
                    },
                )
                .and_then(|completion| {
                    completion.ok_or_else(|| {
                        ServiceError::io(FailureClass::Admission, "read completion was suppressed")
                            .with_request(request)
                    })
                });
        }
        let token = self.reserve(request)?;
        let result = read_member(
            &mut self.members[member_index].store,
            &mut self.admission,
            request,
            token,
            &plan,
        );
        match result {
            Ok((bytes, completion)) => {
                let generation = self
                    .recovery_generation()
                    .map_err(|error| error.with_request(request))?;
                let trace = empty_trace(self.topology.topology_epoch(), generation)
                    .map_err(|error| error.with_request(request))?;
                let release_authorization = self
                    .finish(token, false, ReleaseRequirement::NotApplicable)
                    .map_err(|cleanup| self.cleanup_error(request, cleanup))?;
                Ok((
                    bytes,
                    OperationEvidence {
                        request,
                        completion,
                        trace,
                        release_authorization,
                    },
                ))
            }
            Err(error) => {
                Err(self.finish_error(token, request, error, ReleaseRequirement::NotApplicable))
            }
        }
    }
}
