# Remaining local release verification

Status: planned verification; no new runtime acceptance is claimed by this cleanup.
This owns the unresolved current-scope checks from
[production_release_readiness_phase_plan-v2.md](../../archive/production_release_readiness_phase_plan-v2.md).
Earlier phase receipts remain historical evidence. [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md)
and the current Question specifications define behavior; old unimplemented proposals do not add scope.

## Scope and ownership

The manager and fresh implementation, test, and independent review agents complete this plan.
Use existing APIs, current controls, local disposable services, and seeded Accounts. The manager
provisions and repairs the local test environment; no step needs production credentials, a real
email account, a human walkthrough, or a deployment decision. Independent source and behavior
assessment resolves engineering choices. Preserve existing supported behavior and repair only
demonstrated gaps against current guidance.

Daemon backends, unproven Sysadmin repair tools, external email-provider acceptance, production
deployment, and schema freeze remain outside local completion. Record them as deferred or separate
operational work without claiming that local tests prove them.

## Milestones and recovery

1. **Reconcile remaining outcomes.** An independent reviewer compares the old Phase 3/4 claims
   with current guidance, source, and named acceptance receipts. Record each as current behavior
   to verify, already accepted with evidence, or superseded/deferred with its authority. Keep
   Instructor Student View, current Question/Blueprint archive and restore behavior, and existing
   local readiness/recovery checks in this reconciliation. Resolve contradictory historical
   requirements from current guidance rather than implementing both models.
2. **Verify Instructor preview.** Run the ordinary Instructor-to-Student-View-to-Instructor browser
   journey. Check answer-free content, retained Instructor identity and edit context, and absence
   of Student Work mutations. Use existing Native JSON and WeBWorK preview paths. An independent
   agent checks network/state evidence and representative captures. Fix a demonstrated failure at
   the existing preview owner and repeat the affected journey.
3. **Verify archive and restore.** Exercise current authorized Question and Blueprint controls
   through ordinary API/browser paths. Check unauthorized denial, current visibility rules,
   restoration, and preservation of exact Revision references. Use current specs rather than the
   old plan's superseded blanket discovery restrictions. Repair observed failures and repeat
   focused SQL/API/browser checks; do not introduce a new lifecycle or permission model.
4. **Verify local readiness and recovery.** Run existing local startup, readiness, and
   database-baseline acceptance. Exercise setup email with an agent-owned local SMTP receiver and
   the unconfigured delivery path. Verify failures retain saved work and report useful redacted
   errors. Repair demonstrated current-scope defects; daemon scheduling and real-provider delivery
   remain explicitly deferred.
5. **Close with independent evidence.** A fresh review agent checks every current-scope row and
   its failure recovery. Run applicable focused checks and `source source_me.sh && ./launchers/all_test.sh`;
   capture changed UI with `./devel/capture_screenshots.sh` if implementation changes it. Fix
   failures and repeat affected checks. Update the evidence ledger, TODO, and changelog, then
   archive this plan with `git mv`. The manager records completion without human sign-off.

## Test boundary

Reuse permanent behavior tests. Keep one-time source reconciliation, failure injection, and
capture probes under ignored `tests/_temp/`, and remove them after recording results. Additional
permanent tests must satisfy [PYTEST_STYLE.md](../../PYTEST_STYLE.md#permanent-test-checklist).
Do not impose byte equality, pixel equality, arbitrary timing thresholds, or an exhaustive matrix
as a replacement for the named user-visible outcomes.
