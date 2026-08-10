//! Run issuance and prefetch capability; this module owns its route behavior.

use super::contracts::RunBackend;
use super::queries::{all_attempts, owned_assignment_for_run, owned_enrollment, owned_run};
use super::support::*;

struct PersistedNonceSource(Option<[u8; 16]>);

impl NonceSourceV1 for PersistedNonceSource {
    fn next_nonce(&mut self) -> Result<[u8; 16], PresentationBuildError> {
        self.0
            .take()
            .ok_or(PresentationBuildError::RenderedIdCollision)
    }
}

fn fresh_presentation(envelope: &QuestionEnvelope) -> Result<PresentationV1, Response> {
    build_presentation_v1(envelope, &[]).map_err(|error| {
        error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &format!("question presentation is invalid: {error}"),
        )
    })
}

fn reproduce_presentation(
    envelope: &QuestionEnvelope,
    binding: PresentationBindingV1,
) -> Result<PresentationV1, Response> {
    let mut nonce = PersistedNonceSource(Some(binding.nonce().as_bytes()));
    let presentation =
        build_presentation_v1_with_nonce_source(envelope, &[], &mut nonce).map_err(|error| {
            error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                &format!("question presentation cannot be reproduced: {error}"),
            )
        })?;
    if presentation.digest != binding.digest() {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "question presentation did not reproduce exactly",
        ));
    }
    Ok(presentation)
}

/// Prepares the next still-unattempted assignment position while the current
/// question remains the sole active attempt. This is intentionally POST: a
/// successful request creates a durable server reservation, but no timer or
/// activity transition.
pub(super) async fn prefetch_next_question<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(predecessor): Path<QuestionAttemptId>,
    body: axum::body::Body,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
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
        .get_question_attempt(authenticated.tenant_context, predecessor)
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
        owned_assignment_for_run(state.store.as_ref(), &authenticated, &run).await
    {
        return response;
    }
    let run_items = match state
        .store
        .assignment_run_items(authenticated.tenant_context, run.id)
        .await
    {
        Ok(items) => items,
        Err(error) => return store_error_response(error),
    };
    let attempts =
        match all_attempts(state.store.as_ref(), authenticated.tenant_context, run.id).await {
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
        .get_prefetched_question(
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
            let presentation = match fresh_presentation(&issued.envelope) {
                Ok(value) => value,
                Err(response) => return response,
            };
            let value = learning_data_access::PrefetchedQuestion {
                tenant: authenticated.tenant_context.tenant_id(),
                run: run.id,
                predecessor,
                assignment_position,
                problem: reference.problem,
                question_version: reference.version,
                seed,
                parameter_hash: issued.parameter_hash.clone(),
                provenance: issued.provenance.clone(),
                presentation: PresentationBindingV1::new(
                    presentation.envelope.presentation_nonce,
                    presentation.digest,
                ),
            };
            let reservation = match state
                .store
                .reserve_or_resume_prefetched_question(
                    authenticated.tenant_context,
                    learning_data_access::ReservePrefetchedQuestionCommand {
                        actor,
                        reservation: value.clone(),
                    },
                )
                .await
            {
                Ok(value) => value,
                Err(StoreError::Conflict) => match state
                    .store
                    .get_prefetched_question(
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
    {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "prefetched question did not reproduce exactly",
        );
    }
    if let Err(response) = reproduce_presentation(&issued.envelope, reservation.presentation) {
        return response;
    }
    no_store(
        Json(PrefetchedNextQuestion {
            predecessor,
            run: run.id,
            assignment_position,
            question_version: reference.version,
            seed: Seed::new(reservation.seed),
            rendered_question_sha256: reservation.provenance.rendered_question_sha256,
            envelope: issued.envelope,
        })
        .into_response(),
    )
}

pub(super) async fn ensure_active_questions<S, B>(
    store: &S,
    backend: &B,
    authenticated: &AuthenticatedSession,
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
    let enrollment = owned_enrollment(store, authenticated, run.enrollment).await?;
    let _assignment = store
        .get_assignment(authenticated.tenant_context, enrollment.assignment)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "assignment not found"))?;
    let run_items = store
        .assignment_run_items(authenticated.tenant_context, run.id)
        .await
        .map_err(store_error_response)?;
    let attempts = all_attempts(store, authenticated.tenant_context, run.id).await?;

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
                    .get_prefetched_question(
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
                assignment_position: position,
                reference,
                question: &question,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await?;
        return Ok(());
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
    assignment_position: u32,
    reference: ProblemVersionRef,
    question: &'a QuestionDefinition,
    prefetched: Option<learning_data_access::PrefetchedQuestion>,
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
    let (seed, parameter_hash, provenance, presentation) = match request.prefetched.as_ref() {
        Some(value) => (
            value.seed,
            value.parameter_hash.clone(),
            value.provenance.clone(),
            value.presentation,
        ),
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
            let presentation = fresh_presentation(&issued.envelope)?;
            (
                seed,
                issued.parameter_hash,
                issued.provenance,
                PresentationBindingV1::new(
                    presentation.envelope.presentation_nonce,
                    presentation.digest,
                ),
            )
        }
    };
    store
        .issue_or_resume_question_attempt(
            authenticated.tenant_context,
            IssueQuestionAttemptCommand {
                actor: authenticated.record.subject.user(),
                attempt: QuestionAttemptId::generate(),
                run: run.id,
                assignment_position: request.assignment_position,
                problem: request.reference.problem,
                question_version: request.reference.version,
                seed,
                parameter_hash,
                provenance,
                presentation,
                prefetched: request.prefetched,
                predecessor_submission: request.predecessor_submission,
            },
        )
        .await
        .map_err(store_error_response)
}
