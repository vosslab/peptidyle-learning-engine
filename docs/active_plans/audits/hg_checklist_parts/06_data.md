## Data and history

- [ ] Answers, keys, grading, and correctness decisions should stay on the server, out of reach of **Students**.
  - Mismatch: The cited history reader authorizes one completed-Attempt projection and the expiry test finalizes grading; neither proves every answer, key, grading, and correctness path remains server-only and unreachable to Students.
- [ ] Public data should stay separate from private, answer-bearing, identifying, or radioactive FERPA data.
  - Mismatch: The aggregate-statistics tables and private receipts show one separation boundary, not a complete audit of public data and every private, answer-bearing, identifying, or FERPA-sensitive path.
- [ ] Human-readable titles and identifiers should be used wherever people must recognize, copy, or enter them.
  - Mismatch: `assignment_title` in one Student-history projection does not prove that all recognition, copy, and entry surfaces use human-readable titles and identifiers.
- [ ] FERPA-sensitive Student data should not become ordinary logs, analytics, URLs, or long-lived browser storage.
  - Mismatch: `src/log.ts` only describes browser console logging and explicitly leaves redaction for later; it does not prove that all logs, analytics, URLs, and browser storage exclude FERPA-sensitive Student data.
- [x] Opaque IDs remain FERPA-sensitive when they link a Student to Course activity.
  - Evidence (source): `schemas/base_schema/authorization.sql` `current_session_account_owns_student_record` authorizes a Student record only through its Course and active membership.

### Human-facing reference IDs

- [ ] Human-facing reference IDs should be short, opaque, easy to communicate, and should not reveal creation order, counts, database keys, ownership, or other object metadata.
  - Evidence (source): `crates/question_model/src/public_route.rs` defines opaque typed read/use formats and `src/navigation/public_route.ts` accepts only their typed forms at browser route boundaries.
  - Mismatch: no connected creation/allocation proof establishes cryptographically random, nonsequential references across all required objects.
- [ ] Blueprint Course IDs use `BP`, Course Instance IDs use `CI`, Assessment IDs use `A`, and Account IDs use `U`, followed directly by a common cryptographically random Crockford Base32 reference format.
  - Evidence (source): `crates/question_model/src/public_route.rs` and generated/browser route contracts read and use the `BP`, `CI`, `A`, and `U` typed formats; authenticated server paths parse those formats before Store authorization.
  - Mismatch: creation and allocation boundaries have not received connected proof of the common cryptographically random format.
- [ ] ID generation enforces uniqueness and retries random collisions.
  - Mismatch: no connected common human-reference allocator/retry collision proof exists.
- [ ] Give an internal object a human-facing reference ID when a useful workflow needs to display, search, communicate, or support it.
  - Mismatch: Current private route-token inventory needs a workflow-by-workflow audit before a reference is exposed or retained as human-facing.
- [ ] Account `U` references are Sysadmin support references and are not automatically exposed to Students or Instructors.
  - Evidence (source): accepted source review found `U` parsing/use limited to authenticated Account-management paths; no Student or Instructor projection was identified in that review.
  - Mismatch: full cross-route authorization and creation-boundary proof remains outstanding.
- [ ] Published Questions and Question Pools retain their existing public `AAAA-ZBBB` IDs.
  - Mismatch: Published Question IDs use `AAAA-ZBBB`, but published Question Pool identities are not complete.

### Student and FERPA data

- [x] **Student** course data falls under FERPA; treat it as radioactive.
  - Evidence (source): `schemas/base_schema/course_membership.sql` `student_record` is protected by RLS and has no PUBLIC privilege.
- [x] **Student** data should be collected reluctantly, used deliberately, and purged predictably.
  - Evidence (source): `schemas/base_schema/course_roster.sql` `course_roster_profile` contains no duplicate Student email; ordinary roster is email-free and direct-Instructor-only.
  - Evidence (source): `schemas/base_schema/course_retention_transitions.sql` `delete_course_student_records` removes identifiable Course Student records while retaining Account and Course teaching material.
  - Evidence (runtime): `schemas/base_schema/course_retention_transitions.sql` `delete_course_student_records` passed a self-owned disposable PG17 purge-preservation probe on 2026-09-15.
  - Owner: Accounts and roles > Account rules > Student role (first identical Human Guidance occurrence).
- [x] FERPA access should be scoped through exact Course membership and **Student** ownership.
  - Evidence (source): `schemas/base_schema/authorization.sql` `current_session_account_owns_student_record` requires the exact Course, Student Record, authenticated Student Account, and active Student membership before Student Work access is allowed.
  - Evidence (test): `crates/learning-data-access/tests/assessment_access_postgres.rs` `access_reader_projects_one_authoritative_decision_and_effective_policy` uses a real `ple_auth` to `ple_app` session to allow the owner and deny a same-Course other Student, nonmember, same Account with another Course record, and ordinary Sysadmin.
  - Decision: This permanent behavior-level BOLA/FERPA oracle protects a stable high-impact outcome. If its baseline gate fails, this record returns to `[ ]` while the session installation, exact-membership predicate, or ownership boundary is repaired and the same gate rerun.
- [x] **Sysadmins** receive only the FERPA access required for a specific administrative task.
  - Evidence (source): `crates/server/src/support_capability.rs` `support_capability_router` exposes only a scoped, revocable exact-record repair reader; it has no whole-Course roster route.
  - Evidence (test): `tests/e2e/e2e_live_demo_support_capability.sh` `prove_issue` exercises named-record repair issuance, concealment, use, and revocation.
- [x] Student Accounts persist independently of Course data and Course retention.
  - Evidence (source): `schemas/base_schema/accounts.sql` `account` is separate from course-scoped `student_record`.
- [ ] Course work, Attempts, submissions, grades, and other FERPA-sensitive data follow the Course retention policy.
  - Mismatch: No Course retention policy or deletion transition is present to govern these records.
- [ ] Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
  - Mismatch: Student-data deletion and its preservation boundary are not implemented.
- [ ] **Student Work** is the collective term for FERPA-sensitive records created by a Student in a Course Instance.
  - Mismatch: The Attempt table links a Student record and Assessment, but the implementation does not establish `Student Work` as the collective product term for all such records.
- [x] Student Work includes Assessment Attempts, saved Question responses, grading outcomes, and the evidence needed to interpret that work after an Attempt is submitted.
  - Evidence (source): `schemas/base_schema/assessment_attempt_history.sql` `read_student_assessment_attempt_history` joins Attempt, issued question, response, submission, grading, and receipt evidence.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `first_score` verifies retained response finalization and resulting score evidence.
- [ ] Student Work is an umbrella term; the underlying records retain their own identities and purposes.
  - Mismatch: Distinct Attempt, issued-Question, and Question-Pool-selection records show separate identities, but no implemented collective `Student Work` term establishes the required umbrella relationship.
- [ ] Student retention removes identifiable Student evidence, not privacy-safe aggregate Question statistics.
  - Mismatch: Retention deletion is not implemented, so this separation has not been realized.
- [ ] Privacy-safe aggregate Question statistics remain after the underlying Student records are deleted.
  - Mismatch: Aggregate statistics exist, but no implemented Student-record deletion transition proves their retention behavior.
- [ ] Aggregate Question statistics must not identify or allow reconstruction of individual Student activity.
  - Mismatch: `question_revision_statistics` omits direct identity fields, but the implementation has no demonstrated disclosure or small-cohort rule preventing aggregate counts from reconstructing an individual Student's activity.
- [x] Published Question statistics retain accepted graded Attempt count and correct count.
  - Evidence (source): `schemas/base_schema/statistics.sql` `question_revision_statistics` has `accepted_graded_attempt_count` and `correct_count`.
- [x] Eligible Question Types may also retain aggregate answer-choice counts.
  - Evidence (source): `schemas/base_schema/statistics.sql` `selected_count` stores aggregate choice counts.
- [x] Question statistics are version-specific first, with clearly labeled Question-level rollups when appropriate.
  - Evidence (source): `schemas/base_schema/statistics.sql` `question_revision_statistics` primary key is `(question_id, revision_number)`.

### Course retention and lifecycle

- [ ] Course retention should follow Course Instance dates and its six-month Active lifetime rather than
  a fixed academic calendar.
  - Mismatch: Course Instance storage has no six-month Active lifetime or retention deadline.
- [ ] The latest Assessment deadline ends normal teaching and starts the Course Instance's FERPA
  retention clock.
  - Mismatch: Assessment deadlines exist, but no Course FERPA retention clock is derived from them.
- [x] Creating or extending a later Assessment deadline may move those dates, but not beyond the
  six-month Active lifetime.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.save_assessment`, `ple_data.save_assessment_inline`, and `ple_data.save_assessment_policies` lock the Course first, reject a Due date after its immutable `active_until_at`, and invoke `ple_data.synchronize_course_assessment_deadline` after an accepted change.
  - Evidence (source): `schemas/base_schema/assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline` stores the current maximum Assessment Due date and moves the active Course retention anchor to that date, or to `active_until_at` when no Due date remains.
  - Evidence (runtime): accepted independent PostgreSQL 17 actual-API proofs exercised `schemas/base_schema/assessments.sql` `ple_data.save_assessment`, `ple_data.save_assessment_inline`, and `ple_data.save_assessment_policies`, covering release, a cleared last Due date, cap rollback, stale CAS, wrong-Instructor denial, deterministic concurrent saves to two Assessments, an archive race, and frozen archived/deleted retention anchors.
- [ ] Starting the FERPA retention clock does not itself notify, archive, hide, or delete Student data.
  - Mismatch: The FERPA retention-clock transition is absent.
- [ ] The configured FERPA retention policy determines the later notice, archive, recovery, and
  permanent deletion transitions.
  - Mismatch: No configured FERPA retention policy or its transitions exists.
- [ ] PLE warns the **Instructors** before the Course Instance becomes Inactive six months after
  creation.
  - Mismatch: No six-month inactivity transition or Instructor warning exists.
- [x] The six-month Active limit prevents Course reuse or deadline extensions from indefinitely delaying
  FERPA retention and deletion.
  - Evidence (source): `schemas/base_schema/course_core.sql` `ple_data.enforce_course_instance_retention_schedule` derives and preserves the immutable six-month `active_until_at`; `schemas/base_schema/assessments.sql` `ple_data.save_assessment` and its sibling save functions reject every saved Due date beyond that cutoff.
  - Evidence (source): `schemas/base_schema/assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline` bounds the active retention anchor by the accepted current maximum Due date or that immutable cutoff and does not move an archived or deleted anchor.
  - Evidence (runtime): accepted independent PostgreSQL 17 actual-API proofs exercised `schemas/base_schema/assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline`, rejecting over-cap saves without partial state, keeping concurrent current deadlines synchronized, and preserving the retention anchor after archive while later Assessment facts changed.
- [ ] Course inactivity and FERPA deletion are separate transitions; becoming Inactive does not itself
  delete Student records.
  - Mismatch: Neither Course inactivity nor FERPA deletion transition is implemented.
- [ ] Retention should work equally for semesters, quarters, summer Courses, and other academic calendars.
  - Mismatch: No Course retention processing exists for any calendar.
- [ ] PLE should notify the **Instructor** before FERPA-sensitive Student data is archived.
  - Mismatch: No Student-data archiving or associated notification exists.
- [ ] Archived Student data should leave normal Instructor and Student interfaces but remain recoverable during the retention period.
  - Mismatch: No archive/recovery state or interface exclusion exists.
- [ ] FERPA-sensitive Student data should be permanently deleted when its retention period expires.
  - Mismatch: No retention-period expiry deletion exists.
- [ ] Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
  - Mismatch: No Student-data deletion transition exists to establish this preservation behavior.
  - Owner: Data and history > Human-facing reference IDs > Student and FERPA data (first identical Human Guidance occurrence).
- [ ] FERPA retention intervals are operational configuration rather than separate product decisions.
  - Mismatch: No operational FERPA retention interval configuration exists.

### Course retention processing

- [ ] A background process should periodically find Course Instances whose retention deadlines have passed.
  - Mismatch: `worker` only sweeps expired Assignment Attempts, not Course retention deadlines.
- [ ] Retention decisions should come from stored Course dates and the Course Instance creation time.
  - Mismatch: No stored Course retention decision or worker implementation exists.
- [ ] The background process should execute retention policy rather than define when retention periods begin or end.
  - Mismatch: No Course retention processor or policy exists.
- [ ] Running the retention process late should produce the same retention decision as running it on schedule.
  - Mismatch: No Course retention process exists to test schedule independence.
- [ ] The retention process should be safe to run repeatedly.
  - Mismatch: No Course retention process exists to establish repeat safety.

### Common revision and history specifications

- [x] Be conservative about creating revisions.
  - Evidence (source): `schemas/base_schema/question_publication_operations.sql` `ple_private.publish_question_revision` locks the Draft and immediate parent before it creates a successor; its `PQR01` source-checksum comparison rejects an unchanged `question_revision_source_binding` before any successor facts are written.
  - Decision: A fresh one-time PostgreSQL 17 probe exercised `schemas/base_schema/question_publication_operations.sql` `ple_private.publish_question_revision` and verified title and description metadata changes make no Revision, an unchanged source is rejected without a partial write, a changed source creates the next Revision, and two serialized sessions admit only one successor. Tags, subject, and topic have no persisted metadata fields yet; the probe asserts that present absence rather than inventing a field-level behavior. The probe is temporary and will be removed, not cited as permanent evidence.
- [x] Assessments, Course Instances, and Draft Questions use current state.
  - Evidence (source): `schemas/base_schema/assessment_operations.sql` `ple_api.save_assessment` updates current Assessment state and advances its `assessment_edit_number`.
  - Evidence (source): `schemas/base_schema/course_operations.sql` `ple_api.update_course_theme` updates the current `ple_data.course_instance` row.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `ple_private.save_authoring_draft` updates the current `ple_private.draft_question`, metadata, and source-binding rows.
- [ ] Published Questions, Question Pools, and Blueprint Courses have immutable revisions.
  - Mismatch: Published Question and Blueprint revision storage exists, but Question Pools are current Assignment configuration and have no immutable Question Pool Revision.
- [x] Mutable working state uses a monotonic sequential Edit Number when needed for concurrency.
  - Evidence (source): `schemas/base_schema/assessment_operations.sql` `save_assessment_inline` requires and advances `assessment_edit_number`.
- [x] An Edit Number is only a counter and does not identify a stored historical object.
  - Evidence (source): `schemas/base_schema/assessment_operations.sql` `assessment_edit_number` is a current-state concurrency field rather than a revision foreign key.
- [ ] Question, Question Pool, and Blueprint Revision Numbers start at 1 and increase sequentially for
  each object.
  - Mismatch: Question and Blueprint revisions have positive sequential numbers, but Question Pools have no Revision Number.
- [ ] A Revision Number identifies a specific immutable Revision stored by PLE.
  - Mismatch: Question and Blueprint Revision Numbers identify immutable rows, but the absent Question Pool Revision leaves this general Revision Number behavior incomplete.
- [x] A new Revision keeps the same Published Question ID or Question Pool ID.
  - Evidence (source): `schemas/base_schema/question_publication_operations.sql` `ple_private.publish_question_revision` inserts its successor with the existing `p_question_id`; `schemas/base_schema/question_pools.sql` `ple_data.append_question_pool_revision` appends the next revision under its existing `p_question_pool_id`.
- [x] Forking a Published Question or Question Pool creates a new public ID.
  - Evidence (source): `crates/server/src/question_fork.rs` `fork_published_question` issues `forked_question_id` before the fork-to-Draft operation; `schemas/base_schema/question_pools.sql` `ple_data.construct_question_pool_revision_fork` inserts the fork as a new Pool lineage with `p_public_question_pool_id`.
- [x] A fork starts at Revision 1 under its new ID.
  - Evidence (source): `schemas/base_schema/question_publication_operations.sql` `ple_private.publish_new_question_lineage` binds a fork's server-allocated Question ID and inserts its `question_revision` at 1; `schemas/base_schema/question_pools.sql` `ple_data.construct_question_pool_revision_fork` inserts the new Pool and its `question_pool_revision` at 1 while copying the source's exact ordered member Question IDs and Revision Numbers.
- [x] Student Work records the exact Assessment Attempt and Published Question Revision delivered to the Student.
  - Evidence (source): `schemas/base_schema/assessment_attempts.sql` `issued_question` records Attempt identity with `question_id` and `revision_number`.
- [x] Student Work records the Student's responses and the grading outcome returned by the Question Backend.
  - Evidence (source): `schemas/base_schema/assessment_attempt_history.sql` `read_student_assessment_attempt_history` returns retained responses and grading results.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `first_score` verifies the finalization grading outcome.
- [ ] Student Work records the Question Pool Revision and selected Published Question Revision for each response.
  - Mismatch: `question_pool_selected_item` retains the selected Published Question revision but no immutable Question Pool Revision identity.
- [x] Changes to Question point values recalculate scores from the stored grading outcome without changing the outcome.
  - Evidence (source): `schemas/base_schema/grading.sql` `score_recorded_credit` calculates current points from retained `normalized_credit`.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `replay_score` changes points and verifies retained credit is replayed.
- [x] Changes to Assessment settings do not change the recorded history of completed Assessment Attempts.
  - Evidence (source): `schemas/base_schema/assessment_attempt_history.sql` `read_student_assessment_attempt_history` deliberately interprets retained Attempt evidence rather than current Assessment content.
- [x] Immutable Question source and Question assets use SHA-256 checksums where needed to verify their stored contents.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `source_object_checksum` binds immutable Question-source contents to SHA-256 object records.
  - Evidence (source): `schemas/base_schema/question_assets.sql` `public_object_checksum` binds immutable Question-asset contents to SHA-256 object records.

### Dates and time zones

- [x] Assessment deadlines are stored as instants.
  - Evidence (source): `schemas/base_schema/assessments.sql` `assessment.due_at` is `timestamptz`.
- [x] Instructor dates and times use the Instructor's IANA time zone.
  - Evidence (source): `schemas/base_schema/accounts.sql` `account_time_zone_is_exact_iana` validates Instructor account zones against `pg_timezone_names`.
- [x] The Instructor's time zone is used to interpret dates and times the Instructor enters.
  - Evidence (source): `crates/learning-data-access/src/postgres/assessment_release.rs` `resolve_in_account_time_zone` resolves entered release times with the account zone.
- [x] Changing an Instructor's time zone changes how existing deadlines are displayed without changing the deadlines.
  - Evidence (source): `crates/learning-data-access/src/postgres/assessment_release.rs` `LocalDateAndTime::from_activity_timestamp_in_account_time_zone` derives display values from stored timestamps and account zone.
- [x] Assessment deadlines are stored as absolute UTC instants.
  - Evidence (source): `schemas/base_schema/assessments.sql` `assessment.due_at` uses PostgreSQL `timestamptz`.
- [x] Students have their own IANA time zone for displaying dates and times.
  - Evidence (source): `schemas/base_schema/accounts.sql` `account_time_zone_is_exact_iana` validates each Account's exact IANA time-zone preference.
- [x] A Student's time zone defaults to the Instructor's time zone during the invite phase.
  - Evidence (source): `schemas/base_schema/accounts.sql` `apply_student_invitation_time_zone_default` copies the Instructor preference while pending.
- [x] Changing a Student's time zone changes how existing deadlines are displayed without changing the deadlines.
  - Evidence (source): `schemas/base_schema/assessment_attempt_access.sql` `read_student_assessment_access` returns stored deadlines and separately reads `display_time_zone`.
- [x] Changing a display time zone changes how a deadline is shown, not the deadline itself.
  - Evidence (source): `schemas/base_schema/assessment_attempt_access.sql` `read_student_assessment_attempt_context` returns `display_time_zone` separately from `expires_at_millis`.
