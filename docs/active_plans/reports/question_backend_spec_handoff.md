# Question backend specification handoff

> Historical handoff. Its unresolved-question table is superseded by
> [question_specs_open_questions.md](../decisions/question_specs_open_questions.md).
> A later source check confirms PG/PGML Draft creation and shared publication already exist in
> the content-loading tools. Only the browser-facing Draft creation route was found limited to
> Native JSON; see [QUESTION_IMPORT_SPEC.md](../../QUESTION_SPECS/QUESTION_IMPORT_SPEC.md).
> This handoff does not establish a missing WeBWorK workflow or a requirement for a new HTTP path.

## Scope completed

This report records evidence used to draft Backend, Type, Native JSON, WeBWorK, import, export,
QTI, and bulk-edit specifications. [Human Guidance](../../HUMAN_GUIDANCE.md) remains authority;
code was used only to name observed routes. The consolidated decision agenda is
[question_specs_open_questions.md](../decisions/question_specs_open_questions.md).

## Legacy coverage

`QUESTION_BACKEND_CONTRACTS.md` supplied the common ownership boundary, Native JSON answer-free
presentation, WeBWorK opaque rendering and 64 KiB response bound, feedback limits, retained
evidence, and extension checklist. `QTI-JSON_OBJECT_FORMAT.md` supplied Native JSON fields,
response shapes, validation, images, scripts, private/public compilation, and QTI boundaries.
`QUESTION_MODEL.md` supplied its connection to publication, Revisions, Pools, and Student Work.

## Observed HTTP evidence

`crates/server/src/authoring.rs` registers Draft create/list/delete, private source load/save,
images, teaching metadata, first publication, and revision publication under
`/api/authoring/drafts`. Native JSON uses
`application/vnd.peptidyle.question+json`, ETag/If-Match Draft edit numbers, and server-issued
image descriptors. `crates/server/src/question_library.rs` exposes Question lookup and exact
Revision detail. `/api/questions/bulk-metadata` supports Published Question metadata; the separate
Pool command allows Topic, Subtopic, and Tags.

## Prioritized unresolved questions

| State | Source | Undecided behavior | Impact | Concrete later question |
| --- | --- | --- | --- | --- |
| Open | Import workflow | How does a changed source record reconcile with an earlier import? | Prevents duplicate Questions or accidental Revisions. | Use the central Q01 wording: when prepare an owner-reviewed new Revision versus identify a different Question? |
| Settled but unimplemented | Approved import plan | Import uses normal authenticated HTTP APIs and captures returned IDs. | High: direct stores or SQL do not validate product behavior. | Which existing API operations need to be added for WeBWorK source and provenance? |
| Open | Export workflow | How are Pools represented in an export? | Export meaning and re-import fidelity differ. | Export Pool metadata and exact member tuples, or flatten only for a declared target? |
| Explicitly deferred | HG feedback rules | When does optional Question feedback become visible? | Affects student disclosure and source mapping. | After understanding PLE feedback use, choose automatic feedback or an Assessment timing rule. |
| Explicitly deferred | HG regrading rules | What happens after correcting an issued Native JSON key? | High infrastructure and fairness risk. | Define regrade behavior only after assessing durable Attempt evidence and scale. |
| Open | QTI profile inventory | Which reader/writer profiles are genuinely supported? | Prevents advertising a format that cannot round-trip safely. | Inspect qti-package-maker-rs public APIs and choose the first real QTI profile. |
| Open | Bulk editing | Can one atomic request change Questions and Pools together? | Lower priority; combined search does not require it. | Is mixed edit useful enough to justify a shared request and failure model? |

## Important limits

QTI reader and writer coverage differ. A network-uncertain import request is read back rather than
blindly repeated. Export targets, Pool export shape, and all-or-nothing versus partial results are
not settled. No specification claims that those behaviors are implemented.
