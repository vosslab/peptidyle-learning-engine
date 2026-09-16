## Courses

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

### Blueprint Courses

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

#### Blueprint Course lifecycle

- [ ] Blueprint Courses have three lifecycle states: **Private**, **Public**, and **Archived**.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_course.availability` permits `private`, `public`, and `archived`.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle receipt against the complete state claim.
- [ ] New Blueprint Courses and forks start Private.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.create_blueprint_course` and `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` create Private lineages.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle receipt against both creation paths.
- [ ] Private Blueprint Courses are visible only to their owning **Instructor**.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` and `ple_api.load_blueprint_course` limit Private access to the owner.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle and actual HTTP receipts against the owner-only claim.
- [ ] Private Blueprint Courses cannot be adopted to create daughter **Course Instances**.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.load_course_instance_blueprint` requires Public availability.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle receipt against the Private-adoption denial.
- [ ] Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` lists Public Blueprints to active Instructors.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle and actual HTTP receipts against the full vetted-Instructor visibility and reusability claim.
  - Owner: Same implementation finding as the earlier Public Blueprint Courses bullet.
- [ ] Public Blueprint Courses can be adopted to create daughter Course Instances.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.load_course_instance_blueprint` requires Public availability for adoption.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle and actual HTTP receipts against the complete Public-adoption workflow.
- [x] Archived Blueprint Courses are read-only and no longer actively maintained.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.save_blueprint_course` and `ple_api.rename_blueprint_course` lock the owner-visible Blueprint and reject `archived` before replay, CAS, or no-op handling.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` covers denied replay, no-op, changed Save, and rename without changing the Blueprint state, then restored writes.
  - Evidence (runtime): `schemas/base_schema/blueprint_operations.sql` `ple_api.save_blueprint_course`; `/private/tmp/ple-daughter-revision-notice-artifacts.JhV6aj/archived-blueprint-http-proof.json` records five `409` denials with unchanged state and preserved Private/Public/restored writes.
- [x] Archived Blueprint Courses remain visible by every vetted **Instructor**.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` defaults `p_include_archived` to false and returns Archived records when that explicit parameter is true.
  - Evidence (runtime): `crates/server/src/blueprint_course.rs` `list_blueprints`; `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-http-proof.json` records owner and nonowner active-Instructor default/false/true lists: only true includes the Archived Blueprint, while the Private Blueprint remains owner-only. The same receipt records nonowner Archived detail `200`, Private detail `404`, Student list/detail `404`, and invalid query values `400`.
- [x] Archived Blueprint Courses are excluded from normal search results unless the search explicitly includes them.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` filters Public, owning Private, and only explicitly requested Archived records; `crates/server/src/blueprint_course.rs` `BlueprintCourseListQuery` accepts only the typed `includeArchived` boolean.
  - Evidence (runtime): `src/features/blueprint_course/blueprint_course_workspace.tsx` `changeIncludeArchived`; `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-browser-proof.json` records the actual compiled-main default-off, Include Archived, read-only Archived-detail, and return-to-off workflow with eight GETs and zero writes. Its companion HTTP receipt records default/false/true membership and strict invalid-query `400` results.
- [x] Archived Blueprint Courses cannot be adopted to create new daughter Course Instances.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.load_course_instance_blueprint` requires Blueprint availability `public` for exact-Revision adoption.
- [ ] Archived Blueprint Courses can be forked but not adopted.
  - Evidence (source): `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` accepts Public or Archived sources, while adoption requires Public availability.
  - Mismatch: `src/api/blueprint_course.ts` has no Instructor fork client method, and `BlueprintCourseLifecycleControls` has no fork action.
- [ ] The owning **Instructor** can return an Archived Blueprint Course to Public before adopting it again.
  - Evidence (source): `crates/learning-data-access/src/postgres/blueprint_course.rs` `restore_blueprint` sets availability to `public`.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle and actual HTTP receipts against restore followed by adoption.
- [ ] Other **Instructors** can fork an Archived Blueprint Course to create a new Private Blueprint Course.
  - Evidence (source): `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` accepts Archived sources and creates a Private child owned by the actor.
  - Mismatch: `src/api/blueprint_course.ts` has no Instructor fork client method, and `BlueprintCourseLifecycleControls` has no fork action.
- [x] Blueprint Courses have no separate draft state.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `CHECK (availability IN ('private', 'public', 'archived'))` defines the complete Blueprint availability state.

#### Blueprint Course revisions

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

#### Blueprint Course stewardship

- [ ] **Instructors** can Star or Watch Public and Archived Blueprint Courses.
  - Mismatch: No Blueprint Star or Watch model, route, or store operation was found.
- [ ] A Star is a visible endorsement and helps **Instructors** save useful Blueprint Courses.
  - Mismatch: No Star model or presentation was found.
- [ ] Vetted **Instructors** can see who Starred a Blueprint Course and its Star count.
  - Mismatch: No Star model or presentation was found.
- [ ] Watching a Blueprint Course is private.
  - Mismatch: No Watch model or presentation was found.
- [ ] Watchers are notified about new Blueprint Revisions and other important Blueprint changes.
  - Mismatch: No Watch or notification implementation was found.
- [ ] Forking or adopting a Blueprint Course does not automatically Star or Watch it.
  - Mismatch: No Star, Watch, or fork implementation exists to verify this invariant.
- [ ] Stars and Watches belong to the Blueprint Course across all of its Revisions.
  - Mismatch: No Star or Watch persistence exists.

#### Blueprint adoption and updates

- [x] Blueprint adoption copies every Assessment from the Blueprint Course into the Course Instance.
  - Evidence (source): `crates/learning-data-access/src/postgres/course_instance.rs` `create_course_instance` obtains `creation_assignments` before atomic creation.
- [x] Course Instances pin the exact Blueprint Revision from which they were adopted.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CourseInstanceCreationSource` requires an exact immutable Blueprint Revision source for adoption.
- [x] New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review and approval.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` lazily obtains the authorized current-parent Course summary and offers each adopted Assessment for review; `src/api/assessment_release.ts` `CourseBlueprintUpdateReview` excludes direct local Assessments and carries matching, removed-source, Type-mismatch, changed, and automatically-added correspondences.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in actual-server and compiled-main proof: each Course-summary read returned five coherent rows (changed, matching, removed, Type mismatch, automatically added) after lazy open/reopen at 1280 by 900 and 390 by 844. The changed Assessment then reviewed and applied with exact source Revision 2 and daughter Edit CAS; the Course refresh showed the applied match. Student and unrelated reads returned `404 no-store`; a private parent was concealed from another Instructor in the privileged-availability fixture; Archived review remained available and new adoption was denied. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
- [x] Routine Blueprint updates should be quick for an **Instructor** to review and approve.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` supplies one Course-level Review action, clear per-Assessment status labels, Refresh, and links to the existing Assessment detail Review/Apply workflow.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in compiled-main browser proof at 1280 by 900 and 390 by 844: lazy open/reopen GET behavior produced the five-row Course summary and the Course-to-Assessment detail review. Cancel issued zero POST requests; Apply used exact source Revision 2 plus daughter Edit CAS and a returning Course refresh showed the match. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
- [x] It should be obvious when a Course Instance is using an older Blueprint Revision.
  - Evidence (source): `schemas/base_schema/course_operations.sql` `ple_api.load_course_instance`, `crates/learning-data-access/src/postgres/course_instance.rs` `decode_view`, `src/api/decoders/course_instance.ts` `decodeCourseInstanceView`, and `src/pages/course_instance_page.tsx` `CourseInstancePage` carry the adopted and current Revision numbers and render the older-Revision notice with strict `bigint` comparisons.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` was exercised by accepted independent actual-server/exact-main browser proof across empty, current, newer, and explicit synthetic Private-origin states. The newer state visibly showed its original adopted Revision and the current newer Revision; Student and unrelated-Instructor reads returned nonenumerating `404 no-store`, no extra Blueprint fetch or write occurred, and browser errors were empty. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.u1qUyY`.
  - Decision: This read-only notice makes a stale daughter obvious. It does not offer, review, approve, or apply a Blueprint update.
- [x] Changes to existing Assessments follow the Blueprint Revision update workflow.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` discovers each adopted Assessment from one current parent Revision; `src/api/assessment_release.ts` `LiveAssessmentReleaseClient` defines the existing detail Apply with source-Revision and daughter-Edit CAS. No persisted offer, receipt, comparison baseline, or new update table is introduced.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in actual-server and compiled-main proof: changed, matching, removed-source, and Type-mismatch existing Assessments were classified before the changed one was explicitly applied. The Course-summary flow is distinct from the earlier per-Assessment proof of exact pins, stale-CAS/no-op/invalid-Released rollback, dates, status, origin, and one populated Assessment Attempt hash at `/private/tmp/ple-daughter-revision-notice-artifacts.kE8MnT`; neither artifact claims all Student Work. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
- [x] Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` validates the new-reference delta and inserts only new Assessments; it does not update existing daughter Assessments.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` changes a retained source Assessment title and proves existing daughter Assessment content, entries, and actual Student Work unchanged through Save/replay/no-op/stale operations. Accepted artifact: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Decision: This negative invariant remains separate from the verified Course-review workflow; it does not claim direct-Assessments or all Student Work.
- [x] Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Evidence (source): `crates/learning-data-access/src/postgres/blueprint_course.rs` Save and `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` atomically append only newly added Assessments to daughters with fresh Course-owned Pool identities and unset dates.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` passed connected PostgreSQL 17 proof, preserving exact pins/settings, existing Assessment content and actual Student Work, and the original adoption Revision pin across two daughters including an inactive Course; an unrelated empty Course remained unchanged. Replay/no-op/stale saves made no duplicate append. Artifact: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8` also accepted temporary bad-payload rollback proof. Existing connected adoption lifecycle regression passed 1 test with 0 ignored: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Decision: Only automatic-new Assessment propagation is verified, not C410 existing-Assessment update offers or the whole Course milestone.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` now invokes the `append.rs` helper `assert_new_assessment_save_preserves_daughter_work`; the existing permanent lifecycle regression passed 1 test with 0 ignored in `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. No separate seed-sharing test was retained.

#### Blueprint Course forks and Change Proposals

- [ ] An **Instructor** can fork a **Blueprint Course** to create a new independent Blueprint Course.
  - Mismatch: No Blueprint fork operation was found.
- [ ] A fork records the source Blueprint Course and Blueprint Revision from which it was created.
  - Mismatch: No fork lineage persistence exists.
- [ ] Forked Blueprint Courses develop independently and have their own Blueprint Revisions.
  - Mismatch: No fork implementation exists.
- [ ] Changes to a source Blueprint Course are never automatically applied to its forks.
  - Mismatch: No fork implementation exists to enforce this invariant.
- [ ] A fork should make newer changes from its source Blueprint Course easy to discover and review.
  - Mismatch: No fork update UI or runtime proof exists.
- [ ] An **Instructor** can selectively bring changes from a source Blueprint Course into their fork.
  - Mismatch: No selective fork-update operation exists.
- [ ] An **Instructor** can create a **Blueprint Course Change Proposal** to propose changes to another Blueprint Course.
  - Mismatch: No Change Proposal model, route, or store operation was found.
- [ ] A Change Proposal shows added, removed, and changed Assessments and Question content.
  - Mismatch: No Change Proposal comparison implementation was found.
- [ ] The receiving **Instructor** decides which proposed changes to accept.
  - Mismatch: No Change Proposal acceptance operation exists.
- [ ] Accepted changes create a new Blueprint Revision of the receiving Blueprint Course.
  - Mismatch: No Change Proposal acceptance operation exists.
- [ ] Change Proposals never directly change daughter Course Instances.
  - Mismatch: No Change Proposal implementation exists to verify this invariant.
- [ ] Daughter Course Instances receive accepted changes through the normal Blueprint update workflow.
  - Mismatch: No Change Proposal or Blueprint update workflow exists.

#### Blueprint Course JSON

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

### Course Instances

#### Course Instance creation

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
- [x] New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review and approval.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` lazily obtains the authorized current-parent Course summary and offers each adopted Assessment for review; `src/api/assessment_release.ts` `CourseBlueprintUpdateReview` excludes direct local Assessments and carries matching, removed-source, Type-mismatch, changed, and automatically-added correspondences.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in actual-server and compiled-main proof: each Course-summary read returned five coherent rows (changed, matching, removed, Type mismatch, automatically added) after lazy open/reopen at 1280 by 900 and 390 by 844. The changed Assessment then reviewed and applied with exact source Revision 2 and daughter Edit CAS; the Course refresh showed the applied match. Student and unrelated reads returned `404 no-store`; a private parent was concealed from another Instructor in the privileged-availability fixture; Archived review remained available and new adoption was denied. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  - Owner: Same verified Course-review workflow as the earlier Blueprint update offer bullet.
- [x] Routine Blueprint updates should be quick for an **Instructor** to review and approve.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` supplies one Course-level Review action, clear per-Assessment status labels, Refresh, and links to the existing Assessment detail Review/Apply workflow.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in compiled-main browser proof at 1280 by 900 and 390 by 844: lazy open/reopen GET behavior produced the five-row Course summary and the Course-to-Assessment detail review. Cancel issued zero POST requests; Apply used exact source Revision 2 plus daughter Edit CAS and a returning Course refresh showed the match. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  - Owner: Same verified Course-review workflow as the earlier routine Blueprint updates bullet.
- [x] It should be obvious when a daughter Course Instance is using an older Blueprint Revision.
  - Evidence (source): `schemas/base_schema/course_operations.sql` `ple_api.load_course_instance`, `crates/learning-data-access/src/postgres/course_instance.rs` `decode_view`, `src/api/decoders/course_instance.ts` `decodeCourseInstanceView`, and `src/pages/course_instance_page.tsx` `CourseInstancePage` use the authorized parent origin and exact adopted/current Revision projection for the same visible notice.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` was covered by independently accepted actual-server/exact-main proof across empty, current, newer, and explicit synthetic Private-origin states; the visually inspected newer capture showed both Revision values and the stale notice. It preserved the original adoption pin, Assessment, and entries; its Work tables were empty, so this proof makes no populated-Student-Work claim. Unauthorized Student and unrelated-Instructor reads returned `404 no-store`. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.u1qUyY`.
  - Decision: This duplicate course-view indication does not implement the separate Blueprint update offer, review, approval, or apply workflow.
- [x] Changes to existing Assessments follow the Blueprint Revision update workflow.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` discovers each adopted Assessment from one current parent Revision; `src/api/assessment_release.ts` `LiveAssessmentReleaseClient` defines the existing detail Apply with source-Revision and daughter-Edit CAS. No persisted offer, receipt, comparison baseline, or new update table is introduced.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in actual-server and compiled-main proof: changed, matching, removed-source, and Type-mismatch existing Assessments were classified before the changed one was explicitly applied. The Course-summary flow is distinct from the earlier per-Assessment proof of exact pins, stale-CAS/no-op/invalid-Released rollback, dates, status, origin, and one populated Assessment Attempt hash at `/private/tmp/ple-daughter-revision-notice-artifacts.kE8MnT`; neither artifact claims all Student Work. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  - Owner: Same verified Course-review workflow as the earlier existing-Assessment update bullet.
- [x] Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` and the PostgreSQL Blueprint Store Save implement the same automatic-new append boundary documented in the earlier identical row.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` accepted connected proof and existing adoption lifecycle regression preserve exact pins/settings, distinct daughter Pool IDs, existing Student Work, original adoption pin, Unreleased state, and unset dates. Artifacts: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8` and `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Owner: Same implementation finding as the earlier newly added Blueprint Assessments bullet.
- [x] Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` inserts only validated newly added Assessments and does not update existing daughter Assessments.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` proves the same negative invariant after changing retained source content, preserving daughter content/entries/actual Student Work through Save/replay/no-op/stale operations. Artifact: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Owner: Same implementation finding as the earlier non-silent Blueprint changes bullet.

### Course names

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
