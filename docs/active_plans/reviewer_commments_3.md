> **Historical review input, not current instructions.** Current authority is
> [implementation_plan.md](implementation_plan.md),
> [release_completion_plan.md](active/release_completion_plan.md), and
> [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md). The M0 result is concluded evidence.

human: evaluate plan for 10 million problems, 1,000 instructors, and 50,000 students; it is flexible for that load? even if we start with 500 problems, 2 instructors and 100 students.

reviewer: Yes. The architecture can scale from 500 to 10 million problems and from 100 to 50,000 students, provided the boundaries are designed correctly. The main scaling risk is not the problem catalog. It is concurrent grading, attempt history, and search.

Scale assessment

Area Expected load Assessment
Published problems 10 million Straightforward with PostgreSQL metadata plus S3 assets
Instructors 1,000 Small
Students 50,000 Moderate
Simultaneous students Probably 1,000 to 10,000 Requires horizontal application scaling
Attempts and submissions Potentially hundreds of millions Main database-growth concern
Problem search 10 million indexed records Requires dedicated search planning
WeBWorK grading Bursty and CPU-heavy Requires a scalable worker pool

Keep the architecture simple initially

Start with:

Load balancer
|
TypeScript application containers
|
PostgreSQL
|
S3-compatible object storage
Separate:
WeBWorK renderer/grader containers

The same structure can serve both the initial and projected load. The deployment size changes, but the application model does not.

Use one shared PostgreSQL cluster. At 1,000 instructors, database-per-instructor would produce unnecessary operational overhead; global account, workspace, Course, and Student ownership columns provide the required boundaries.

Ten million problems

Ten million rows is not unusually large for PostgreSQL. The catalog should store compact metadata:

problem
problem_version
problem_backend
problem_tag
problem_asset
publication
license

Large or binary Source Object References remain in S3.

The important design constraints are:

- immutable published versions
- stable problem IDs
- indexed ownership and publication fields
- normalized tags and subjects
- checksums for all Source Object References
- no S3 bucket listings during requests
- no parsing ZIP or XML files during student access

Ten million problems may make complex catalog search expensive. PostgreSQL full-text search may be enough initially. A dedicated search index can be added later without changing the authoritative database.

Student activity is the larger concern

Assume:

- 50,000 students
- 10 assignments each term
- 20 problems per assignment
- 3 attempts per problem

That produces about:

50,000 × 10 × 20 × 3 = 30 million question attempts per term

Over several years, the attempts and audit tables may reach hundreds of millions of rows.

Plan these as append-heavy tables:

- assignment_attempts
- question_attempts
- submissions
- grade_events
- timer_events
- audit_events

PostgreSQL supports declarative table partitioning, which can later split large logical tables into smaller physical tables. Partitioning by academic term or creation month would fit this workload.

Do not partition every table from the first prototype. Design IDs and timestamps so partitioning can be introduced before activity tables become large.

Concurrent use

Fifty thousand registered students does not mean 50,000 simultaneous requests.

Even with 5,000 active students, the stateless TypeScript application can run across multiple containers:

Load balancer
|
+---+---+---+---+
API API API API

Application containers should not hold authoritative session or timer state in local memory. Any container must be able to handle the next request.

RDS PostgreSQL can scale vertically, use provisioned I/O, and add read replicas. Read replicas can remove catalog and reporting queries from the primary write database.

WeBWorK is the likely bottleneck

WeBWorK rendering and grading may consume much more CPU than ordinary database requests.

Use a pool of isolated workers:

Peptidyle API
|
grading queue
|
+---------+---------+
WeBWorK WeBWorK WeBWorK
worker worker worker

Scale workers based on:

- queue depth
- grading latency
- CPU usage
- timeout rate

Question prefetching can hide render latency, but submitted answers must still be graded server-side.

Object storage

Ten million problems may reference tens of millions of objects. That is appropriate for S3. AWS describes S3 as supporting effectively arbitrary data volumes, and its object model is designed for large-scale storage.

Use:

- checksum-addressed objects
- immutable keys
- CDN caching
- lifecycle rules
- separate source and generated-object classes
- no user filenames as authoritative keys

Database records should point directly to known object IDs and keys.

Shared catalog and owned records

Use this boundary:

Shared

- published problems
- immutable problem versions
- shared assets
- licenses
- taxonomy
- backend definitions

Workspace-, Course-, and Student-owned

- drafts
- courses
- assignments
- enrollments
- student attempts
- grades
- private analytics

Assignments reference immutable shared problem versions.

Do not copy published problems into each Course database.

Design now for later scale

These choices should be made before implementation:

1. Use globally unique IDs rather than sequential IDs as external identifiers.
2. Keep application servers stateless.
3. Make published problem versions immutable.
4. Keep binary files outside PostgreSQL.
5. Separate synchronous grading from asynchronous export and import work.
6. Record exact renderer version, seed, and problem version for every attempt.
7. Include the exact workspace, Course, Student, account, or capability identity that owns each record.
8. Use cursor pagination, never large OFFSET queries.
9. Make catalog search replaceable by a dedicated search service.
10. Keep database access behind repository or service boundaries.

What not to build yet

At the initial scale, you do not need:

- Kubernetes
- multiple regional deployments
- database sharding
- Redis
- Kafka
- dedicated search infrastructure
- dozens of microservices
- separate databases per instructor

Those can create more complexity than the application itself.

Recommendation

The proposed architecture is flexible enough for both target sizes:

Initial:
1 application container
1 WeBWorK container
1 modest PostgreSQL instance
1 object-storage bucket
Later:
multiple application containers
autoscaled WeBWorK workers
larger PostgreSQL primary
read replicas
partitioned attempt tables
dedicated search index
CDN-backed object delivery

The architecture does not need to change fundamentally. The initial schema and service boundaries must simply
avoid assumptions that there will always be one application process, one renderer, or a small attempt history.

human: I have had students push 30 or more attempts to better their learning. just because a student gets 100%
on an assignments does not mean that they cannot do it again to learn better through algorithmic problem variation

reviewer:

That materially changes the load model. Assignment completion is not the end of activity. Students may restart completed assignments repeatedly, with new algorithmic variants.

Use separate concepts:

student_record_id
assignment_id
current_grade
best_grade
completion_status
first_completed_at
assignment_attempt
attempt_number
started_at
completed_at
score
mode
question_attempt
issued_question_id
question_revision_id
seed
response
result

A Student may have one enrollment record but 30 or more Assignment Attempts. Each Assignment
Attempt may contain several Question Attempts per Issued Question.

For 50,000 students, 10 assignments, 20 questions, and 30 complete runs:

50,000 × 10 × 20 × 30 = 300 million question instances

Additional incorrect attempts could push the activity tables well beyond 500 million rows over time.

This is still feasible, but the plan should account for:

- append-only attempt and event tables
- time-based or academic-term partitioning
- compact response records
- separate summary tables for current grades and instructor views
- asynchronous aggregation of learning analytics
- retention rules for verbose render traces and temporary artifacts
- exact seed and renderer version storage for reproducibility

Do not calculate the current grade by scanning every historical attempt. Update a compact summary transactionally when a run changes:

student_assignment_summary
best_score
latest_score
completed_run_count
total_question_attempts
last_activity_at

Historical runs remain available for learning analysis, while ordinary course pages read the summary row.

The product model should also distinguish:

- completion requirement: first time all required questions are correct
- grade policy: first, latest, highest, or instructor-defined result
- continued practice: unlimited new runs after completion
- variation policy: new seeds, selected Question Variants, or full regeneration

The architecture remains suitable, but the earlier 30-million-row estimate was too low. Peptidyle should be
designed as a high-volume attempt-event system, not merely an assignment submission system.
