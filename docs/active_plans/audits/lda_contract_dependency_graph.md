# Data-access contract dependency graph

## Purpose and evidence

This one-time record sets the implementation order for the data-access
contract cutover. It was derived from the current contract modules, their
in-memory and PostgreSQL implementations, live selectors in
`tests/e2e/e2e_database_baseline.sh`, and Graphify affected sets for
`session.rs`, `contracts/courses.rs`, `contracts/catalog.rs`,
`contracts/runs.rs`, and `contracts/grading_operations.rs`.

The graph records source dependencies, not acceptance. A family is complete
only after its exact-scope contract, both adapters, focused offline selector,
and assigned disposable PostgreSQL case agree.

## Shared identity boundary

`session.rs` is a cross-family root. Its affected set reaches every Store
family, both adapters, server authentication, worker composition, and seed
construction. The preparatory `ActorContext` boundary remains stable while
the contract families below acquire exact resource inputs.

The final session-record cutover follows those families. It creates an actor
only from a resolved durable session record and removes the old provider
subject and duplicate durable-session models in one compile-coordinated
change. A transaction adapter installs the resolved actor; it does not create
an authorization scope.

## Contract families and order

| Order | Contract surface | Exact durable parent | Depends on | Assigned proof lane |
| --- | --- | --- | --- | --- |
| 1 | `catalog.rs`, `catalog_store.rs`, `problem_curation.rs` | Published `QuestionId` plus immutable version; private workspace relationship for edits | resolved actor | catalog Store selectors and catalog PostgreSQL selectors |
| 2 | `reusable_curriculum.rs`, `curriculum_adoption.rs` | `BlueprintReference`, immutable revision, and destination CourseInstance application | 1, resolved actor | reusable-curriculum Memory selectors, then direct baseline PostgreSQL selectors |
| 3 | `courses.rs`, `store_capabilities.rs` course and membership operations | `CourseId`, membership episode, assigned Instructor, and approved-Instructor relation | 2 for ordinary CourseInstance creation; resolved actor | course and membership selectors, plus `postgres_enrollment_live` replacement cases |
| 4 | `entitlement.rs`, Gradebook operations in `store_capabilities.rs` | Student record, matching enrollment episode, CourseInstance, assignment | 3 | entitlement and calculated-Gradebook selectors, plus `postgres_entitlement_membership_live` and gradebook cases |
| 5 | assignment-definition, assignment-editing, preview, and policy operations | CourseInstance and exact assignment revision | 3, 4 where Student disclosure applies | assignment definition/policy selectors and teaching projection PostgreSQL cases |
| 6 | `runs.rs`, issued snapshot, feedback, and activity operations | issued run/attempt, Student enrollment, exact assignment and version | 4, 5 | run and receipt selectors, submission replay PostgreSQL case |
| 7 | `grading_operations.rs` and automated-submission execution | accepted submission, attempt, assignment generation, immutable receipt, worker lease | 6 | grading selectors and worker-claim PostgreSQL case |
| 8 | `workers.rs`, retention, exports, object delivery, external tools | closed typed target, generation, and worker lease | 1, 3, 6, 7 | worker, retention, and object-delivery selectors |
| 9 | `store.rs` facade and `store_capabilities.rs` composition | exact inputs introduced by families 1-8 | 1-8 | feature-enabled LDA compilation and complete focused set |
| 10 | final session record/store and server authentication | resolved `SessionId` and global `UserId` | 1-9 | focused LDA/server tests, then the affected protected-route and RLS lanes |

## Independent work boundaries

Catalog/private authoring and the reusable-course source surface share only
the resolved actor and stable catalog version references. Their contract
definitions can settle before course delivery implementation starts.

Course/membership and Student/Gradebook are sequential: Student access names
the exact enrollment and course-membership episode established by the course
family. Assignment delivery follows that relation; issued work and accepted
submissions follow delivery. Workers consume locked durable targets from these
completed families and never infer their parent from a broad installation
scope.

The facade is deliberately last. It composes capabilities but owns no new
authorization rule. Updating it only after each focused trait prevents a
temporary generic context from becoming the next durable API.

## Live PostgreSQL allocation

The present baseline runner supplies evidence groups that move with the
families, rather than a single undifferentiated database gate:

| Family | Existing live evidence to rework against the baseline |
| --- | --- |
| course, membership, Student | `postgres_enrollment_live`, `postgres_entitlement_membership_live` |
| catalog and authoring | catalog search, catalog detail, discovery-evidence, and import-provenance selectors |
| course delivery and Gradebook | course-term, effective-policy, disclosure-policy, course-grade, and teaching-projection selectors |
| issued work and submissions | submission replay, flat-question grading, and QTI provenance selectors |
| workers | `postgres_worker_filter_live` and accepted-submission execution cases |
| external delivery and objects | flat-question asset, external-tool, and object-delivery cases |

Each migrated case will assert its named relation or lease predicate using the
server-resolved actor. A caller-supplied installation-wide value proves no
authority.

## Implementation rule

Each family first replaces broad scope inputs with the exact actor and durable
resource identities in its public contract. Its Memory and PostgreSQL adapters
then evaluate the documented relation in their protected operation. Consumers
follow only after the focused family passes. This preserves one direct model
through the baseline, Store contracts, server, and browser transport.
