# Plan: audited Student work and calculated Gradebook

## Context

`WP-INST-G2` supplies one Instructor-facing calculated Gradebook and one deliberate inspection of a named Student's immutable issued work. It consumes accepted `WP-INST-S6` course-grade calculation and `WP-INST-G1` operation references. The [implementation status registry](../implementation_status.md) allocates package and migration ownership.

The system is stable enough for this architecture change: the preceding material-tree Validation completed green, G1 is accepted, and current-source review confirms separate calculation, operation, evidence, and course-record boundaries. `domain::course_grade::calculate_course_grade` owns calculation, `CourseGradebookStore` owns grade-scheme reads, and G1 owns metadata-only operation rows and receipts. G2 joins them through one calculated-page contract and one audited-detail contract.

## Objectives

1. Present a roster-first Gradebook whose totals are calculated only on the server under the selected course-grade scheme.
2. Let an Instructor select one named Student and exact run from either a Gradebook row or an operation context.
3. Show immutable issued presentation and the Student's submitted answer through a solution-free, `no-store`, audit-recorded detail response.
4. Keep navigation, list, operation, cursor, audit, and ordinary screenshot contracts answer-free.
5. Make the next Instructor action visible at the required 1280 by 800 desktop viewport.

## Design philosophy

- **One calculator.** `domain::course_grade::calculate_course_grade` remains the sole course-total authority.
- **One calculated Gradebook.** `CourseGradebookStore` assembles a bounded, roster-ordered server projection from current summaries and the active scheme. Derived totals remain derived.
- **One inspection broker.** `StudentWorkInspectionStore` resolves the public composite, verifies immutable evidence, writes both audit facts, and returns the allowed detail projection.
- **One exact Student choice.** Operation and Gradebook routes converge only after an Instructor chooses a named Student and run.
- **One safe rendering ownership boundary.** `question_model::presentation` translates durable submitted responses into the identifiers and labels of the issued presentation. Browser code renders that closed projection.

## Scope

### Calculated Gradebook and continuation

`GET /api/courses/{course}/gradebook` becomes the canonical calculated, cursor-paged Gradebook read:

```text
CalculatedGradebookPage {
  scheme_revision, mode, rounding, roster_revision, observation_time,
  continuation_witness, next_cursor, rows: CalculatedGradebookRow[]
}
CalculatedGradebookRow {
  membership_ref, display_label, course_grade_outcome,
  assignment_cells: CalculatedAssignmentCell[]
}
CalculatedAssignmentCell {
  assignment_ref, title, inclusion_or_category_context, scoring_state,
  selected_score_summary, inspection_choice
}
AssignmentInspectionChoice =
  | { kind: "selected_run", basis, run_ref, submitted_at }
  | { kind: "choose_run", completed_run_count }
  | { kind: "no_submitted_run" }
GradebookFilterRequest {
  assignment_ref?, membership_ref?, operation_ref?
}
```

The closed filter request accepts zero or one typed filter scope. Operation context is resolved server-side. Its response supplies one explicit selection union:

```text
GradebookSelectionResult =
  | { kind: "single_student", membership_ref, assignment_ref, inspection_choice }
  | { kind: "student_selection", rows: StudentSelectionRow[], next_cursor? }
StudentSelectionRow {
  membership_ref, display_label, assignment_ref, inspection_choice
}
```

`student_selection` is bounded, roster-ordered, answer-free, and contains only safe Student labels,
public membership and assignment locators, and the shared `AssignmentInspectionChoice`. Every
`selected_run` row opens the canonical detail route; every `choose_run` row opens the same bounded run
chooser used by a Gradebook cell. It gives an Instructor a named choice before opening inspected
work. `single_student` permits the direct route for a Student-grouped operation. The normalized
operation reference, filter, structural revisions, selection result, and last membership position
bind into the opaque selection cursor. Gradebook page cursors bind scheme revision, roster revision,
normalized filters, and last structural roster position.

`selected_run` names the exact run supplying `current_score` under the assignment's first, latest,
highest, or Instructor-selected policy. The visible inspect action names the Student, assignment, run
basis, and submitted time. `choose_run` opens a bounded semantic run chooser when the current score
does not select one exact run; the chooser marks any score-selected run, labels every submitted run,
and restores focus to its invoking Gradebook cell when dismissed. The canonical detail request begins
only after this human-visible run choice is exact.

PostgreSQL reads every page in one repeatable-read snapshot. A structural scheme or roster revision produces the typed reload result. Cursor continuation preserves structural roster order; score outcomes can advance on each later page and each page carries its own observation time plus per-assignment scoring generation/status witness. The interface labels that live state truthfully.

All public references use existing typed, human-safe forms. The browser receives no internal tenant, enrollment, attempt, submission, job, provider, or database identifiers. `CourseGradeExportRow` is an export-only PII type with no page serializer or conversion into `CalculatedGradebookRow`. Grade Settings owns scheme configuration and audited CSV export; Gradebook owns calculated Student totals.

### Audited Student-work inspection

The canonical browser destination and API are:

```text
/instructor/courses/:courseRef/gradebook/students/:membershipRef/assignments/:assignmentRef/runs/:runRef
GET /api/courses/{course}/gradebook/students/{membership}/assignments/{assignment}/runs/{run}
```

The Store resolves the full `TenantId + CourseId + CourseMembershipReference + AssignmentReference + RunReference` composite, active direct-Instructor membership, retention state, immutable receipt/issued-capability identity, disclosure policy, and scoring state. Its `return_context` is a closed Gradebook or grading-operation context with safe public references and a focus target; reload/back/direct-link recovery restores that context or gives a visible reselect action when exact evidence is unavailable.

`question_model::presentation` owns the pure `project_durable_response_to_rendered_v1` conversion. It accepts durable submitted response plus verified issued-presentation descriptor and returns `InspectedStudentResponseV1`, a closed union of rendered response identifiers, display text, and safe typed artifact/external states. Its variants express allowed Student-facing facts such as a rendered selected option, entered text, uploaded-artifact state, or external-tool completion state. They carry no answer key, expected response, checker/rubric, private source, provider payload, hidden diagnostic, canonical durable object keys, or grading authority. The server performs this projection before serialization.

This explicit detail is response-bearing and solution-free. Each submitted response owns one verified evidence state: `IssuedPresentation`, which carries the immutable `ReceiptPresentationSnapshot` and recomputed `PresentationDigestV1`, or `PresentationNotApplicable`, which proves the checksummed no-presentation capability and absent issued tuple for an ExternalTool submission. It shows the named Student's submitted response, the applicable verified evidence, timing, score/correctness-only feedback, and scoring state with `Cache-Control: no-store`. Hint, rationale, and correct-response content remain outside this closed detail type until a separately named authoring/storage classification can machine-enforce solution-free instructional content. URLs, cursors, browser storage, operation receipts, ordinary page responses, and audit payloads remain answer-free.

`InspectedStudentWorkDetailV1` also returns required current presentation facts
`student_display_label` and `assignment_title`. The broker resolves the active course roster
profile's validated display label and the current course assignment title after it has validated the
complete authorized composite.
Both values are bounded safe display strings and are response projection only: they do not enter
the return context, URL, cursor, browser storage, `record_access_log`, or `audit_event`. The ready
detail uses them to name the Student and assignment, so ordinary navigation, reload, and direct
entry have the same server-owned context.

### Authority, audit, and request boundary

The audit-writing GET accepts two closed browser request profiles before Store invocation:

1. a same-origin fetch or same-origin navigation with `Sec-Fetch-Site: same-origin` and the matching
   fetch or navigation mode and destination; or
2. an explicit user-initiated top-level navigation with `Sec-Fetch-Site: none`,
   `Sec-Fetch-Mode: navigate`, `Sec-Fetch-Dest: document`, and `Sec-Fetch-User: ?1`.

The server applies an exact header decision table. Cross-site resource or navigation initiation and
requests outside these profiles receive the same generic secure unavailable response as other
concealed authorization failures. Session, tenant, direct-Instructor course authority, purpose,
action, target, and audit payload are derived only by the server.

`StudentWorkInspectionStore::inspect_student_work` performs one atomic transaction:

1. Resolve authenticated session, tenant, direct-Instructor course membership, request-origin witness, and retention state.
2. Resolve the public composite and verify immutable receipt, issued-capability/presentation, disclosure, and scoring evidence.
3. Write the server-owned Student-record `record_access_log` fact and metadata-only `audit_event` fact.
4. Return the closed solution-free detail projection.

Successful audit records contain the server-derived actor, purpose `gradebook_inspection`, action, authoritative timestamp, protected internal `TenantId`, `CourseId`, `CourseMembershipId`, `AssignmentId`, and `RunId`, plus one ordered per-submission evidence witness. Each witness retains only its internal attempt identity, receipt timestamp, and either an issued-presentation digest or the closed no-presentation capability state. Browser navigation uses public course, membership, assignment, and run references only; those locators do not enter persisted inspection facts. Audit facts contain no response, score, feedback body, email, key, provider payload, source, diagnostic, token, lease, SQL value, or public UUID. The SQL broker uses typed parameters for every request-provided value. A failure to append either audit row makes detail unavailable.

The W4 route and SQL broker own denial telemetry. They classify rejected request origin, authorization, and broker failures with server-owned reason/context before the Store can read Student work. That telemetry records no Student-work target or Student-record access fact, so the access log remains an accurate statement of successful FERPA record reads. W2B returns the ordinary concealed unavailable result and appends neither successful inspection fact when its resolved evidence is unavailable.

### Migration authority

Every migration leaves a closed authority state; each migration establishes its own owner, fixed `search_path`, explicit revocations, least-privilege grants, and catalog/ACL proof before the next migration adds capability.

| Migration                                           | Owner                | Atomic responsibility                                                                                                                                                                                |
| --------------------------------------------------- | -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `2026081870_student_work_inspection_authority.sql`  | inspection authority | Dedicated `NOLOGIN`/`NOINHERIT` owner role, fixed search path, baseline revocations, narrow grants, and catalog proof.                                                                               |
| `2026081871_student_work_inspection_witness.sql`    | immutable evidence   | Private composite receipt/issued-presentation/response witness, integrity rules, private access owner, and catalog proof.                                                                            |
| `2026081872_student_work_inspection_capability.sql` | inspection broker    | The only application-executable inspection function, owned by the dedicated role with fixed search path, parameter-bound SQL, revocations/grant, composite resolution, and atomic dual audit writes. |
| `2026081873_student_work_inspection_indexes.sql`    | query evidence       | Query-demonstrated indexes and bounded plan evidence while retaining the closed broker authority.                                                                                                    |
| `2026081874_tenant_bound_worker_failure.sql`        | queue failure broker | Tenant-bound failure finalization, exact leased-job ownership, publisher adaptation, and retirement of the unscoped capability.                                                                       |
| `2026081875_student_work_inspection_rowset_contract.sql` | broker rowset repair | Forward repair that aligns the broker's transient JSON rowset with exact PostgreSQL field names.                                                                                                      |
| `2026081876_student_work_inspection_safe_labels.sql` | inspection detail labels | Server-owned validated Student display label and assignment title in the existing audited inspection broker response, with labels excluded from audit facts.                                           |

The inspection capability is the single app-executable private-response reader.

## Positive ownership boundaries

`WP-INST-G3` owns item/course analysis and replacement impact after consuming G2's safe inspection seam. `WP-INST-G4` owns improvement threads and decisions. `WP-INST-G5` owns cross-course actionable Instructor work. Release packages own production onboarding and deployment. Existing deterministic grading and score publication retain mutable scoring authority.

## Architecture and implementation sequence

### G2-W1: freeze bindings and reserve migrations

**Owner:** architecture and documentation. Record this plan, decision record, handoff, four
inspection migrations, and the tenant-bound worker-failure migration discovered by the connected
Gradebook acceptance path.

**Narrow verification:** documentation-link and package/migration-registry checks pass.

### G2-W2A: calculated Gradebook contracts and Memory parity

**Owner files:** `crates/question_model/src/course_grade.rs`, public-route and cursor modules, `crates/learning-data-access/src/course_gradebook.rs`, and Memory course-grade/run implementations. Build closed page/row/cell/filter, cursor/reload, and export-separation contracts.

**Narrow verification:** deterministic model/Memory conformance for both grade modes, dropped work, live scoring state, cursor binding, and foreign references.

### G2-W2B: inspected-response/detail contracts and Memory parity

**Owner files:** `crates/question_model/src/presentation/`, inspected-detail models, `crates/domain/src/disclosure_policy.rs`, `StudentWorkInspectionStore`, and Memory inspection implementation. Build pure response projection, closed `IssuedPresentation`/`PresentationNotApplicable` evidence states, score/correctness-only inspection feedback, composite/retention/integrity checks, internal audit witnesses, and return context.

**Narrow verification:** exhaustive deterministic pure response-projection coverage remains in `question_model::presentation`; Store conformance covers one rendered response family and one verified `PresentationNotApplicable` ExternalTool receipt, plus exact composite binding, concealment, retention, and paired audit witnesses. This division proves each owner without duplicating a fixture matrix through Store setup.

### G2-W3A: PostgreSQL paged Gradebook

**Owner files:** `crates/learning-data-access/src/postgres/course_gradebook.rs`, the tenant-bound
`JobStore` failure contract, migration `2026081874`, and focused PostgreSQL support modules.
Implement bulk roster page assembly and stable structural continuation with page-local score
witnesses. Preserve tenant context when a recalculation worker reports terminal failure so the
assignment and grading-operation transition commits through the production queue capability.

**Narrow verification:** disposable PostgreSQL proof for repeatable-read page, reload response, structural order, live score witness, and export separation.

### G2-W3B: SQL inspection broker and PostgreSQL detail

**Owner files:** migrations `2026081870` through `2026081873`, forward broker-rowset repair
`2026081875`, safe-label projection `2026081876`, and
`crates/learning-data-access/src/postgres/student_work_inspection.rs`. Implement the closed
witness, only executable broker, and PostgreSQL Store projection/audit integration. The broker
joins `accepted_submission_private_response` through its full immutable submission composite,
recomputes canonical `StudentResponse` bytes, and compares `response_sha256` before projecting
that private response against the verified receipt presentation. Migration `1876` recreates only
the existing broker under its established owner, `SECURITY DEFINER`, fixed search path, and ACL
proof; it returns validated `student_display_label` and `assignment_title` in each transient row
after composite resolution, while the paired audit payload remains label-free.

**Narrow verification:** disposable PostgreSQL proof for role/RLS/ACL, parameter binding, origin
witness, exact composite, retention, atomic audit, generic security failures, required bounded
label/title values, cross-row agreement, and label-free audit payloads.

### G2-W4A: server routes

**Owner files:** `crates/server/src/course/gradebook.rs` and course routing. Expose server-owned page, selection, and detail contracts; enforce Fetch Metadata and `no-store`; derive audit facts server-side.

**Narrow verification:** offline server-route behavior for valid/reload/selection/unavailable states,
the closed same-origin and user-initiated navigation decision table, and a read-only grading boundary.

### G2-W4B: strict client and Instructor UI

**Owner files:** `src/api/client.ts`, strict decoders, route contract, Gradebook/operation/detail
page-model/CSS modules. Require `student_display_label` and `assignment_title` in the closed detail
decoder, then build course-total-first semantic Gradebook, explicit Student chooser, named
inspected detail, recovery, focus restoration, and return navigation from that one response.

**Narrow verification:** offline decoder/page behavior validates closed unions, required bounded
safe labels, status announcements, semantic table/hierarchy, visible focus, and restored context.

The UI contract keeps the course name, Gradebook heading, selected scheme, total-state summary, and
current recovery action ahead of the roster table. The roster uses semantic row and column headers.
Recalculating state announces its status and offers `Reload Gradebook`; a structural conflict explains
the change and reloads the first page; a failed assignment links to its existing Grading operations
surface; unavailable inspected work focuses a generic heading and offers `Return to Gradebook` with
the valid filter and Student/assignment context restored. Keyboard focus returns to the Gradebook
heading after reload and to the invoking cell after a chooser or detail return.

### G2-W5: connected and task-based evidence

**Owner files:** focused acceptance scenarios and browser/visual evidence. Exercise ordinary Student completion through Instructor Gradebook, named selection, audited detail, operation return, retry/recalculation recovery, and truthful live score state.

The named-detail journey selects the Student/run, displays the server-provided Student and
assignment labels, reloads the direct inspection URL, and displays the same labels after its one
audited detail request. It keeps Gradebook and grading-operation return behavior intact.

### G2-W6: documentation and final validation

**Owner files:** user/operational documentation, screenshots, changelog, and final material tree. Refresh documentation from proven contracts and run the validation authority.

## ASVS alignment

| ASVS requirement                                                            | G2 application                                                                                                                                                                                   |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| V1.1.1-V1.1.2; V1.2.1-V1.2.4, V1.2.10                                       | Decode once, render with context-appropriate escaping, bind every SQL value, and retain CSV formula protection at the separate export boundary.                                                  |
| V1.5.2-V1.5.3                                                               | Decode only closed request, cursor, response, and audit shapes with consistent UTF-8 JSON handling across Rust and TypeScript.                                                                   |
| V2.1.1-V2.1.3; V2.2.1-V2.2.3                                                | Document and enforce typed path/filter/composite rules, bounded pages, closed unions, and contextual consistency at the trusted service layer.                                                   |
| V2.3.1-V2.3.4                                                               | Keep selection and inspection in the declared order and make evidence verification plus both audit writes one atomic transaction.                                                                |
| V3.5.1-V3.5.3                                                               | Accept the closed same-origin and explicit user-initiated top-level navigation Fetch Metadata profiles and reject cross-site initiation before Store access.                                     |
| V4.1.1, V4.1.4                                                              | Return the declared UTF-8 JSON content type and expose only the registered HTTP methods.                                                                                                         |
| V8.1.1-V8.1.2; V8.2.1-V8.2.3; V8.3.1-V8.3.3; V8.4.1                         | Apply function-, record-, and field-level authorization from the originating Instructor at route, Store, and PostgreSQL layers, including tenant isolation and immediate membership state.       |
| V14.1.1-V14.1.2; V14.2.1-V14.2.7; V14.3.1-V14.3.3                           | Classify Student work, minimize the explicit response, honor retention, use `no-store`, keep it out of URLs and browser storage, and clear reactive detail state when the session or route ends. |
| V16.1.1; V16.2.1-V16.2.5; V16.3.2-V16.3.4; V16.4.1-V16.4.2; V16.5.1-V16.5.3 | Inventory the paired audit/security telemetry, use structured server-owned metadata and synchronized timestamps, protect logs, and return generic fail-closed errors.                            |

## Evidence classification and acceptance

| Evidence class                   | G2 evidence                                                                                                                                       |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Permanent                        | Deterministic offline Rust model/Store, server route, strict decoder, and page behavior with closed unions, required safe labels, and stable accessibility semantics. |
| Connected acceptance             | PostgreSQL/RLS/broker/role/retention/audit proof and ordinary Student-to-Instructor HTTPS journey, including reload/direct-entry label continuity. |
| Visual acceptance                | 1280 by 800 Instructor review of total-first Gradebook, Student chooser, named audited detail, reload/direct-entry continuity, return/recovery, hierarchy, focus, and contrast. |
| One-time implementation evidence | Query-plan/index rationale, migration fresh/no-op/checksum observations, rendered-review notes, and independent architecture/security/HCI review. |

G2 is accepted when both grade modes produce server-calculated totals; structural continuation
reloads cleanly while each page states its score witness; a named Student's immutable work is
inspectable through an atomic audit; the server-owned validated Student display label and assignment
title persist through reload and direct entry without entering audits; Memory/PostgreSQL have the
same observable behavior; the task-based live flow and visual review pass; and
`source source_me.sh && ./all_test.sh` passes on the final material tree.

## Documentation updates

`docs/DESIGN_DECISIONS.md` records the authority and rendering decision. `implementation_status.md` remains the package/migration authority. The implementation and Instructor capability plans, roadmap, TODO, and changelog link to this focused boundary. G2-W6 refreshes operational/Instructor documentation after the implemented workflow is proven.
