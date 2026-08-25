//! Run issuance and prefetch capability; this module owns its route behavior.

use super::contracts::{IssuedAttemptMetadata, RunBackend, RunBackendError};
use super::queries::{all_attempts, owned_assignment_for_run, owned_run};
use super::support::*;
use learning_data_access::{
    AssetStore, FlatGradingCapability, IssuedNativeAssetBindingV1, IssuedQuestionFamilyWitnessV1,
    IssuedQuestionSnapshotV1, NativeExecutionEnvelopeCapability, PrefetchedPrivateExecutionV1,
    PrefetchedQuestionDescriptorV1, QtiGradingCapability,
};
use question_model::{
    ResponseDefinition,
    envelope::{AssetRef, ContentBlock},
};

fn referenced_assets(envelope: &QuestionEnvelope) -> Result<Vec<AssetRef>, RunBackendError> {
    fn add_reference(
        references: &mut Vec<AssetRef>,
        reference: &AssetRef,
    ) -> Result<(), RunBackendError> {
        if let Some(existing) = references
            .iter()
            .find(|existing| existing.asset == reference.asset)
        {
            if existing.checksum != reference.checksum {
                return Err(RunBackendError::Invalid(
                    "one question references an asset with conflicting checksums".to_string(),
                ));
            }
        } else {
            references.push(reference.clone());
        }
        Ok(())
    }
    fn add_content(
        references: &mut Vec<AssetRef>,
        content: &[ContentBlock],
    ) -> Result<(), RunBackendError> {
        for block in content {
            if let ContentBlock::Image { asset, .. } = block {
                add_reference(references, asset)?;
            }
        }
        Ok(())
    }
    let mut references = Vec::new();
    add_content(&mut references, &envelope.prompt)?;
    match &envelope.response {
        ResponseDefinition::MultipleChoice { choices, .. } => {
            for choice in choices {
                add_content(&mut references, &choice.body)?;
            }
        }
        ResponseDefinition::MultiBlank { blanks } => {
            for blank in blanks {
                add_content(&mut references, &blank.label)?;
            }
        }
        ResponseDefinition::Matching { prompts, choices } => {
            for choice in prompts.iter().chain(choices) {
                add_content(&mut references, &choice.body)?;
            }
        }
        ResponseDefinition::Ordering { items } => {
            for item in items {
                add_content(&mut references, &item.body)?;
            }
        }
        ResponseDefinition::Hotspot {
            surface, regions, ..
        } => {
            add_reference(&mut references, surface)?;
            for region in regions {
                add_content(&mut references, &region.label)?;
            }
        }
        ResponseDefinition::Numeric { .. }
        | ResponseDefinition::ShortText { .. }
        | ResponseDefinition::FileUpload { .. }
        | ResponseDefinition::ExternalTool {} => {}
    }
    references.sort_by_key(|reference| reference.asset);
    Ok(references)
}

async fn fresh_presentation<S: AssetStore>(
    store: &S,
    context: TenantContext,
    reference: ProblemVersionRef,
    envelope: &QuestionEnvelope,
) -> Result<Option<PresentationV1>, RunBackendError> {
    if matches!(
        envelope.response,
        ResponseDefinition::FileUpload { .. } | ResponseDefinition::ExternalTool {}
    ) {
        return Ok(None);
    }
    let registered = store
        .catalog_asset_bindings(context, reference)
        .await
        .map_err(|error| {
            RunBackendError::Unavailable(format!(
                "published asset registry is unavailable: {error}"
            ))
        })?;
    let bindings = referenced_assets(envelope)?
        .into_iter()
        .map(|authored| {
            let registered = registered
                .iter()
                .find(|registered| registered.asset == authored.asset)
                .ok_or_else(|| {
                    RunBackendError::Invalid(
                        "published question references an unavailable immutable asset".to_string(),
                    )
                })?;
            if registered.rendition_checksum.to_string() != authored.checksum {
                return Err(RunBackendError::Invalid(
                    "published asset checksum does not match the authored reference".to_string(),
                ));
            }
            Ok(AssetBindingV1 {
                asset: authored.asset,
                authored_checksum: authored.checksum,
                rendition_checksum: registered.rendition_checksum.to_string(),
                intrinsic_width: registered.intrinsic_width,
                intrinsic_height: registered.intrinsic_height,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    build_presentation_v1(envelope, &bindings)
        .map(Some)
        .map_err(|error| {
            RunBackendError::Invalid(format!("question presentation is invalid: {error}"))
        })
}

async fn issued_native_physical_bindings<S: AssetStore>(
    store: &S,
    context: TenantContext,
    reference: ProblemVersionRef,
    envelope: &QuestionEnvelope,
) -> Result<Vec<IssuedNativeAssetBindingV1>, RunBackendError> {
    let registered = store
        .catalog_asset_bindings(context, reference)
        .await
        .map_err(|_| {
            RunBackendError::Unavailable("published asset registry is unavailable".into())
        })?;
    referenced_assets(envelope)?
        .into_iter()
        .map(|authored| {
            let registered = registered
                .iter()
                .find(|value| value.asset == authored.asset)
                .ok_or_else(|| {
                    RunBackendError::Invalid(
                        "published question references an unavailable immutable asset".into(),
                    )
                })?;
            if registered.rendition_checksum.to_string() != authored.checksum {
                return Err(RunBackendError::Invalid(
                    "published asset checksum does not match the authored reference".into(),
                ));
            }
            Ok(IssuedNativeAssetBindingV1 {
                asset: authored.asset,
                object: registered.object,
                authored_checksum: authored.checksum,
                rendition_checksum: registered.rendition_checksum.to_string(),
                intrinsic_width: registered.intrinsic_width,
                intrinsic_height: registered.intrinsic_height,
            })
        })
        .collect()
}

fn receipt_presentation(presentation: PresentationV1) -> ReceiptPresentationSnapshot {
    ReceiptPresentationSnapshot {
        envelope: presentation.envelope,
        asset_bindings: presentation.asset_bindings,
    }
}

/// Declares the immutable server-only envelope obligation for a native family
/// that is not graded by the flat contract.  Browser presentation capability
/// deliberately does not stand in for this execution authority.
fn native_execution_envelope_capability(
    question: &QuestionDefinition,
    flat_grading: FlatGradingCapability,
) -> NativeExecutionEnvelopeCapability {
    if matches!(
        question.source,
        question_model::QuestionSource::Native { .. }
    ) && matches!(flat_grading, FlatGradingCapability::NotApplicable)
    {
        NativeExecutionEnvelopeCapability::Required
    } else {
        NativeExecutionEnvelopeCapability::NotApplicable
    }
}

/// Constructs the closed issuance witness exactly once, after all private
/// issuance contracts and answer-free presentation authority are available.
/// ASVS 2.2.3: source family and capability combinations are positive
/// validated at this trusted boundary before durable reservation or issue.
fn issued_question_snapshot(
    question: &QuestionDefinition,
    flat: FlatGradingCapability,
    webwork: learning_data_access::WebworkGradingCapability,
    qti: QtiGradingCapability,
    presentation: Option<&ReceiptPresentationSnapshot>,
    native_physical_asset_bindings: Vec<IssuedNativeAssetBindingV1>,
) -> Result<IssuedQuestionSnapshotV1, RunBackendError> {
    let witness = match &question.source {
        question_model::QuestionSource::Native { .. }
            if matches!(flat, FlatGradingCapability::Required) =>
        {
            IssuedQuestionFamilyWitnessV1::Flat {}
        }
        question_model::QuestionSource::Native { .. } => IssuedQuestionFamilyWitnessV1::Native {
            // Presentation snapshots own selected physical bindings. The
            // remaining envelope-less native families contain no renderable
            // physical binding authority at this seam.
            physical_asset_bindings: native_physical_asset_bindings,
        },
        question_model::QuestionSource::Webwork { .. } => IssuedQuestionFamilyWitnessV1::Webwork {},
        question_model::QuestionSource::Qti {
            package_object,
            package_sha256,
            ..
        } => IssuedQuestionFamilyWitnessV1::Qti {
            source_artifact: question_model::SourceArtifact {
                object: *package_object,
                sha256: package_sha256.clone(),
            },
        },
        question_model::QuestionSource::Imathas {
            snapshot,
            snapshot_sha256,
            integration_profile,
            ..
        } => IssuedQuestionFamilyWitnessV1::External {
            source_artifact: question_model::SourceArtifact {
                object: *snapshot,
                sha256: snapshot_sha256.clone(),
            },
            integration_profile_identity: integration_profile.clone(),
        },
        question_model::QuestionSource::H5p { .. } => {
            return Err(RunBackendError::Unsupported(
                "H5P cannot be issued by the deterministic run backend".into(),
            ));
        }
    };
    IssuedQuestionSnapshotV1::new(question.clone(), witness)
        .and_then(|snapshot| {
            snapshot.validate_for_issuance_context(flat, webwork, qti, presentation)?;
            Ok(snapshot)
        })
        .map_err(|_| RunBackendError::Invalid("issued question snapshot is invalid".into()))
}

fn bind_webwork_replay(
    question: &QuestionDefinition,
    issued: &IssuedAttemptMetadata,
    presentation: Option<&PresentationV1>,
) -> Result<Option<learning_data_access::WebworkReplayMappingV1>, RunBackendError> {
    match (&question.source, issued.webwork_replay.clone()) {
        (question_model::QuestionSource::Webwork { .. }, Some(replay)) => presentation
            .ok_or_else(|| {
                RunBackendError::Invalid("WeBWorK issuance lacks a presentation binding".into())
            })
            .and_then(|presentation| {
                crate::webwork_backend::persist_replay_mapping(replay, presentation).map(Some)
            }),
        (question_model::QuestionSource::Webwork { .. }, None) => Err(RunBackendError::Invalid(
            "WeBWorK issuance omitted private replay state".into(),
        )),
        (_, Some(_)) => Err(RunBackendError::Invalid(
            "non-WeBWorK issuance returned private replay state".into(),
        )),
        (_, None) => Ok(None),
    }
}

/// Prepares the next still-unattempted assignment position while the current
/// question remains the sole active attempt. This is intentionally POST: a
/// successful request creates a durable server reservation, but no timer or
/// activity transition.
pub(super) async fn prefetch_next_question<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path((course, assignment, predecessor)): Path<(CourseId, AssignmentId, QuestionAttemptId)>,
    body: axum::body::Body,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    // ASVS 2.2.1 and 8.3.1: parse the complete route shape into closed IDs,
    // then verify it at the trusted service layer before any prefetch write.
    let binding = LearnerWorkRoutingBinding::new(course, assignment);
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    // This mutation has no browser-controlled parameters. Consume the body so
    // chunked requests cannot smuggle a seed, position, or provenance past a
    // mere Content-Length check.
    let bytes = match to_bytes(body, 1).await {
        Ok(value) => value,
        Err(_) => {
            return error_response(StatusCode::BAD_REQUEST, "prefetch request body is invalid");
        }
    };
    if !bytes.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "prefetch request must not contain a body",
        );
    }
    let active = match state
        .store
        .learner_get_question_attempt(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            predecessor,
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "attempt not found"),
        Err(error) => return store_error_response(error),
    };
    let run = match owned_run(state.store.as_ref(), &authenticated, active.run).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if active.response.is_some() || run.completed_at.is_some() {
        return error_response(StatusCode::CONFLICT, "attempt is no longer active");
    }
    if let Err(response) =
        require_run_binding(state.store.as_ref(), &authenticated, binding, &run).await
    {
        return response;
    }
    let run_items = match state
        .store
        .learner_assignment_run_items(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            run.id,
        )
        .await
    {
        Ok(Some(items)) => items,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "run not found"),
        Err(error) => return store_error_response(error),
    };
    let attempts = match all_attempts(state.store.as_ref(), &authenticated, run.id).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if attempts
        .iter()
        .any(|attempt| attempt.response.is_none() && attempt.id != predecessor)
    {
        return error_response(StatusCode::CONFLICT, "another question attempt is active");
    }
    let Some((assignment_position, reference)) = run_items.iter().find_map(|item| {
        let position = item.issued_position;
        attempts
            .iter()
            .all(|attempt| attempt.assignment_position != position)
            .then_some((position, item.reference))
    }) else {
        return no_store(StatusCode::NO_CONTENT.into_response());
    };
    let question = match load_run_question(state.store.as_ref(), &authenticated, reference).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let actor = authenticated.record.subject.user();
    let existing = match state
        .store
        .learner_get_prefetched_question(
            authenticated.tenant_context,
            actor,
            run.id,
            predecessor,
            assignment_position,
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let (reservation, issued) = match existing {
        Some(value) => (value, None),
        None => {
            let seed = match fresh_seed() {
                Ok(value) => value,
                Err(error) => return backend_error_response(error),
            };
            let issued = match state
                .backend
                .issue(authenticated.tenant_context, reference, &question, seed)
                .await
            {
                Ok(value) => value,
                Err(error) => return backend_error_response(error),
            };
            let presentation = match fresh_presentation(
                state.store.as_ref(),
                authenticated.tenant_context,
                reference,
                &issued.envelope,
            )
            .await
            {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return error_response(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "this response family cannot be prefetched",
                    );
                }
                Err(error) => return backend_error_response(error),
            };
            let webwork_replay = match bind_webwork_replay(&question, &issued, Some(&presentation))
            {
                Ok(value) => value,
                Err(error) => return backend_error_response(error),
            };
            let presentation_snapshot = receipt_presentation(presentation.clone());
            let issued_question_snapshot = match issued_question_snapshot(
                &question,
                issued.flat_grading_capability,
                issued.webwork_grading_capability,
                issued.qti_grading_capability,
                Some(&presentation_snapshot),
                Vec::new(),
            ) {
                Ok(value) => value,
                Err(error) => return backend_error_response(error),
            };
            let native_execution_envelope_capability =
                native_execution_envelope_capability(&question, issued.flat_grading_capability);
            let value = PrefetchedQuestionDescriptorV1 {
                tenant: authenticated.tenant_context.tenant_id(),
                run: run.id,
                predecessor,
                assignment_position,
                problem: reference.problem,
                question_version: reference.version,
                issued_question_snapshot,
                seed,
                parameter_hash: issued.parameter_hash.clone(),
                provenance: issued.provenance.clone(),
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: PresentationBindingV1::new(
                    presentation.envelope.presentation_nonce,
                    presentation.digest,
                ),
                presentation_snapshot,
                grading_envelope: issued.envelope.clone(),
                native_execution_envelope_capability,
                flat_grading_capability: issued.flat_grading_capability,
                webwork_grading_capability: issued.webwork_grading_capability,
                qti_grading_capability: issued.qti_grading_capability,
            };
            let private_execution = PrefetchedPrivateExecutionV1 {
                flat_grading: issued.flat_grading.clone(),
                webwork_replay,
                webwork_grading: issued.webwork_grading.clone(),
                qti_grading: issued.qti_grading.clone(),
            };
            let reservation = match state
                .store
                .reserve_or_resume_prefetched_question(
                    authenticated.tenant_context,
                    learning_data_access::ReservePrefetchedQuestionCommand {
                        actor,
                        binding,
                        reservation: value.clone(),
                        private_execution,
                    },
                )
                .await
            {
                Ok(value) => value,
                Err(StoreError::Conflict) => match state
                    .store
                    .learner_get_prefetched_question(
                        authenticated.tenant_context,
                        actor,
                        run.id,
                        predecessor,
                        assignment_position,
                    )
                    .await
                {
                    Ok(Some(value)) => value,
                    Ok(None) => {
                        return error_response(StatusCode::CONFLICT, "attempt is no longer active");
                    }
                    Err(error) => return store_error_response(error),
                },
                Err(error) => return store_error_response(error),
            };
            let issued = (reservation == value).then_some(issued);
            (reservation, issued)
        }
    };
    let issued = match issued {
        Some(value) => value,
        None => match state
            .backend
            .issue(
                authenticated.tenant_context,
                reference,
                &question,
                reservation.seed,
            )
            .await
        {
            Ok(value) => value,
            Err(error) => return backend_error_response(error),
        },
    };
    if issued.parameter_hash != reservation.parameter_hash
        || issued.provenance != reservation.provenance
        || issued.envelope.version != reservation.question_version
        || issued.envelope.seed != Seed::new(reservation.seed)
        || issued.flat_grading_capability != reservation.flat_grading_capability
        || issued.webwork_grading_capability != reservation.webwork_grading_capability
        || issued.qti_grading_capability != reservation.qti_grading_capability
    {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "prefetched question did not reproduce exactly",
        );
    }
    let receipt_presentation = match question_model::presentation::rebuild_public_presentation_v1(
        &reservation.presentation_snapshot.envelope,
        &reservation.presentation_snapshot.asset_bindings,
    ) {
        Ok(value) => value,
        Err(error) => {
            return backend_error_response(RunBackendError::Invalid(format!(
                "prefetched receipt presentation is invalid: {error}"
            )));
        }
    };
    if receipt_presentation.digest != reservation.presentation.digest()
        || receipt_presentation.envelope.presentation_nonce != reservation.presentation.nonce()
    {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "prefetched receipt presentation does not match its binding",
        );
    }
    // Reissue proves the public deterministic rendering.  Private WebWork
    // replay is deliberately not readable from the descriptor: the Store's
    // sealed broker compares it when reservation/promotion is performed.
    no_store(
        Json(PrefetchedNextQuestion {
            predecessor,
            run: run.id,
            assignment_position,
            question_version: reference.version,
            seed: Seed::new(reservation.seed),
            rendered_question_sha256: reservation.provenance.rendered_question_sha256,
            pool_selection: pool_selection_for_position(&run_items, assignment_position),
            envelope: issued.envelope,
        })
        .into_response(),
    )
}

pub(super) async fn ensure_active_questions<S, B>(
    store: &S,
    backend: &B,
    authenticated: &AuthenticatedSession,
    binding: LearnerWorkRoutingBinding,
    run: &AssignmentRun,
    predecessor: Option<QuestionAttemptId>,
) -> Result<(), Response>
where
    S: Store + CatalogStore,
    B: RunBackend,
{
    if run.completed_at.is_some() {
        return Ok(());
    }
    require_run_binding(store, authenticated, binding, run).await?;
    let run_items = store
        .learner_assignment_run_items(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            run.id,
        )
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "run not found"))?;
    let attempts = all_attempts(store, authenticated, run.id).await?;

    if attempts.iter().any(|attempt| attempt.response.is_none()) {
        return Ok(());
    }

    for item in &run_items {
        let position = item.issued_position;
        let reference = item.reference;
        if attempts
            .iter()
            .all(|attempt| attempt.assignment_position != position)
        {
            let question = load_run_question(store, authenticated, reference).await?;
            let prefetched = match predecessor {
                Some(predecessor) => store
                    .learner_get_prefetched_question(
                        authenticated.tenant_context,
                        authenticated.record.subject.user(),
                        run.id,
                        predecessor,
                        position,
                    )
                    .await
                    .map_err(store_error_response)?,
                None => None,
            }
            .filter(|value| {
                value.tenant == authenticated.tenant_context.tenant_id()
                    && value.run == run.id
                    && value.assignment_position == position
                    && value.problem == reference.problem
                    && value.question_version == reference.version
            });
            issue_question(
                store,
                backend,
                authenticated,
                run,
                IssueQuestionRequest {
                    binding,
                    assignment_position: position,
                    reference,
                    question: &question,
                    prefetched,
                    predecessor_submission: predecessor,
                },
            )
            .await?;
            return Ok(());
        }
    }

    for item in &run_items {
        let position = item.issued_position;
        let reference = item.reference;
        let position_attempts: Vec<_> = attempts
            .iter()
            .filter(|attempt| attempt.assignment_position == position)
            .collect();
        let question = load_run_question(store, authenticated, reference).await?;
        if position_attempts
            .iter()
            .filter_map(|attempt| attempt.result)
            .any(|result| result.correct)
        {
            continue;
        }
        let attempt_count = u32::try_from(position_attempts.len()).map_err(|_| {
            error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "question attempt count overflow",
            )
        })?;
        if question
            .attempt_policy
            .max_attempts
            .is_some_and(|maximum| attempt_count >= maximum)
        {
            continue;
        }
        issue_question(
            store,
            backend,
            authenticated,
            run,
            IssueQuestionRequest {
                binding,
                assignment_position: position,
                reference,
                question: &question,
                prefetched: None,
                predecessor_submission: predecessor,
            },
        )
        .await?;
        return Ok(());
    }
    Ok(())
}

/// Confirms that a route-owned binding names the exact authorized run source.
///
/// ASVS 8.2.2 and 8.4.1: a mismatched object or tenant route is concealed
/// before prefetch state can be reserved. The binding remains routing context;
/// the Store independently prepares and authorizes learner work before issue.
pub(super) async fn require_run_binding<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    binding: LearnerWorkRoutingBinding,
    run: &AssignmentRun,
) -> Result<(), Response> {
    let assignment = owned_assignment_for_run(store, authenticated, run).await?;
    if assignment.id != binding.assignment || assignment.course_id != binding.course {
        return Err(error_response(StatusCode::NOT_FOUND, "run not found"));
    }
    Ok(())
}

pub(super) async fn load_run_question<S: CatalogStore>(
    store: &S,
    authenticated: &AuthenticatedSession,
    reference: ProblemVersionRef,
) -> Result<QuestionDefinition, Response> {
    let record = store
        .get_catalog_problem(authenticated.tenant_context, reference)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "question version not found"))?;
    let question = record.question;
    if record.problem != reference.problem
        || record.version != reference.version
        || question.problem != reference.problem
        || question.version != reference.version
    {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "published question identity does not match the requested version",
        ));
    }
    if question.attempt_policy.max_attempts == Some(0) {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "question max attempts must be greater than zero",
        ));
    }
    Ok(question)
}

pub(super) struct IssueQuestionRequest<'a> {
    binding: LearnerWorkRoutingBinding,
    assignment_position: u32,
    reference: ProblemVersionRef,
    question: &'a QuestionDefinition,
    /// Answer-free durable prefetch descriptor.  The matching private
    /// contracts are reissued by the trusted backend and compared only by the
    /// Store's sealed promotion path.
    prefetched: Option<PrefetchedQuestionDescriptorV1>,
    predecessor_submission: Option<QuestionAttemptId>,
}

pub(super) async fn issue_question<S, B>(
    store: &S,
    backend: &B,
    authenticated: &AuthenticatedSession,
    run: &AssignmentRun,
    request: IssueQuestionRequest<'_>,
) -> Result<QuestionAttempt, Response>
where
    S: Store,
    B: RunBackend,
{
    let (
        issued_question_snapshot,
        seed,
        parameter_hash,
        provenance,
        presentation_capability,
        presentation,
        presentation_snapshot,
        grading_envelope,
        flat_grading,
        flat_grading_capability,
        webwork_replay,
        webwork_grading,
        webwork_grading_capability,
        qti_grading,
        qti_grading_capability,
        native_execution_envelope_capability,
    ) = match request.prefetched.as_ref() {
        Some(value) => {
            // The descriptor is intentionally answer-free. Reconstruct the
            // private contracts from the deterministic server backend; the
            // Store later compares/promotes those opaque contracts under its
            // private authority without exposing them through a read API.
            let issued = backend
                .issue(
                    authenticated.tenant_context,
                    request.reference,
                    request.question,
                    value.seed,
                )
                .await
                .map_err(backend_error_response)?;
            if issued.parameter_hash != value.parameter_hash
                || issued.provenance != value.provenance
                || issued.envelope != value.grading_envelope
                || issued.envelope.seed != Seed::new(value.seed)
                || issued.flat_grading_capability != value.flat_grading_capability
                || issued.webwork_grading_capability != value.webwork_grading_capability
                || issued.qti_grading_capability != value.qti_grading_capability
                || native_execution_envelope_capability(
                    request.question,
                    issued.flat_grading_capability,
                ) != value.native_execution_envelope_capability
            {
                return Err(backend_error_response(RunBackendError::Invalid(
                    "prefetched question did not reproduce exactly".into(),
                )));
            }
            let presentation = question_model::presentation::rebuild_public_presentation_v1(
                &value.presentation_snapshot.envelope,
                &value.presentation_snapshot.asset_bindings,
            )
            .map_err(|error| {
                backend_error_response(RunBackendError::Invalid(format!(
                    "prefetched receipt presentation is invalid: {error}"
                )))
            })?;
            if presentation.digest != value.presentation.digest()
                || presentation.envelope.presentation_nonce != value.presentation.nonce()
            {
                return Err(backend_error_response(RunBackendError::Invalid(
                    "prefetched receipt presentation does not match its binding".into(),
                )));
            }
            let webwork_replay =
                bind_webwork_replay(request.question, &issued, Some(&presentation))
                    .map_err(backend_error_response)?;
            (
                value.issued_question_snapshot.clone(),
                value.seed,
                value.parameter_hash.clone(),
                value.provenance.clone(),
                value.presentation_capability,
                Some(value.presentation),
                Some(value.presentation_snapshot.clone()),
                Some(value.grading_envelope.clone()),
                issued.flat_grading,
                value.flat_grading_capability,
                webwork_replay,
                issued.webwork_grading,
                value.webwork_grading_capability,
                issued.qti_grading,
                value.qti_grading_capability,
                value.native_execution_envelope_capability,
            )
        }
        None => {
            let seed = fresh_seed().map_err(backend_error_response)?;
            let issued = backend
                .issue(
                    authenticated.tenant_context,
                    request.reference,
                    request.question,
                    seed,
                )
                .await
                .map_err(backend_error_response)?;
            let presentation = fresh_presentation(
                store,
                authenticated.tenant_context,
                request.reference,
                &issued.envelope,
            )
            .await
            .map_err(backend_error_response)?;
            let webwork_replay =
                bind_webwork_replay(request.question, &issued, presentation.as_ref())
                    .map_err(backend_error_response)?;
            let presentation_capability = if presentation.is_some() {
                PresentationCapability::EnvelopeV1
            } else {
                PresentationCapability::NotApplicable
            };
            let presentation_snapshot = presentation.clone().map(receipt_presentation);
            let grading_envelope = presentation.as_ref().map(|_| issued.envelope.clone());
            let flat_grading = issued.flat_grading;
            let flat_grading_capability = issued.flat_grading_capability;
            let webwork_grading = issued.webwork_grading;
            let webwork_grading_capability = issued.webwork_grading_capability;
            let qti_grading = issued.qti_grading;
            let qti_grading_capability = issued.qti_grading_capability;
            let native_execution_envelope_capability =
                native_execution_envelope_capability(request.question, flat_grading_capability);
            let native_physical_asset_bindings = if matches!(
                request.question.source,
                question_model::QuestionSource::Native { .. }
            ) && matches!(
                flat_grading_capability,
                FlatGradingCapability::NotApplicable
            ) {
                issued_native_physical_bindings(
                    store,
                    authenticated.tenant_context,
                    request.reference,
                    &issued.envelope,
                )
                .await
                .map_err(backend_error_response)?
            } else {
                Vec::new()
            };
            let issued_question_snapshot = issued_question_snapshot(
                request.question,
                flat_grading_capability,
                webwork_grading_capability,
                qti_grading_capability,
                presentation_snapshot.as_ref(),
                native_physical_asset_bindings,
            )
            .map_err(backend_error_response)?;
            (
                issued_question_snapshot,
                seed,
                issued.parameter_hash,
                issued.provenance,
                presentation_capability,
                presentation.map(|presentation| {
                    PresentationBindingV1::new(
                        presentation.envelope.presentation_nonce,
                        presentation.digest,
                    )
                }),
                presentation_snapshot,
                grading_envelope,
                flat_grading,
                flat_grading_capability,
                webwork_replay,
                webwork_grading,
                webwork_grading_capability,
                qti_grading,
                qti_grading_capability,
                native_execution_envelope_capability,
            )
        }
    };
    store
        .issue_or_resume_question_attempt(
            authenticated.tenant_context,
            IssueQuestionAttemptCommand {
                actor: authenticated.record.subject.user(),
                binding: request.binding,
                attempt: QuestionAttemptId::generate(),
                run: run.id,
                assignment_position: request.assignment_position,
                problem: request.reference.problem,
                question_version: request.reference.version,
                issued_question_snapshot,
                seed,
                parameter_hash,
                provenance,
                presentation_capability,
                presentation,
                presentation_snapshot,
                grading_envelope,
                native_execution_envelope_capability,
                flat_grading,
                flat_grading_capability,
                webwork_replay,
                webwork_grading,
                webwork_grading_capability,
                qti_grading,
                qti_grading_capability,
                prefetched: request.prefetched,
                predecessor_submission: request.predecessor_submission,
            },
        )
        .await
        .map_err(store_error_response)
}

#[cfg(test)]
mod presentation_snapshot_tests;
