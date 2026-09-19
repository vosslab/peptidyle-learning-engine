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
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_owns_student_record` authorizes a Student record only through its Course and active membership.

### Human-facing reference IDs

- [ ] Human-facing reference IDs should be short, opaque, and easy to communicate.
  - Verification pending: current source defines compact opaque forms, but rendered display, entry, copy, and support workflows have not been audited for ease of communication.
- [x] Human-facing reference IDs should not reveal creation order, counts, database keys, ownership, or other object metadata.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `crockford_id_suffix` and `crates/server/src/question_publication.rs` `RandomQuestionIdIssuer` derive public identities from operating-system randomness.
- [x] A public ID is the one universal, canonical human-facing identifier for a PLE object that needs one.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `public_id_reservation` records one canonical value per public object kind, while the owning tables store that value directly.
- [ ] Store and use the exact same public ID in the database, Rust, JSON, URLs, object storage, hashes, logs, and browser UI.
  - Verification pending: SQL, Rust, generated TypeScript, and route contracts use exact canonical values, but object-storage, hash, log, and every browser projection still need a complete inventory.
- [ ] Preserve the canonical ID exactly across system boundaries.
  - Verification pending: typed SQL, Rust, and browser validators are exact, but every transport, persistence, logging, object-storage, and display boundary has not been inventoried.
- [ ] Parsing, serialization, API transport, persistence, and display do not reformat or translate the canonical ID.
  - Verification pending: strict Rust and browser parsing plus canonical SQL storage are implemented; a complete serialization, API, persistence, and display inventory remains pending.
- [x] In ID format notation, `X` denotes a cryptographically random Crockford Base32 character.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `crockford_id_suffix` and `crates/server/src/question_publication.rs` `RandomQuestionIdIssuer` mint each `X` from operating-system randomness.
- [x] In ID format notation, `Z` denotes the calculated checksum character.
  - Evidence (source): `crates/question_model/src/question_library.rs` `public_id_checksum_character` calculates `Z` from the canonical checksum input.
- [x] Both `X` and `Z` represent characters stored as part of the canonical ID.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `is_canonical_prefixed_public_id` and `schemas/base_schema/50_functions/question_lineages.sql` `published_question_id_is_crockford_shape` validate the complete stored values.
- [x] `Z` is not a literal character or separate metadata.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `assign_public_id` appends the calculated checksum directly to the stored canonical ID.
- [x] Public IDs use the Crockford Base32 alphabet `0123456789ABCDEFGHJKMNPQRSTVWXYZ`.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QUESTION_ID_ALPHABET` is the shared public-ID alphabet used by the Rust issuers and generated browser contract.
- [x] Public IDs have one canonical uppercase ASCII form.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionId::from_str` and `crates/question_model/src/public_route.rs` `impl_public_id` reject every noncanonical form.
- [x] Human-entered IDs may use lowercase Crockford characters.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` and `normalizeHumanEnteredPublicId` uppercase only explicit human-entry values before validation.
- [x] Human-entered IDs may use `O` or `o` for `0`.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` and `normalizeHumanEnteredPublicId` map the Crockford `O` alias to `0` before validation.
- [x] Human-entered IDs may use `I`, `i`, `L`, or `l` for `1`.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` and `normalizeHumanEnteredPublicId` map the Crockford `I` and `L` aliases to `1` before validation.
- [x] Normalize human-entered IDs to canonical form, then validate the canonical syntax and checksum at the human-input boundary.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` and `normalizeHumanEnteredPublicId` normalize explicit human entry and then invoke the generated exact validators.
- [ ] Store, transmit, display, copy, and generate only the canonical form.
  - Verification pending: strict generators, model parsers, SQL constraints, and browser validators are implemented, but every storage, transport, display, and copy surface has not been inventoried.
- [x] Calculate the checksum from the ASCII bytes of every other uppercase canonical-ID character.
  - Evidence (source): `crates/question_model/src/question_library.rs` `public_id_checksum_character` hashes caller-supplied canonical ASCII characters after typed constructors exclude separators and checksum positions.
- [x] Include type prefixes in the checksum input.
  - Evidence (source): `crates/question_model/src/public_route.rs` `impl_public_id` builds checksum input from the exact type prefix plus seven random characters.
- [x] Exclude only separators and the checksum position from the checksum input.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionId::from_str` excludes the hyphen and checksum position, while `crates/question_model/src/public_route.rs` `impl_public_id` excludes only final `Z`.
- [x] `XXXX-ZXXX` has checksum input `XXXXXXX`.
  - Evidence (source): `crates/question_model/src/question_library.rs` `impl std::str::FromStr for QuestionId` concatenates the four characters before the hyphen with the three characters after `Z`.
- [x] For `BPXXXXXXXZ`, `CIXXXXXXXZ`, `UXXXXXXXZ`, and `AXXXXXXXZ`, calculate the checksum from every preceding character.
  - Evidence (source): `crates/question_model/src/public_route.rs` `impl_public_id` hashes each literal prefix followed by its seven random Crockford characters.
- [x] Use public unsalted SHA-256 for the checksum.
  - Evidence (source): `crates/question_model/src/question_library.rs` `public_id_checksum_character` applies SHA-256 directly to the canonical checksum input.
- [x] Map the high five bits of SHA-256 digest byte 0 through the Crockford alphabet.
  - Evidence (source): `crates/question_model/src/question_library.rs` `public_id_checksum_character` shifts digest byte 0 by three bits and indexes `QUESTION_ID_ALPHABET`.
- [ ] Validate the public-ID syntax and embedded checksum before database lookup or resolution.
  - Verification pending: typed Rust and browser parsing plus canonical SQL predicates exist, but a complete lookup and resolution call-site inventory remains pending.
- [x] The embedded checksum detects typos.
  - Evidence (test): `crates/question_model/src/public_route.rs` `public_ids_are_exact_checksum_validated_values` accepts canonical vectors and rejects altered checksum characters for every current public-ID family.
- [x] Blueprint Course IDs use `BPXXXXXXXZ`.
  - Evidence (source): `crates/question_model/src/public_route.rs` `BlueprintCourseId` and `schemas/base_schema/50_functions/blueprints.sql` `blueprint_course.blueprint_course_id` enforce the exact form.
- [x] Course Instance IDs use `CIXXXXXXXZ`.
  - Evidence (source): `crates/question_model/src/public_route.rs` `CourseInstanceId` and `schemas/base_schema/50_functions/course_core.sql` `course_instance.course_instance_id` enforce the exact form.
- [x] Assessment IDs use `AXXXXXXXZ`.
  - Evidence (source): `crates/question_model/src/public_route.rs` `AssessmentId` and `schemas/base_schema/50_functions/assessments.sql` `assessment.assessment_id` enforce the exact form.
- [x] Account IDs use `UXXXXXXXZ`.
  - Evidence (source): `crates/question_model/src/public_route.rs` `AccountId` and `schemas/base_schema/50_functions/accounts.sql` `account.account_id` enforce the exact form.
- [x] Each prefixed public ID uses seven cryptographically random Crockford Base32 characters and a final embedded checksum.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `assign_public_id` generates seven random characters, calculates the checksum over prefix plus random identity, and stores the result.
- [x] Each prefixed public-ID random namespace contains 32^7 = 34,359,738,368 values.
  - Evidence (source): `crates/question_model/src/public_route.rs` `PUBLIC_ID_RANDOM_LENGTH` fixes seven random positions over the 32-character `QUESTION_ID_ALPHABET`.
- [x] The checksum adds no identity space.
  - Evidence (source): `crates/question_model/src/public_route.rs` `impl_public_id` derives the checksum deterministically from the prefix and seven-character random identity.
- [x] ID generation enforces global uniqueness across all public IDs and retries random collisions.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `public_id_reservation` provides one global collision boundary, and `assign_public_id` retries the shared `QP001` collision signal.
- [x] Once issued, a public ID permanently identifies that object.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `public_id_reservation_is_permanent` rejects reservation update or deletion.
- [x] Never reuse a public ID for another object, including after deletion or archival.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `public_id_reservation` is an append-only global registry retained independently of object lifecycle state.
- [ ] Give an internal object a human-facing reference ID when a useful workflow needs it.
  - Verification pending: the current public-ID families are explicit, but every internal identity and human-facing workflow has not been audited against the useful-workflow boundary.
- [ ] Useful human-facing ID workflows include display, search, communication, and support.
  - Verification pending: current display, search, communication, and support surfaces need a workflow-by-workflow identity inventory.
- [ ] Other internal objects use native UUID identifiers.
  - Verification pending: many internal records use UUIDs, but the complete internal-identity and route-token inventory remains pending.
- [x] An object with a public ID uses that public ID as its primary key and as the target of every foreign key to it.
  - Evidence (source): `schemas/base_schema/50_functions/course_core.sql` `course_instance.course_instance_id`, `schemas/base_schema/50_functions/assessments.sql` `assessment.assessment_id`, `schemas/base_schema/50_functions/blueprints.sql` `blueprint_course.blueprint_course_id`, `schemas/base_schema/50_functions/accounts.sql` `account.account_id`, and `schemas/base_schema/50_functions/question_pools.sql` `question_pool.question_pool_id` store the public ID as the primary key.
- [ ] Internal UUIDs never substitute for or appear as public identities.
  - Verification pending: owning tables store public IDs as primary keys, but every API, URL, export, log, and browser projection has not been inventoried.
- [ ] Account `U` references are Sysadmin support references.
  - Verification pending: `crates/question_model/src/public_route.rs` `AccountId` exists, but the complete Sysadmin support workflow has not been verified as its sole human-facing use.
- [ ] Account `U` references are not automatically exposed to Students or Instructors.
  - Verification pending: prior source review found no ordinary Student or Instructor projection; full cross-route and browser-output verification remains pending.
- [x] Published Questions and Question Pools use the public `XXXX-ZXXX` format.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `published_question_id_is_crockford_shape` and `schemas/base_schema/50_functions/question_pools.sql` `question_pool.question_pool_id` enforce the same syntax and checksum.
- [x] Published Questions and Question Pools share the same public-ID namespace.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `public_id_reservation` uses one primary key for both `published_question` and `question_pool` reservations.
- [x] An `XXXX-ZXXX` value identifies either a Published Question or a Question Pool, never both.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `reserve_public_id` rejects a second object-kind reservation for an already issued canonical value.

### Content classification

- [ ] PLE uses one shared global content classification vocabulary for **Courses** and **Library
  Objects**.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Every Course has exactly one **Discipline**.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] **Subject**, **Topic**, and **Subtopic** are optional for Courses.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Every Library Object has exactly one **Discipline** and one **Subject**.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] **Topic** and **Subtopic** are optional for Library Objects.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Courses retain the hierarchy because their classification supports Course organization, search,
  filtering, and discovery.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Content classification uses **Discipline** -> **Subject** -> **Topic** -> **Subtopic**.
  - Mismatch: Partial SQL foundation gives Subject-Discipline associations and one-parent Topic/Subtopic relationships, but no complete content classification behavior exists.
- [ ] **Discipline** is the broad academic field, such as Biology, Chemistry, or Mathematics.
  - Mismatch: `content_discipline` exists as an owner-only SQL vocabulary table, but authenticated management and content use remain absent.
- [ ] **Subject** identifies a global area associated with one or more Disciplines, such as Genetics,
  Biochemistry, or Ecology.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] **Topic** identifies a major area within a Subject, such as Enzyme Inhibition or Chromosomal Inheritance.
  - Mismatch: `content_topic.subject_uuid` has a mandatory parent foreign key, but Topic management and content use remain absent.
- [ ] **Subtopic** provides a narrower classification within a Topic, such as Enzyme Catalysis Mechanisms or X-Linked Recessive Crosses.
  - Mismatch: `content_subtopic.topic_uuid` has a mandatory parent foreign key, but Subtopic management and content use remain absent.
- [ ] Subjects have a global identity across PLE, and Subject names are unique across PLE.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] A Subject may be associated with one or more Disciplines.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] A Topic belongs to one Subject.
  - Mismatch: The SQL foreign key enforces one Topic parent, but authenticated management and complete product behavior remain open.
- [ ] A Subtopic belongs to one Topic.
  - Mismatch: The SQL foreign key enforces one Subtopic parent, but authenticated management and complete product behavior remain open.
- [ ] Course and Library Object selections follow the hierarchy: the Subject is associated with the
  selected Discipline, the Topic belongs to that Subject, and the Subtopic belongs to that Topic.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Courses and Library Objects select from the same shared global vocabulary.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] **Sysadmins** exclusively create and manage the Discipline vocabulary and its lifecycle.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Discipline is a stable vocabulary expected to change infrequently.
  - Mismatch: `content_discipline` is a bounded foundation table, but no managed lifecycle establishes its stable vocabulary behavior.
- [ ] **Instructors** classify content by selecting from the Sysadmin-managed Disciplines.
  - Mismatch: No Instructor reader, selector, or content attachment exists.
- [ ] **Instructors** may create new Subjects within a selected Discipline.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] When an Instructor attempts to create a Subject whose globally unique name already exists, PLE
  offers the existing Subject.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] PLE requires explicit Instructor acceptance before associating the existing Subject with the
  selected Discipline.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Creating or selecting vocabulary should fit naturally into the classification workflow.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] **Instructors** may create new Topics within a Subject.
  - Mismatch: No Instructor Topic writer exists.
- [ ] **Instructors** may create new Subtopics within a Topic.
  - Mismatch: No Instructor Subtopic writer exists.
- [ ] Classification selection, browsing, and filtering begin with Discipline.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Course and Library Object classification follow Discipline -> Subject -> Topic -> Subtopic,
  progressively narrowing the available choices at each level.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Selecting a Discipline limits Subject choices to Subjects associated with that Discipline.
  - Mismatch: `content_subject_discipline` stores associations, but no authenticated selector limits Subject choices.
- [ ] After selecting a Subject, search interfaces may offer an explicit option to include content
  associated with that Subject across its other Disciplines.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] **Tags** provide flexible labels outside the Discipline, Subject, Topic, and Subtopic hierarchy.
  - Mismatch: Tag storage and content use are not implemented by this vocabulary foundation.
- [ ] Courses and Library Objects may have any number of Tags, including none.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Classification supports searching, filtering, sorting, organization, and discovery wherever those capabilities are useful.
  - Mismatch: No classification reader, query, or product discovery behavior exists.
- [ ] Subject, Topic, and Subtopic names must satisfy length limits and formatting requirements.
  - Mismatch: The SQL tables bound and reject untrimmed/control-character names, but trusted strip-before-validate writers are absent.
- [ ] Length allowances increase from Subject to Topic to Subtopic, supporting more specific names as classification becomes narrower.
  - Mismatch: SQL bounds increase from Subject to Topic to Subtopic, but authenticated vocabulary management remains absent.
- [ ] Strip leading and trailing whitespace from Subject, Topic, and Subtopic names and validate the resulting names consistently.
  - Mismatch: The SQL tables reject untrimmed storage; no writer strips input before validation.

### Student and FERPA data

- [x] **Student** course data falls under FERPA; treat it as radioactive.
  - Evidence (source): `schemas/base_schema/20_tables/course_membership.sql` `student_record` is protected by RLS and has no PUBLIC privilege.
- [x] **Student** data should be collected reluctantly, used deliberately, and purged predictably.
  - Evidence (source): `schemas/base_schema/50_functions/course_roster.sql` `course_roster_profile` contains no duplicate Student email; ordinary roster is email-free and direct-Instructor-only.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records` removes identifiable Course Student records while retaining Account and Course teaching material.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records` passed a self-owned disposable PG17 purge-preservation probe on 2026-09-15.
  - Owner: 02_accounts.md / Student role (first occurrence; identical requirement and status).
- [x] FERPA access should be scoped through exact Course membership and **Student** ownership.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_owns_student_record` requires the exact Course, Student Record, authenticated Student Account, and active Student membership before Student Work access is allowed.
  - Evidence (test): `crates/learning-data-access/tests/assessment_access_postgres.rs` `access_reader_projects_one_authoritative_decision_and_effective_policy` uses a real `ple_auth` to `ple_app` session to allow the owner and deny a same-Course other Student, nonmember, same Account with another Course record, and ordinary Sysadmin.
  - Decision: This permanent behavior-level BOLA/FERPA oracle protects a stable high-impact outcome. If its baseline gate fails, this record returns to `[ ]` while the session installation, exact-membership predicate, or ownership boundary is repaired and the same gate rerun.
- [x] **Sysadmins** receive only the FERPA access required for a specific administrative task.
  - Evidence (source): `crates/server/src/support_capability.rs` `support_capability_router` exposes only a scoped, revocable exact-record repair reader; it has no whole-Course roster route.
  - Evidence (test): `tests/e2e/e2e_live_demo_support_capability.sh` `prove_issue` exercises named-record repair issuance, concealment, use, and revocation.
- [x] Student Accounts persist independently of Course data and Course retention.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `account` is separate from course-scoped `student_record`.
- [x] Course work, Attempts, submissions, grades, and other FERPA-sensitive data follow the Course retention policy.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` deletes private Attempt roots and Course-scoped Student records only after the archived state, while retaining Course teaching material and identity-free aggregate rows.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` passed a fresh PostgreSQL 17 actual-role proof with a released Assessment, issued Question, saved response, whole-Assessment submission, `question_response`, grading result, grading receipt, and aggregate; deletion removed the identifiable Student Work descendants at the stored expiry.
- [x] Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` deletes Course-scoped Student records and private Student evidence without deleting Course, Assessment, Question, or configuration relations.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` passed a fresh PostgreSQL 17 actual-role proof retaining the global Student Account, Course, Assessment, Published Question, immutable source, settings, and an unrelated Course membership after the populated Student Work was deleted.
- [ ] **Student Work** is the collective term for FERPA-sensitive records created by a Student in a Course Instance.
  - Mismatch: The Attempt table links a Student record and Assessment, but the implementation does not establish `Student Work` as the collective product term for all such records.
- [x] Student Work includes Assessment Attempts, saved Question responses, grading outcomes, and the evidence needed to interpret that work after an Attempt is submitted.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_assessment_attempt_history` joins Attempt, issued question, response, submission, grading, and receipt evidence.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `first_score` verifies retained response finalization and resulting score evidence.
- [ ] Student Work is an umbrella term; the underlying records retain their own identities and purposes.
  - Mismatch: Distinct Attempt, issued-Question, and Question-Pool-selection records show separate identities, but no implemented collective `Student Work` term establishes the required umbrella relationship.
- [x] Student retention removes identifiable Student evidence, not privacy-safe aggregate Question statistics.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` preserves existing identity-free aggregate rows while deleting private Attempt roots and Course Student records; `schemas/base_schema/20_tables/statistics.sql` defines the retained Question Revision count tables without Student identity fields.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` passed a fresh PostgreSQL 17 actual-role proof deleting populated identifiable Student Work while retaining its existing anonymous Question Revision aggregate.
- [x] Privacy-safe aggregate Question statistics remain after the underlying Student records are deleted.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` explicitly preserves existing identity-free aggregate rows, and `schemas/base_schema/20_tables/statistics.sql` retains Question Revision statistics independently of private grading receipts.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` passed a fresh PostgreSQL 17 actual-role proof retaining the accepted-graded and correct counts after their submitted Student Work and private receipts were deleted.
- [ ] Aggregate Question statistics must not identify or allow reconstruction of individual Student activity.
  - Mismatch: `question_revision_statistics` omits direct identity fields, but the implementation has no demonstrated disclosure or small-cohort rule preventing aggregate counts from reconstructing an individual Student's activity.
- [x] Published Question statistics retain accepted graded Attempt count and correct count.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `question_revision_statistics` has `accepted_graded_attempt_count` and `correct_count`.
- [x] Eligible Question Types may also retain aggregate answer-choice counts.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `selected_count` stores aggregate choice counts.
- [x] Question statistics are version-specific first, with clearly labeled Question-level rollups when appropriate.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `question_revision_statistics` primary key is `(question_id, revision_number)`.

### Course retention and lifecycle

- [ ] Course retention should follow Course Instance dates and its six-month Active lifetime rather than
  a fixed academic calendar.
  - Mismatch: Course Instance storage has no six-month Active lifetime or retention deadline.
- [ ] The latest Assessment deadline ends normal teaching and starts the Course Instance's FERPA
  retention clock.
  - Mismatch: Assessment deadlines exist, but no Course FERPA retention clock is derived from them.
- [x] Creating or extending a later Assessment deadline may move those dates, but not beyond the
  six-month Active lifetime.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `ple_data.save_assessment`, `ple_data.save_assessment_inline`, and `ple_data.save_assessment_policies` lock the Course first, reject a Due date after its immutable `active_until_at`, and invoke `ple_data.synchronize_course_assessment_deadline` after an accepted change.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline` stores the current maximum Assessment Due date and moves the active Course retention anchor to that date, or to `active_until_at` when no Due date remains.
  - Evidence (runtime): accepted independent PostgreSQL 17 actual-API proofs exercised `schemas/base_schema/50_functions/assessments.sql` `ple_data.save_assessment`, `ple_data.save_assessment_inline`, and `ple_data.save_assessment_policies`, covering release, a cleared last Due date, cap rollback, stale CAS, wrong-Instructor denial, deterministic concurrent saves to two Assessments, an archive race, and frozen archived/deleted retention anchors.
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
  - Evidence (source): `schemas/base_schema/50_functions/course_core.sql` `ple_data.enforce_course_instance_retention_schedule` derives and preserves the immutable six-month `active_until_at`; `schemas/base_schema/50_functions/assessments.sql` `ple_data.save_assessment` and its sibling save functions reject every saved Due date beyond that cutoff.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline` bounds the active retention anchor by the accepted current maximum Due date or that immutable cutoff and does not move an archived or deleted anchor.
  - Evidence (runtime): accepted independent PostgreSQL 17 actual-API proofs exercised `schemas/base_schema/50_functions/assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline`, rejecting over-cap saves without partial state, keeping concurrent current deadlines synchronized, and preserving the retention anchor after archive while later Assessment facts changed.
- [ ] Course inactivity and FERPA deletion are separate transitions; becoming Inactive does not itself
  delete Student records.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` separates `ple_api.archive_course_student_records` from `ple_api.delete_course_student_records`; the latter requires the archived state and performs the deletion transaction.
  - Verification pending: the 2026-09-16 actual-role PostgreSQL 17 gate exercised archive then bounded deletion, but notification, configured intervals, worker scheduling, and connected interface behavior remain open.
- [ ] Retention should work equally for semesters, quarters, summer Courses, and other academic calendars.
  - Mismatch: No Course retention processing exists for any calendar.
- [ ] PLE should notify the **Instructor** before FERPA-sensitive Student data is archived.
  - Mismatch: No Student-data archiving or associated notification exists.
- [ ] Archived Student data should leave normal Instructor and Student interfaces but remain recoverable during the retention period.
  - Mismatch: No archive/recovery state or interface exclusion exists.
- [x] FERPA-sensitive Student data should be permanently deleted when its retention period expires.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` is an archived-Course deletion transition and is repeat-safe after a deleted state.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` refused a premature transition, deleted populated Student Work at the stored due time, returned false on repeat, and serialized two concurrent executors as one true transition followed by one false reread in fresh PostgreSQL 17 actual-role proof.
- [x] Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` deletes Course-scoped Student records and private Student evidence without deleting Course, Assessment, Question, or configuration relations.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` passed a fresh PostgreSQL 17 actual-role proof retaining the global Student Account, Course, Assessment, Published Question, immutable source, settings, and an unrelated Course membership after the populated Student Work was deleted.
  - Owner: 06_data.md / Student and FERPA data (first occurrence; identical requirement and status).
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
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `ple_private.publish_question_revision` locks the Draft and immediate parent before it creates a successor; its `PQR01` source-checksum comparison rejects an unchanged `question_revision_source_binding` before any successor facts are written.
  - Decision: A fresh one-time PostgreSQL 17 probe exercised `schemas/base_schema/50_functions/question_publication_operations.sql` `ple_private.publish_question_revision` and verified title and description metadata changes make no Revision, an unchanged source is rejected without a partial write, a changed source creates the next Revision, and two serialized sessions admit only one successor. Tags, subject, and topic have no persisted metadata fields yet; the probe asserts that present absence rather than inventing a field-level behavior. The probe is temporary and will be removed, not cited as permanent evidence.
- [x] Assessments, Course Instances, and Draft Questions use current state.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_operations.sql` `ple_api.save_assessment` updates current Assessment state and advances its `assessment_edit_number`.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `ple_api.update_course_theme` updates the current `ple_data.course_instance` row.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `ple_private.save_authoring_draft` updates the current `ple_private.draft_question`, metadata, and source-binding rows.
- [ ] Published Questions, Question Pools, and Blueprint Courses have immutable revisions.
  - Mismatch: Published Question and Blueprint revision storage exists, but Question Pools are current Assignment configuration and have no immutable Question Pool Revision.
- [x] Mutable working state uses a monotonic sequential Edit Number when needed for concurrency.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_operations.sql` `save_assessment_inline` requires and advances `assessment_edit_number`.
- [x] An Edit Number is only a counter and does not identify a stored historical object.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_operations.sql` `assessment_edit_number` is a current-state concurrency field rather than a revision foreign key.
- [ ] Question, Question Pool, and Blueprint Revision Numbers start at 1 and increase sequentially for
  each object.
  - Mismatch: Question and Blueprint revisions have positive sequential numbers, but Question Pools have no Revision Number.
- [ ] A Revision Number identifies a specific immutable Revision stored by PLE.
  - Mismatch: Question and Blueprint Revision Numbers identify immutable rows, but the absent Question Pool Revision leaves this general Revision Number behavior incomplete.
- [x] A new Revision keeps the same Published Question ID or Question Pool ID.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `ple_private.publish_question_revision` inserts its successor with the existing `p_question_id`; `schemas/base_schema/50_functions/question_pools.sql` `ple_data.append_question_pool_pin` appends the next revision under its existing `p_question_pool_id`.
- [x] Forking a Published Question or Question Pool creates a new public ID.
  - Evidence (source): `crates/server/src/question_fork.rs` `fork_published_question` issues `forked_question_id` before the fork-to-Draft operation; `schemas/base_schema/50_functions/question_pools.sql` `ple_data.construct_question_pool_pin_fork` inserts the fork as a new Pool lineage with `p_public_question_pool_id`.
- [x] A fork starts at Revision 1 under its new ID.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `ple_private.publish_new_question_lineage` binds a fork's server-allocated Question ID and inserts its `question_revision` at 1; `schemas/base_schema/50_functions/question_pools.sql` `ple_data.construct_question_pool_pin_fork` inserts the new Pool and its `question_pool_pin` at 1 while copying the source's exact ordered member Question IDs and Revision Numbers.
- [x] Student Work records the exact Assessment Attempt and Published Question Revision delivered to the Student.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempts.sql` `issued_question` records Attempt identity with `question_id` and `revision_number`.
- [x] Student Work records the Student's responses and the grading outcome returned by the Question Backend.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_assessment_attempt_history` returns retained responses and grading results.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `first_score` verifies the finalization grading outcome.
- [ ] Student Work records the Question Pool Revision and selected Published Question Revision for each response.
  - Mismatch: `question_pool_selected_item` retains the selected Published Question revision but no immutable Question Pool Revision identity.
- [x] Changes to Question point values recalculate scores from the stored grading outcome without changing the outcome.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `score_recorded_credit` calculates current points from retained `normalized_credit`.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `replay_score` changes points and verifies retained credit is replayed.
- [x] Changes to Assessment settings do not change the recorded history of completed Assessment Attempts.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_assessment_attempt_history` deliberately interprets retained Attempt evidence rather than current Assessment content.
- [x] Immutable Question source and Question assets use SHA-256 checksums where needed to verify their stored contents.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `source_object_checksum` binds immutable Question-source contents to SHA-256 object records.
  - Evidence (source): `schemas/base_schema/20_tables/question_assets.sql` `public_object_checksum` binds immutable Question-asset contents to SHA-256 object records.
- [x] A public-ID checksum is one embedded character derived from other ID characters.
  - Evidence (source): `crates/question_model/src/question_library.rs` `public_id_checksum_character` derives one Crockford character from the canonical ID characters.
- [x] A stored-content checksum is a full SHA-256 value verifying exact bytes.
  - Evidence (source): `crates/question_model/src/student_work/source_object_checksum.rs` `SourceObjectChecksum` accepts exactly one 64-character lowercase SHA-256 hexadecimal value.
- [x] Public-ID checksums and stored-content checksums are not interchangeable.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionId` embeds one Crockford checksum character, while `crates/question_model/src/student_work/source_object_checksum.rs` `SourceObjectChecksum` is a separate full-digest type with incompatible validation.

### Dates and time zones

- [x] Assessment deadlines are stored as instants.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `assessment.due_at` is `timestamptz`.
- [x] Instructor dates and times use the Instructor's IANA time zone.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `account_time_zone_is_exact_iana` validates Instructor account zones against `pg_timezone_names`.
- [x] The Instructor's time zone is used to interpret dates and times the Instructor enters.
  - Evidence (source): `crates/learning-data-access/src/postgres/assessment_release.rs` `resolve_in_account_time_zone` resolves entered release times with the account zone.
- [x] Changing an Instructor's time zone changes how existing deadlines are displayed without changing the deadlines.
  - Evidence (source): `crates/learning-data-access/src/postgres/assessment_release.rs` `LocalDateAndTime::from_activity_timestamp_in_account_time_zone` derives display values from stored timestamps and account zone.
- [x] Assessment deadlines are stored as absolute UTC instants.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `assessment.due_at` uses PostgreSQL `timestamptz`.
- [x] Students have their own IANA time zone for displaying dates and times.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `account_time_zone_is_exact_iana` validates each Account's exact IANA time-zone preference.
- [x] A Student's time zone defaults to the Instructor's time zone during the invite phase.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `apply_student_invitation_time_zone_default` copies the Instructor preference while pending.
- [x] Changing a Student's time zone changes how existing deadlines are displayed without changing the deadlines.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_access.sql` `read_student_assessment_access` returns stored deadlines and separately reads `display_time_zone`.
- [x] Changing a display time zone changes how a deadline is shown, not the deadline itself.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_access.sql` `read_student_assessment_attempt_context` returns `display_time_zone` separately from `expires_at_millis`.
