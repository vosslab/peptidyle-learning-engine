## Course specifications

- [ ] **Courses** organize reusable teaching content and its delivery to **Students**.
  - Mismatch: The current Course model does not establish the complete stated product boundary.
- [ ] PLE has two Course forms: **Blueprint Courses** and **Course Instances**.
  - Mismatch: The current paths implement related records but do not verify the complete product distinction.
- [ ] **Blueprint Courses** provide reusable course designs for creating Course Instances.
  - Mismatch: Adoption is implemented only for the current stored Blueprint shape.
- [x] Course Instances may be created from a Blueprint Course or started empty.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CourseInstanceCreationSource` and `src/api/decoders/course_instance.ts` `decodeCreateCourseInstanceInput` accept strict Empty or exact Adopted source forms.
  - Evidence (runtime): `src/pages/course_list_page.tsx` `TeachingCourseListPage` was exercised against the actual server in bounded exact-main browser proof: Empty creation persisted without Blueprint-list requests; separate Public Blueprint exact-Revision adoption created a daughter Course and Unreleased Practice Assessment. Successful API responses were `no-store`. This creation-only row does not establish direct started-empty Assessment authoring or the full teaching lifecycle.
- [ ] A Course can have multiple co-**Instructors** with equal teaching authority.
  - Mismatch: The schema has an assigned Instructor distinction, not verified equal co-Instructor authority.
- [ ] **Sysadmins** can create Courses, but **Instructors** teach them.
  - Mismatch: Sysadmin creation selection exists, but role behavior is not fully verified.
- [x] Every Course Instance must have at least one assigned **Instructor**.
  - Evidence (source): `schemas/base_schema/course_membership.sql` `assert_assigned_instructor_membership` rejects a Course Instance without a current assigned Instructor membership.
- [ ] Creating a Course Instance establishes its first Instructor membership but does not give that Instructor greater Course authority than later co-Instructors.
  - Mismatch: `CourseInstanceView.is_assigned_instructor` exposes a special authority distinction.

### Blueprint Course specifications

- [x] **Blueprint Courses** are reusable course definitions for building **Course Instances**.
  - Evidence (source): `crates/learning-data-access/src/blueprint_course.rs` `BlueprintCourseStore` persists reusable Blueprint content and revisions.
- N/A Blueprint Courses are a similar concept as LibreTexts' ADAPT alpha courses.
  - Reason: This comparison provides human-oriented product context, not an implemented PLE behavior.
- [x] Blueprint Courses have no **Students**, deadlines, or other teaching-specific delivery settings.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_course_revision` stores reusable content without Course Instance delivery fields.
- [x] Blueprint Courses do not contain dates or relative schedules.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_course_revision` has no date or schedule columns.
- [ ] Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` lists Public Blueprints to active Instructors.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle and actual HTTP receipts against the full vetted-Instructor visibility and reusability claim.
- [ ] Blueprint Courses contain only **Published Questions** and published **Question Pools**.
  - Mismatch: Current pin validation covers Question revisions but not the required published Pool behavior.
- [ ] An **Instructor** may deliberately publish an existing Course Instance structure as a new Blueprint Course.
  - Mismatch: No Course Instance-to-Blueprint publishing route or store operation was found.

#### Blueprint Course lifecycle specifications

- [ ] Blueprint Courses have three lifecycle states: **Private**, **Public**, and **Archived**.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_course.availability` permits `private`, `public`, and `archived`.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle receipt against the complete state claim.
- [ ] New Blueprint Courses and forks start Private.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.create_blueprint_course` and `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` create Private lineages.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle receipt against both creation paths.
- [ ] Private Blueprint Courses are visible only to their owning **Instructor**.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` and `ple_api.load_blueprint_course` limit Private access to the owner.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle and actual HTTP receipts against the owner-only claim.
- [x] Instructors may develop and use Private Blueprint Courses without publishing them.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_model.ts` `blueprintLifecyclePresentation` permits the owner to edit a Private Blueprint and withholds adoption; Private is deliberately not a daughter-Course source.
- [ ] Private Blueprint Courses cannot be adopted to create daughter **Course Instances**.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.load_course_instance_blueprint` requires Public availability.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle receipt against the Private-adoption denial.
- [x] Making a Blueprint Course Public adds it to the shared Blueprint Course collection.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.set_blueprint_availability` publishes owner content and `ple_api.list_blueprint_courses` includes Public Blueprints for active Instructors.
- [x] Public Blueprint Courses and their Revision history are visible to all vetted **Instructors**.
  - Evidence (runtime): `schemas/base_schema/blueprint_history.sql` `ple_api.list_blueprint_history` uses ordinary visibility for Revision and metadata facts. Accepted Public-history proof is `/private/tmp/ple-blueprint-owned-pool-artifacts.sEJZUB/history-proof.json`.
- [ ] Public Blueprint Courses can be adopted to create daughter Course Instances.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.load_course_instance_blueprint` requires Public availability for adoption.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle and actual HTTP receipts against the complete Public-adoption workflow.
- [x] A Public Blueprint Course with no adoptions may return to Private.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.set_blueprint_availability` permits this transition only before a daughter Course Instance exists; the accepted lifecycle runtime contract covers the rule.
- [x] A Public Blueprint Course with one or more adoptions remains Public.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.set_blueprint_availability` rejects Public-to-Private after an adoption; the accepted lifecycle runtime contract exercises the denial.
- [x] Blueprint Courses have no separate Draft state.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `CHECK (availability IN ('private', 'public', 'archived'))` defines the complete Blueprint availability state.

#### Archived Blueprint Course specifications

- [x] Archived Blueprint Courses are read-only and no longer actively maintained.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.save_blueprint_course` and `ple_api.rename_blueprint_course` lock the owner-visible Blueprint and reject `archived` before replay, CAS, or no-op handling.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` covers denied replay, no-op, changed Save, and rename without changing the Blueprint state, then restored writes.
  - Evidence (runtime): `schemas/base_schema/blueprint_operations.sql` `ple_api.save_blueprint_course`; `/private/tmp/ple-daughter-revision-notice-artifacts.JhV6aj/archived-blueprint-http-proof.json` records five `409` denials with unchanged state and preserved Private/Public/restored writes.
- [x] Archived Blueprint Courses and their Revision history remain visible to all vetted **Instructors**.
  - Evidence (runtime): `schemas/base_schema/blueprint_history.sql` `ple_api.list_blueprint_history` uses ordinary visibility for Archived Revision and metadata facts. Accepted Archived-history and discovery proof is `/private/tmp/ple-blueprint-owned-pool-artifacts.sEJZUB/history-proof.json` and `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-http-proof.json`.
- [x] Archived Blueprint Courses do not appear in normal discovery unless explicitly included.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` filters Public, owning Private, and only explicitly requested Archived records; `crates/server/src/blueprint_course.rs` `BlueprintCourseListQuery` accepts only the typed `includeArchived` boolean.
  - Evidence (runtime): `src/features/blueprint_course/blueprint_course_workspace.tsx` `changeIncludeArchived`; `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-browser-proof.json` records the actual compiled-main default-off, Include Archived, read-only Archived-detail, and return-to-off workflow with eight GETs and zero writes. Its companion HTTP receipt records default/false/true membership and strict invalid-query `400` results.
- [x] Archived Blueprint Courses cannot be adopted to create new daughter Course Instances.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.load_course_instance_blueprint` requires Blueprint availability `public` for exact-Revision adoption.
- [ ] Archived Blueprint Courses can be forked.
  - Evidence (source): `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` accepts Public or Archived sources, while adoption requires Public availability.
  - Mismatch: `src/api/blueprint_course.ts` has no Instructor fork client method, and `BlueprintCourseLifecycleControls` has no fork action.
- [ ] Forking an Archived Blueprint Course creates a new Private Blueprint Course.
  - Evidence (source): `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` accepts Archived sources and creates a Private child owned by the actor.
  - Mismatch: `src/api/blueprint_course.ts` has no Instructor fork client method, and `BlueprintCourseLifecycleControls` has no fork action.
- [ ] The owning **Instructor** can return an Archived Blueprint Course to Public.
  - Evidence (source): `crates/learning-data-access/src/postgres/blueprint_course.rs` `restore_blueprint` sets availability to `public`.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle and actual HTTP receipts against restore followed by adoption.
- [x] Blueprint Course visibility includes its content, Revision history, and recorded changes.
  - Evidence (runtime): `schemas/base_schema/blueprint_history.sql` `ple_api.list_blueprint_history` provides separate, ordinary-visibility Revision and metadata-event pages; `src/features/blueprint_course/blueprint_history.tsx` `BlueprintHistory` presents both read-only. Accepted bounded proof is `/private/tmp/ple-blueprint-owned-pool-artifacts.sEJZUB/history-proof.json`.
- [ ] Visibility does not grant editing authority.
  - Verification pending: prior C883 owner/nonowner Apply denials are contributor evidence; complete owner mutation boundaries and connected workflow need current-authority verification.

#### Blueprint Course revision specifications

- [x] Blueprint Courses use immutable **Blueprint Revisions** for saved reusable content.
  - Evidence (source): `schemas/base_schema/blueprint_revision_integrity.sql` `blueprint_course_revision_is_immutable` rejects Revision updates and deletes.
- [x] Blueprint Course content editing uses explicit Save.
  - Evidence (source): `crates/server/src/blueprint_course.rs` `save_blueprint` is the explicit content-save route handler.
- [x] Saving changed Blueprint content creates the next Blueprint Revision.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.save_blueprint_course` inserts the next `blueprint_course_revision` when `changed` is true.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` asserts one changed Save creates one new Revision.
- [x] Multiple content edits before Save become one Blueprint Revision.
  - Evidence (source): `crates/question_model/src/blueprint_course/blueprint_children.rs` `ReplaceBlueprintCourseContentInput` carries one complete replacement tree per Save.
- [x] Saving unchanged Blueprint content does not create another Revision.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.save_blueprint_course` returns the expected Revision without inserting when `changed` is false.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` asserts a canonical no-op Save returns Revision 2 with `changed` false.
- [x] Blueprint Course metadata can change without creating a Blueprint Revision.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.rename_blueprint_course` updates `blueprint_course` metadata without inserting a `blueprint_course_revision`.
- [x] Blueprint Course names are metadata and identify the Blueprint across Revisions.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_course` owns names while `blueprint_course_revision` keys content by course reference and revision.
- [x] Changing a Blueprint Course name does not create a new Blueprint Revision.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.rename_blueprint_course` updates names and metadata ETag without inserting a `blueprint_course_revision`.

#### Blueprint Course stewardship specifications

- [ ] **Instructors** can Star or Watch Public and Archived Blueprint Courses.
  - Mismatch: No Blueprint Star or Watch model, route, or store operation was found.
- [ ] A Star is a visible endorsement and helps **Instructors** save useful Blueprint Courses.
  - Mismatch: No Star model or presentation was found.
- [ ] Vetted **Instructors** can see who Starred a Blueprint Course and its Star count.
  - Mismatch: No Star model or presentation was found.
- [ ] Watching a Blueprint Course is private.
  - Mismatch: No Watch model or presentation was found.
  - Mismatch: source-bound Watch events exist for revisions and forks, but improvement threads and impact notices have no product-defined model or private delivery behavior.
  - Question: For improvement threads, who may create/read/reply/edit/resolve them, which identity/attachments/linkage/notification/retention rules apply; and for impact notices, who may create them, under what condition, with what text/category/severity/manual-or-derived/linkage/audience/update/cancel rules?
- [ ] Watchers are notified about new Blueprint Revisions and other important Blueprint changes.
  - Mismatch: No Watch or notification implementation was found.
- [ ] Forking or adopting a Blueprint Course does not automatically Star or Watch it.
  - Mismatch: No Star, Watch, or fork implementation exists to verify this invariant.
- [ ] Stars and Watches belong to the Blueprint Course across all of its Revisions.
  - Mismatch: No Star or Watch persistence exists.

#### Blueprint adoption and incorporation specifications

- [x] Blueprint adoption copies every Assessment from the Blueprint Course into the Course Instance.
  - Evidence (source): `crates/learning-data-access/src/postgres/course_instance.rs` `create_course_instance` obtains `creation_assignments` before atomic creation.
- [x] Course Instances pin the exact Blueprint Revision from which they were adopted.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CourseInstanceCreationSource` requires an exact immutable Blueprint Revision source for adoption.
- [x] New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` lazily obtains the authorized current-parent Course summary and offers each adopted Assessment for review; `src/api/assessment_release.ts` `CourseBlueprintUpdateReview` excludes direct local Assessments and carries matching, removed-source, Type-mismatch, changed, and automatically-added correspondences.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in actual-server and compiled-main proof: each Course-summary read returned five coherent rows (changed, matching, removed, Type mismatch, automatically added) after lazy open/reopen at 1280 by 900 and 390 by 844. The changed Assessment then reviewed and applied with exact source Revision 2 and daughter Edit CAS; the Course refresh showed the applied match. Student and unrelated reads returned `404 no-store`; a private parent was concealed from another Instructor in the privileged-availability fixture; Archived review remained available and new adoption was denied. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
- [x] Routine Blueprint changes should be quick for an **Instructor** to review and incorporate.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` supplies one Course-level Review action, clear per-Assessment status labels, Refresh, and links to the existing Assessment detail Review/Apply workflow.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in compiled-main browser proof at 1280 by 900 and 390 by 844: lazy open/reopen GET behavior produced the five-row Course summary and the Course-to-Assessment detail review. Cancel issued zero POST requests; Apply used exact source Revision 2 plus daughter Edit CAS and a returning Course refresh showed the match. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
- [x] It should be obvious when a Course Instance is based on an older Blueprint Revision.
  - Evidence (source): `schemas/base_schema/course_operations.sql` `ple_api.load_course_instance`, `crates/learning-data-access/src/postgres/course_instance.rs` `decode_view`, `src/api/decoders/course_instance.ts` `decodeCourseInstanceView`, and `src/pages/course_instance_page.tsx` `CourseInstancePage` carry the adopted and current Revision numbers and render the older-Revision notice with strict `bigint` comparisons.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` was exercised by accepted independent actual-server/exact-main browser proof across empty, current, newer, and explicit synthetic Private-origin states. The newer state visibly showed its original adopted Revision and the current newer Revision; Student and unrelated-Instructor reads returned nonenumerating `404 no-store`, no extra Blueprint fetch or write occurred, and browser errors were empty. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.u1qUyY`.
  - Decision: This read-only notice makes a stale daughter obvious. It does not offer, review, approve, or apply a Blueprint update.
- [ ] The **Instructor** decides which changes to existing Assessments to incorporate.
  - Verification pending: source-audit this changed requirement against its current parent section and the existing implementation; no full current-scope proof is claimed by the prior wording.
- [x] Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` validates the new-reference delta and inserts only new Assessments; it does not update existing daughter Assessments.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` changes a retained source Assessment title and proves existing daughter Assessment content, entries, and actual Student Work unchanged through Save/replay/no-op/stale operations. Accepted artifact: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Decision: This negative invariant remains separate from the verified Course-review workflow; it does not claim direct-Assessments or all Student Work.
- [x] Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Evidence (source): `crates/learning-data-access/src/postgres/blueprint_course.rs` Save and `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` atomically append only newly added Assessments to daughters with fresh Course-owned Pool identities and unset dates.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` passed connected PostgreSQL 17 proof, preserving exact pins/settings, existing Assessment content and actual Student Work, and the original adoption Revision pin across two daughters including an inactive Course; an unrelated empty Course remained unchanged. Replay/no-op/stale saves made no duplicate append. Artifact: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8` also accepted temporary bad-payload rollback proof. Existing connected adoption lifecycle regression passed 1 test with 0 ignored: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Decision: Only automatic-new Assessment propagation is verified, not C410 existing-Assessment update offers or the whole Course milestone.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` now invokes the `append.rs` helper `assert_new_assessment_save_preserves_daughter_work`; the existing permanent lifecycle regression passed 1 test with 0 ignored in `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. No separate seed-sharing test was retained.

#### Blueprint Course fork specifications

- [x] An **Instructor** can fork a Public or Archived **Blueprint Course** to create a new Private Blueprint Course.
  - Evidence (source): `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` accepts one exact Public or Archived source Revision and delegates the actor-owned Private child to the lineage Store.
  - Evidence (runtime): `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` is exercised by accepted actual HTTP evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-lineage-pair-http-proof.json` and compiled-main browser evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.wVCQ4m/comparison-browser.json` that create and open a Private fork.
- [x] A fork is owned by the **Instructor** who created it.
  - Evidence (source): `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` derives the actor from the attested session rather than accepting an owner or availability from the client.
  - Evidence (runtime): `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` is exercised by accepted browser fixture state at `/private/tmp/ple-blueprint-owned-pool-artifacts.wVCQ4m/comparison-fixture-state.json`, which records the created Private fork; the actual HTTP lineage receipt rejects concealed Private intermediates for other Instructors.
- [x] A fork records the source Blueprint Course and Blueprint Revision from which it was created.
  - Evidence (source): `crates/server/src/blueprint_course/fork.rs` `BlueprintForkSource` carries the source reference and Revision to the Store.
- [x] Forking a Blueprint Course creates new Blueprint Assessments.
  - Evidence (source): `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` allocates the forked tree from the exact source Revision.
  - Evidence (runtime): `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-owned-pool-http-proof.json`, verifying fresh Assessment and Pool IDs with the same ordered Question Revision membership.
- [x] Published Questions in the new Blueprint Assessments retain the same Published Question IDs and exact Revisions.
  - Evidence (source): `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` allocates the forked tree from the exact source Revision.
  - Evidence (runtime): `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-owned-pool-http-proof.json`, verifying fresh Assessment and Pool IDs with the same ordered Question Revision membership.
- [x] Question Pools in the new Blueprint Assessments are forked and receive new Question Pool IDs.
  - Evidence (source): `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` allocates the forked tree from the exact source Revision.
  - Evidence (runtime): `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-owned-pool-http-proof.json`, verifying fresh Assessment and Pool IDs with the same ordered Question Revision membership.
- [x] Forked Question Pools initially contain the same Published Question IDs and exact Revisions as their source.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_comparison.rs` `inventory_question_ids` derives comparison relationships from the exact fixed and Pool member Question IDs.
  - Evidence (runtime): `crates/question_model/src/blueprint_course/fork_comparison.rs` `inventory_question_ids` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-owned-pool-http-proof.json`, verifying forked Pools retain exact ordered Question Revision membership under fresh Pool IDs.
- [x] Forked Blueprint Courses develop independently and have their own Blueprint Revisions.
  - Evidence (source): `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` creates a separately editable fork tree.
- [x] Changes to a source Blueprint Course are never automatically applied to its forks.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_apply.rs` `apply_blueprint_fork` changes only explicit selections.
- [x] A Blueprint Course shows its known forks and the **Instructor** who owns each fork.
  - Evidence (runtime): accepted C881 actual-server proof at `/private/tmp/ple-fork-reader-artifacts.nRikDO` exercises `crates/server/src/blueprint_course/known_forks.rs` `list_known_forks`, returns each fork's owner and recorded origin, and conceals unrelated Instructors with `404 no-store`.
  - Evidence (source): `crates/server/src/blueprint_course/known_forks.rs` `list_known_forks` and `src/features/blueprint_forks/blueprint_fork_review.tsx` `BlueprintKnownForks` present authorized known-fork rows and owner names.
- [x] PLE should make newer source Revisions easy for the fork owner to discover and review.
  - Evidence (source): `src/features/blueprint_forks/blueprint_fork_review.tsx` `BlueprintKnownForks` presents the source/fork review entry.
- [x] PLE should make newer Revisions in downstream forks visible from their source Blueprint Course.
  - Evidence (source): `crates/server/src/blueprint_course/known_forks.rs` `list_known_forks` returns each visible child fork's current Revision for the source view.
  - Evidence (runtime): `crates/server/src/blueprint_course/known_forks.rs` `list_known_forks` is exercised by accepted compiled-main browser evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.wVCQ4m/comparison-browser.json`, which loads the source known-forks row before opening the pair review.
- [x] The fork owner decides whether to incorporate source changes into the fork.
  - Evidence (source): `crates/server/src/blueprint_course/fork_apply.rs` `apply_fork_update` passes explicit selected destinations and both source/fork Revision and metadata preconditions to the Store.
  - Evidence (runtime): `crates/server/src/blueprint_course/fork_apply.rs` `apply_fork_update` is exercised by accepted 84-request actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.vs0NCo/blueprint-local-id-apply-http-proof.json`, verifying owner selection, four denied preconditions, and zero-write denials.
- [x] PLE should make it easy for the fork owner to incorporate selected source changes.
  - Evidence (source): `src/features/blueprint_forks/blueprint_fork_apply.tsx` `BlueprintForkApply` presents selected current-pair Apply choices.
  - Evidence (runtime): `src/features/blueprint_forks/blueprint_fork_apply.tsx` `BlueprintForkApply` is exercised by accepted compiled-main browser proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.wVCQ4m/comparison-browser.json`, completing selected Apply at desktop and narrow viewports.

#### Blueprint Course Change Proposal specifications

- [ ] A **Blueprint Course Change Proposal** proposes changes from one Blueprint Course to another.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] An **Instructor** can create a Change Proposal for a Blueprint Course they do not own.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] A Change Proposal records the source Blueprint Course and exact Blueprint Revision.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] A Change Proposal records the target Blueprint Course and exact Blueprint Revision used for comparison.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] The proposed changes are represented using the canonical Blueprint Course JSON format.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] PLE compares the proposed JSON with the target Blueprint Revision to determine the proposed changes.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] A Change Proposal should present those changes in a human-readable interface rather than requiring
  the receiving **Instructor** to review raw JSON.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] A Change Proposal may include any Blueprint Course content represented in its canonical JSON.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] Changes may include Course names and metadata, Assessment names and settings, Assessment additions
  and removals, and Question membership changes.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] Question content changes belong to the Published Question and are not Blueprint Course changes.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] PLE should present proposed changes in terms meaningful to Instructors rather than as raw JSON changes.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] The receiving **Instructor** can review proposed changes before changing the target Blueprint Course.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] The receiving Instructor decides which proposed changes to accept.
  - Mismatch: No Change Proposal acceptance operation exists.
- [ ] The receiving Instructor may accept the entire Change Proposal or selected proposed changes.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] Accepted changes are applied to the current target Blueprint Course and create a new Blueprint Revision.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] The Change Proposal remains a record of what was proposed and what was accepted.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] If the target Blueprint Course changes after the proposal was created, PLE should show that the
  proposal was based on an older target Revision.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] PLE should not silently apply a proposal against a newer target Revision when the changes no longer
  apply cleanly.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.
- [ ] Change Proposals never directly change daughter Course Instances.
  - Mismatch: No Change Proposal implementation exists to verify this invariant.
- [ ] Daughter Course Instances receive accepted changes through the normal Blueprint incorporation workflow.
  - Mismatch: `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` and the existing current-pair comparison/Apply path do not implement a persisted Change Proposal with exact source/target comparison pins, reviewable canonical-JSON scope, selective acceptance, retained acceptance records, and stale-target handling. This current requirement is not established by fork/Apply evidence.

#### Blueprint Course comparison specifications

- [x] Any **Instructor** can compare related Blueprint Courses in the same fork lineage when both are visible to that Instructor.
  - Evidence (source): `schemas/base_schema/blueprint_lineage.sql` `ple_api.load_blueprint_comparison_sources` authorizes an arbitrary related visible current pair before it is projected.
  - Evidence (runtime): `schemas/base_schema/blueprint_lineage.sql` `ple_api.load_blueprint_comparison_sources` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-lineage-pair-http-proof.json`, covering visible sibling and transitive pairs in both orientations while concealing Private intermediates and denying unrelated pairs.
- [x] Fork comparison normally compares the newest Revision of the source Blueprint Course with the newest Revision of the fork.
  - Evidence (runtime): `src/api/decoders/blueprint_comparison.ts` `decodeBlueprintComparisonView` is exercised by accepted current-pair HTTP evidence, returning current source and fork names, ETags, and Revisions.
  - Evidence (source): `src/api/decoders/blueprint_comparison.ts` `decodeBlueprintComparisonView` requires the current source and fork Revision references.
- [x] Older Revisions remain available through Blueprint history but are not the normal comparison workflow.
  - Evidence (runtime): `src/features/blueprint_course/blueprint_history.tsx` `BlueprintHistory` inspects exact older Revisions separately from current-head comparison. Accepted bounded proof is `/private/tmp/ple-blueprint-owned-pool-artifacts.sEJZUB/history-proof.json`.
- [x] Blueprint Course differences are calculated from canonical JSON when the Instructor requests the comparison.
  - Evidence (source): `crates/server/src/blueprint_course/fork_review.rs` `load_comparison` requests C880's canonical comparison projection on demand rather than persisting comparison state.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_comparison.rs` `compare_blueprint_courses` compares canonical Blueprint snapshots.
  - Evidence (runtime): C882's actual HTTP receipt exercises `crates/server/src/blueprint_course/fork_review.rs` `load_comparison` as a real GET-only `no-store` review with zero `ple_data` mutations.
- [x] Shared Published Question IDs provide durable relationships between Published Questions across Blueprint Course forks.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_comparison.rs` `compare_blueprint_courses` derives relationships from shared Question IDs only.
  - Evidence (runtime): `crates/question_model/src/blueprint_course/fork_comparison.rs` `compare_blueprint_courses` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-lineage-pair-http-proof.json`, verifying Rev2-versus-Rev1 shared Question-ID relationships.
- [x] Blueprint Course comparison does not require Blueprint Assessment identity or history across forks.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_comparison.rs` `BlueprintComparisonAssessment` retains only side-local references while relationships carry shared Question IDs.
  - Evidence (runtime): `crates/question_model/src/blueprint_course/fork_comparison.rs` `BlueprintComparisonAssessment` is exercised by the accepted browser fixture at `/private/tmp/ple-blueprint-comparison-ui-proof/fixture.py`, which verifies source and fork Assessment IDs are disjoint before comparison and Apply.
- [ ] Comparison should show shared, added, removed, and changed Assessments, Published Questions, and Question Pools.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
- [x] Comparison should remain useful when Assessment names, order, or structure have changed.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_comparison.rs` `compare_blueprint_courses` uses shared Question IDs instead of Assessment names, positions, or cross-Blueprint Assessment identity.
  - Evidence (runtime): `crates/question_model/src/blueprint_course/fork_comparison.rs` `compare_blueprint_courses` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-lineage-pair-http-proof.json`, covering renamed, reordered, and split canonical content.
- [x] Comparison visibility follows Blueprint Course visibility rather than fork ownership.
  - Evidence (source): `crates/server/src/blueprint_course/fork_review.rs` `load_comparison` uses ordinary Blueprint visibility for read-only direct-source review.
  - Evidence (runtime): C881/C882 accepted `crates/server/src/blueprint_course/fork_review.rs` `load_comparison` ordinary-visibility direct-source review at `/private/tmp/ple-fork-reader-artifacts.nRikDO` and `/private/tmp/ple-fork-review-http-artifacts.LTUgsF` permits visible Public/Archived sides and conceals unauthorized Private sides; ownership restricts Apply, not comparison.

#### Blueprint Course JSON specifications

- [ ] Blueprint Courses have a canonical JSON representation for comparison, import, export, and exchange.
  - Mismatch: Current JSON is internal stored content; no canonical import/export exchange surface was found.
- [ ] Canonical Blueprint JSON must contain enough information to fully recreate a Blueprint Course.
  - Mismatch: No complete export/import round trip was found.
- [ ] Importing exported Blueprint JSON should reproduce the same Blueprint Course content and structure.
  - Mismatch: No Blueprint JSON import or export operation exists.
- [ ] Blueprint JSON contains Blueprint metadata and an ordered list of Blueprint Assessments.
  - Mismatch: Stored revision JSON does not demonstrate the required complete canonical exchange shape.
- [x] Blueprint Assessments contain only reusable teaching settings.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `ple_data.blueprint_content_is_closed` allowlists reusable Assessment content and defaults without Course delivery dates or release state.
- [ ] Blueprint Assessments contain ordered **Published Questions** and published **Question Pools**.
  - Mismatch: Current stored content pins Questions but does not verify published Pool support.
- [x] Blueprint Assessments have no deadlines, release dates, Student data, or other Course Instance settings.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_course_revision` and its children have no Student or delivery-date fields.
- [ ] Blueprint Revisions can be compared through their canonical JSON representations.
  - Mismatch: No canonical JSON comparison surface was found.
- [ ] Blueprint Course Change Proposals use canonical JSON to identify changes between Blueprint Revisions.
  - Mismatch: No Change Proposal implementation exists.
- N/A Canonical Blueprint JSON may support offline inspection or editing, even if it is not optimized for hand editing.
  - Reason: This explicitly optional future capability does not require implemented behavior.
- [ ] Canonical Blueprint JSON is the complete exchange format, not the primary persistence model.
  - Mismatch: No canonical exchange format implementation exists.

### Course Instance specifications

#### Course Instance creation specifications

- [x] An **Instructor** can create a Course Instance from a Public Blueprint Course.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.load_course_instance_blueprint` requires a Public Blueprint at the selected exact Revision; `src/pages/course_list_page.tsx` `TeachingCourseListPage` exposes the Adopted source only after public Blueprint discovery.
  - Evidence (runtime): `src/pages/course_list_page.tsx` `TeachingCourseListPage` was exercised in private actual-HTTP and exact-main browser proof: an Instructor created and published a Blueprint through its API, selected its exact Public Revision, created a daughter Course Instance, and read its Unreleased Practice Assessment with finite Attempt limit and dates unset. This does not establish Pool copying or release/delivery workflows.
- [x] **Instructors** can also create a new empty Course Instance without a parent Blueprint Course.
  - Evidence (source): `src/pages/course_list_page.tsx` `TeachingCourseListPage` defaults to Empty, activates Blueprint discovery only for Adopted, and sends the strict `source: { kind: "empty" }` wire through `src/api/http_client/course_instance.ts`.
  - Evidence (runtime): `src/pages/course_list_page.tsx` `TeachingCourseListPage` was exercised in a bounded authenticated actual-main browser and HTTP proof: an Instructor created an Empty Course Instance, then the resulting row and persisted Course read were observed, with zero Blueprint-list requests and `no-store` responses. Student creation denial was exercised at the HTTP boundary.
- [ ] Course Instances have **Students**, deadlines, releases, and other delivery-specific settings.
  - Mismatch: This audit has not found the complete Course Instance delivery model in A8 paths.
- [ ] Course Instances contain only **Published Questions** and published **Question Pools**.
  - Mismatch: Current adoption validates Question pins but not required published Pool behavior.
- [x] Course Instances are visible only to their co-**Instructors** and enrolled **Students**.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `resolve_course_navigation` permits only an active Course Member.
- [ ] Active Courses are current teaching Course Instances.
  - Mismatch: No active/inactive Course Instance lifecycle model was found.
- [ ] Inactive Courses are past Course Instances and retain Course metadata, including after
  FERPA-sensitive Student data is removed.
  - Mismatch: No inactive Course lifecycle and retention linkage was verified in A8 paths.
- [ ] An **Instructor** may deliberately publish reusable Course Instance structure as a new **Blueprint Course**.
  - Mismatch: No Course Instance-to-Blueprint publishing operation exists.
- [x] A new academic term uses a new Course Instance. Rollover is not a separate product model.
  - Evidence (source): `crates/question_model/src/course_term.rs` `CourseTerm` is input to each `CreateCourseInstanceInput`; no rollover model was found.

#### Blueprint adoption and daughter Course Instances

- [x] An **adoption** occurs when an **Instructor** creates a Course Instance from a Blueprint Course.
  - Evidence (source): `crates/learning-data-access/src/postgres/course_instance.rs` `create_course_instance` consumes Blueprint source inputs during creation.
- [x] Blueprint Courses track how many Course Instances have been created from them as their adoption count.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` computes `total_adoptions` by counting Course Instances with each Blueprint reference.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` asserts the adopted Blueprint summary has `total_adoptions` equal to 1.
- [x] A Course Instance created from a Blueprint Course is a daughter Course Instance of that Blueprint Course.
  - Evidence (source): `schemas/base_schema/course_core.sql` `course_instance` records Blueprint reference and Revision source columns.
- [x] A daughter Course Instance records its parent Blueprint Course and the exact Blueprint Revision used to create it.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CreateCourseInstanceInput` includes `blueprint_course` and `blueprint_revision`.
- [x] Creating a Course Instance from a Blueprint Course counts as an adoption of that Blueprint Course.
  - Evidence (source): `schemas/base_schema/course_core.sql` `course_instance_creation_event` records the Blueprint reference and Revision at creation.
- [x] The new Course Instance receives every Assessment from the selected Blueprint Revision.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_data.initialize_course_assessments` constructs the Course Assessments from selected Blueprint content.
- [ ] Creating a Course Instance from a Blueprint Course copies its Assessments, Questions, Question Pools, and reusable settings.
  - Mismatch: Current adoption evidence does not verify published Pool copying.
- [x] Course Instance Assessments created from a Blueprint Course start unreleased with dates unset.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_data.initialize_course_assessments` initializes adopted Assessments as unreleased with delivery dates unset.
- [x] New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` lazily obtains the authorized current-parent Course summary and offers each adopted Assessment for review; `src/api/assessment_release.ts` `CourseBlueprintUpdateReview` excludes direct local Assessments and carries matching, removed-source, Type-mismatch, changed, and automatically-added correspondences.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in actual-server and compiled-main proof: each Course-summary read returned five coherent rows (changed, matching, removed, Type mismatch, automatically added) after lazy open/reopen at 1280 by 900 and 390 by 844. The changed Assessment then reviewed and applied with exact source Revision 2 and daughter Edit CAS; the Course refresh showed the applied match. Student and unrelated reads returned `404 no-store`; a private parent was concealed from another Instructor in the privileged-availability fixture; Archived review remained available and new adoption was denied. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  - Owner: 08_courses.md > Course specifications > Blueprint Course specifications > Blueprint adoption and incorporation specifications (first current-source occurrence).
- [x] Routine Blueprint changes should be quick for an **Instructor** to review and incorporate.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` supplies one Course-level Review action, clear per-Assessment status labels, Refresh, and links to the existing Assessment detail Review/Apply workflow.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in compiled-main browser proof at 1280 by 900 and 390 by 844: lazy open/reopen GET behavior produced the five-row Course summary and the Course-to-Assessment detail review. Cancel issued zero POST requests; Apply used exact source Revision 2 plus daughter Edit CAS and a returning Course refresh showed the match. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  - Owner: 08_courses.md > Course specifications > Blueprint Course specifications > Blueprint adoption and incorporation specifications (first current-source occurrence).
- [x] It should be obvious when a daughter Course Instance is based on an older Blueprint Revision.
  - Evidence (source): `schemas/base_schema/course_operations.sql` `ple_api.load_course_instance`, `crates/learning-data-access/src/postgres/course_instance.rs` `decode_view`, `src/api/decoders/course_instance.ts` `decodeCourseInstanceView`, and `src/pages/course_instance_page.tsx` `CourseInstancePage` use the authorized parent origin and exact adopted/current Revision projection for the same visible notice.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` was covered by independently accepted actual-server/exact-main proof across empty, current, newer, and explicit synthetic Private-origin states; the visually inspected newer capture showed both Revision values and the stale notice. It preserved the original adoption pin, Assessment, and entries; its Work tables were empty, so this proof makes no populated-Student-Work claim. Unauthorized Student and unrelated-Instructor reads returned `404 no-store`. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.u1qUyY`.
  - Decision: This duplicate course-view indication does not implement the separate Blueprint update offer, review, approval, or apply workflow.
- [ ] The **Instructor** decides which changes to existing Assessments to incorporate.
  - Verification pending: source-audit this changed requirement against its current parent section and the existing implementation; no full current-scope proof is claimed by the prior wording.
  - Owner: 08_courses.md > Course specifications > Blueprint Course specifications > Blueprint adoption and incorporation specifications (first current-source occurrence).
- [x] Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` and the PostgreSQL Blueprint Store Save implement the same automatic-new append boundary documented in the earlier identical row.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` accepted connected proof and existing adoption lifecycle regression preserve exact pins/settings, distinct daughter Pool IDs, existing Student Work, original adoption pin, Unreleased state, and unset dates. Artifacts: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8` and `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Owner: 08_courses.md > Course specifications > Blueprint Course specifications > Blueprint adoption and incorporation specifications (first current-source occurrence).
- [x] Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` inserts only validated newly added Assessments and does not update existing daughter Assessments.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` proves the same negative invariant after changing retained source content, preserving daughter content/entries/actual Student Work through Save/replay/no-op/stale operations. Artifact: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Owner: 08_courses.md > Course specifications > Blueprint Course specifications > Blueprint adoption and incorporation specifications (first current-source occurrence).

### Course short and long name specifications

- [x] Blueprint Courses and Course Instances each have their own short name and long name.
  - Evidence (source): `crates/question_model/src/blueprint_course/blueprint_children.rs` `CreateBlueprintCourseInput` and `crates/learning-data-access/src/course_instance.rs` `CreateCourseInstanceInput` each own both names.
- [x] Short names are entered or chosen deliberately by **Instructors**.
  - Evidence (source): `crates/question_model/src/blueprint_course/blueprint_children.rs` `pub short_name: String` is a submitted validated field.
- [ ] Short names are for compact navigation and should stay under about 16 characters when practical.
  - Mismatch: Current name validation permits up to 200 or 500 characters without the stated compact guidance.
- [x] Long names are descriptive names used for headings, breadcrumbs, and Course listings.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `pub long_name: String` is documented as the heading and breadcrumb name.
- N/A A Blueprint Course might be `Biochemistry` / `Upper-Level Introductory Biochemistry`.
  - Reason: This is an illustrative name example, not an implementation requirement.
- N/A A Course Instance might be `BCHM 355/455` / `BCHM 355/455 Section 20 Biochemistry (Roosevelt U; Spring 2026)`.
  - Reason: This is an illustrative name example, not an implementation requirement.
- [x] Course Instance names are properties of the Course Instance and are not derived from Blueprint Course names.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CreateCourseInstanceInput` requires independently supplied `short_name` and `long_name`.
