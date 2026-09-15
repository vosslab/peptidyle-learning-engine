## Courses

- [ ] **Courses** organize reusable teaching content and its delivery to **Students**.
  - Mismatch: The current Course model does not establish the complete stated product boundary.
- [ ] PLE has two Course forms: **Blueprint Courses** and **Course Instances**.
  - Mismatch: The current paths implement related records but do not verify the complete product distinction.
- [ ] **Blueprint Courses** provide reusable course designs for creating Course Instances.
  - Mismatch: Adoption is implemented only for the current stored Blueprint shape.
- [ ] Course Instances may be created from a Blueprint Course or started empty.
  - Mismatch: `CreateCourseInstanceInput` requires a Blueprint source; no empty creation exists.
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
  - Mismatch: Current `available` availability has no verified vetted-Instructor public-read contract.
- [ ] Blueprint Courses contain only **Published Questions** and published **Question Pools**.
  - Mismatch: Current pin validation covers Question revisions but not the required published Pool behavior.
- [ ] An **Instructor** may deliberately publish an existing Course Instance structure as a new Blueprint Course.
  - Mismatch: No Course Instance-to-Blueprint publishing route or store operation was found.

#### Blueprint Course lifecycle

- [ ] Blueprint Courses have three lifecycle states: **Private**, **Public**, and **Archived**.
  - Mismatch: `blueprint_course.availability` permits only `available` and `archived`.
- [ ] New Blueprint Courses and forks start Private.
  - Mismatch: New records use `available`; no fork implementation was found.
- [ ] Private Blueprint Courses are visible only to their owning **Instructor**.
  - Mismatch: No Private lifecycle state exists.
- [ ] Private Blueprint Courses cannot be adopted to create daughter **Course Instances**.
  - Mismatch: No Private lifecycle state exists.
- [ ] Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
  - Mismatch: No Public lifecycle state or vetted-Instructor contract exists.
  - Owner: Same implementation finding as the earlier Public Blueprint Courses bullet.
- [ ] Public Blueprint Courses can be adopted to create daughter Course Instances.
  - Mismatch: Adoption does not prove the required Public lifecycle gate.
- [ ] Archived Blueprint Courses are read-only and no longer actively maintained.
  - Mismatch: `schemas/base_schema/blueprints.sql` `ple_api.save_blueprint_course` checks ownership but does not reject an Archived Blueprint Course, so its owner can still save changed content.
- [ ] Archived Blueprint Courses remain visible by every vetted **Instructor**.
  - Mismatch: No vetted-Instructor archived visibility evidence was found.
- [ ] Archived Blueprint Courses are excluded from normal search results unless the search explicitly includes them.
  - Mismatch: Blueprint listing has no include-archived search option.
- [x] Archived Blueprint Courses cannot be adopted to create new daughter Course Instances.
  - Evidence (source): `crates/learning-data-access/src/postgres/course_blueprint_adoption.rs` `creation_assignments` resolves an available exact Blueprint Revision before creation.
- [ ] Archived Blueprint Courses can be forked but not adopted.
  - Mismatch: No fork operation exists.
- [ ] The owning **Instructor** can return an Archived Blueprint Course to Public before adopting it again.
  - Mismatch: `restore_blueprint` returns `available`, not the required Public state.
- [ ] Other **Instructors** can fork an Archived Blueprint Course to create a new Private Blueprint Course.
  - Mismatch: No fork operation or Private state exists.
- [x] Blueprint Courses have no separate draft state.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `CHECK (availability IN ('private', 'public', 'archived'))` defines the complete Blueprint availability state.

#### Blueprint Course revisions

- [x] Blueprint Courses use immutable **Blueprint Revisions** for saved reusable content.
  - Evidence (source): `schemas/base_schema/blueprint_revision_integrity.sql` `blueprint_course_revision_is_immutable` rejects Revision updates and deletes.
- [x] Blueprint Course content editing uses explicit Save.
  - Evidence (source): `crates/server/src/blueprint_course.rs` `save_blueprint` is the explicit content-save route handler.
- [x] Saving changed Blueprint content creates the next Blueprint Revision.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `ple_api.save_blueprint_course` inserts the next `blueprint_course_revision` when `changed` is true.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` asserts one changed Save creates one new Revision.
- [x] Multiple content edits before Save become one Blueprint Revision.
  - Evidence (source): `crates/question_model/src/blueprint_course/blueprint_children.rs` `ReplaceBlueprintCourseContentInput` carries one complete replacement tree per Save.
- [x] Saving unchanged Blueprint content does not create another Revision.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `ple_api.save_blueprint_course` returns the expected Revision without inserting when `changed` is false.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` asserts a canonical no-op Save returns Revision 2 with `changed` false.
- [x] Blueprint Course metadata can change without creating a Blueprint Revision.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `ple_api.rename_blueprint_course` updates `blueprint_course` metadata without inserting a `blueprint_course_revision`.
- [x] Blueprint Course names are metadata and identify the Blueprint across Revisions.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_course` owns names while `blueprint_course_revision` keys content by course reference and revision.
- [x] Changing a Blueprint Course name does not create a new Blueprint Revision.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `ple_api.rename_blueprint_course` updates names and metadata ETag without inserting a `blueprint_course_revision`.

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
- [ ] New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review and approval.
  - Mismatch: No Blueprint update offer, review, or approval operation was found.
- [ ] Routine Blueprint updates should be quick for an **Instructor** to review and approve.
  - Mismatch: No applicable UI or runtime proof exists for this usability behavior.
- [ ] It should be obvious when a Course Instance is using an older Blueprint Revision.
  - Mismatch: No stale-Revision indicator or runtime proof was found.
- [ ] Changes to existing Assessments follow the Blueprint Revision update workflow.
  - Mismatch: No Blueprint update workflow exists.
- [ ] Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Mismatch: No update workflow exists to verify the non-silent behavior.
- [ ] Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Mismatch: No daughter-update implementation exists.

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
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_revision_assignment` stores Revision-owned reusable assignment positions without delivery settings.
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

- [ ] An **Instructor** can create a Course Instance from a Public Blueprint Course.
  - Mismatch: Creation accepts an exact Blueprint Revision but has no Public-state gate.
- [ ] **Instructors** can also create a new empty Course Instance without a parent Blueprint Course.
  - Mismatch: `CreateCourseInstanceInput` requires Blueprint Course and Revision fields.
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
  - Evidence (source): `schemas/base_schema/blueprints.sql` `ple_api.list_blueprint_courses` computes `total_adoptions` by counting Course Instances with each Blueprint reference.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` asserts the adopted Blueprint summary has `total_adoptions` equal to 1.
- [x] A Course Instance created from a Blueprint Course is a daughter Course Instance of that Blueprint Course.
  - Evidence (source): `schemas/base_schema/course_core.sql` `course_instance` records Blueprint reference and Revision source columns.
- [x] A daughter Course Instance records its parent Blueprint Course and the exact Blueprint Revision used to create it.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CreateCourseInstanceInput` includes `blueprint_course` and `blueprint_revision`.
- [x] Creating a Course Instance from a Blueprint Course counts as an adoption of that Blueprint Course.
  - Evidence (source): `schemas/base_schema/course_core.sql` `course_instance_creation_event` records the Blueprint reference and Revision at creation.
- [x] The new Course Instance receives every Assessment from the selected Blueprint Revision.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `initialize_course_assignments` constructs the Course assignments from selected Blueprint content.
- [ ] Creating a Course Instance from a Blueprint Course copies its Assessments, Questions, Question Pools, and reusable settings.
  - Mismatch: Current adoption evidence does not verify published Pool copying.
- [x] Course Instance Assessments created from a Blueprint Course start unreleased with dates unset.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `initialize_course_assignments` initializes adopted assignments as unreleased with delivery dates unset.
- [ ] New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review and approval.
  - Mismatch: No daughter-update implementation exists.
  - Owner: Same implementation finding as the earlier Blueprint update offer bullet.
- [ ] Routine Blueprint updates should be quick for an **Instructor** to review and approve.
  - Mismatch: No applicable UI or runtime proof exists for this usability behavior.
  - Owner: Same implementation finding as the earlier routine Blueprint updates bullet.
- [ ] It should be obvious when a daughter Course Instance is using an older Blueprint Revision.
  - Mismatch: No stale-Revision indicator or runtime proof was found.
- [ ] Changes to existing Assessments follow the Blueprint Revision update workflow.
  - Mismatch: No Blueprint update workflow exists.
  - Owner: Same implementation finding as the earlier Assessment update-workflow bullet.
- [ ] Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Mismatch: No daughter-update implementation exists.
  - Owner: Same implementation finding as the earlier newly added Blueprint Assessments bullet.
- [ ] Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Mismatch: No update workflow exists to verify the non-silent behavior.
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
