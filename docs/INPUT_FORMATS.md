# Input and exchange formats

This page is the file-I/O index for PLE. It names only formats implemented by the current source
or explicitly reserved in the release plan. It does not replace the linked schema, adapter, or API
contract.

## Browser and server boundary

The browser may receive answer-free question presentation, safe import reports, normalized roster
previews, and export status or downloadable artifacts. It never receives answer keys, expected
values, hidden correct choices, private rubrics, grading code, provider credentials, raw provider
results, Object Addresses, or source archives. The complete allowlist and privacy boundary are in
[API_CONTRACTS.md](API_CONTRACTS.md).

Authoring, import, grading, and export workers may handle private payloads after authorization.
Private source bytes, Answer Keys, Question Feedback, Question Answer Explanations,
and format-specific Question Grading Input remain in their owning adapter or
object-store boundary;
they are not browser formats merely because an Instructor can initiate the operation.

## Live-demo operator input

The supported front door is `./run_live_demo.sh` with `start`, `stop`, and `--headless`. It uses a
fixed disposable target and does not accept a caller-selected project, identity, environment,
SMTP configuration, or skip-build option. See [USAGE.md](USAGE.md) and
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md).

The lower-level lifecycle receives a private ASCII `NAME=value` owner manifest closed to `OWNER`,
`PROJECT`, `ENV_FILE`, and `CAPABILITY_FILE`; `PROFILE` is required for the live-demo-browser
owner. The referenced files are current-user-owned regular files with mode `0600`. The generated
`runtime.yaml` record is operational evidence, not an authoring or Student-upload format.

## Implemented authoring and import

| Format                 | Surface and media type                                                      | Implemented boundary                                                                                                                                        | Owner                                                                                                                                           |
| ---------------------- | --------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| PLE Question JSON      | Private PLE Question JSON route; `application/vnd.peptidyle.question+json`  | One answer-bearing document with the closed eight Question Formats: MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT; maximum 256 KiB                 | PLE Question JSON adapter source (currently `crates/adapters/ple/src/question_json.rs`), [QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) |
| Canvas QTI 1.2 ZIP     | Deferred private QTI profile route; exact `application/zip`; maximum 32 MiB | Strict `canvas-qti-1.2-static-single-choice/v1` profile. Unsupported semantics refuse without loss; archive, answers, mappings, and provenance stay private | [crates/adapters/qti/src/profiles/canvas.rs](../crates/adapters/qti/src/profiles/canvas.rs)                                                     |
| Blackboard QTI 2.1 ZIP | Deferred private QTI profile route; exact `application/zip`; maximum 32 MiB | Strict `blackboard-qti-2.1-static-single-choice-pool/v1` profile. Unsupported semantics refuse without loss; browser reports are answer-free                | [crates/adapters/qti/src/profiles/blackboard.rs](../crates/adapters/qti/src/profiles/blackboard.rs)                                             |
| H5P `.h5p` package     | Trusted private adapter/object-store boundary, not a browser upload route   | `H5P.MultiChoice` converts to an unpublished, key-free practice question with `clientRendering` only. It cannot be used as a server-graded assignment       | [crates/adapters/h5p/src/import.rs](../crates/adapters/h5p/src/import.rs), [CONTRACTS.md](CONTRACTS.md)                                         |

QTI conversion produces an answer-free draft handoff for the authoring UI. The worker separately
retains the original archive, private answer bindings, choice maps, digests, and source provenance.
The public runtime receives only the PLE Question projection. See
[implementation_plan.md](active_plans/implementation_plan.md) and
[QUESTION_MODEL.md](QUESTION_MODEL.md).

## Private server source

PLE can publish a private immutable PG or PGML Question Source through a Source Object Reference to the configured external
`webwork-pg-renderer`. This is not a Student upload, browser renderer file, WebWork2 import, or
general Open Problem Library route. The server sends source, path, seed, display policy, and
resolved answer to `/render-api`; the browser receives only the typed PLE presentation envelope
and submits a PLE response. The four reviewed Chapter 1 sources are the current evidence boundary.
See [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md).

iMathAS is a PLE-managed Question Backend, not a file format. Its iMathAS Question Backend Launch
and Result Verification tokens remain server-private; no unverified hosted MyOpenMath import is
accepted.

## Roster CSV import

An authorized course Instructor, or an audited Sysadmin support session, may preview and explicitly
commit UTF-8 CSV at `/api/courses/{course}/roster-imports/preview`. The exact current grammar is:

```csv
email,roster_id
student@example.edu,900123456
```

- The media type is `text/csv`; the body is at most 1 MiB and 500 data rows.
- Headers must be exactly `email,roster_id` in that order.
- Preview normalizes and classifies rows; commit selects preview row numbers with strong revisions
  and idempotency. Raw CSV bytes are not retained after normalized staging.
- `roster_id` is course-scoped matching data, not an account key or authentication credential.

The deferred route follows the ownership rules in
[ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md).

## Implemented CSV exports

### Course totals

`POST /api/courses/{course}/grade-export.csv` also requires an empty body and returns synchronous,
no-store `text/csv; charset=utf-8` attachment data, bounded to 500 active students. It begins with
`metadata` and `student` records and declares `totalPoints` or `weightedCategories` plus the fixed
four-decimal half-away-from-zero rule. Unavailable rows carry a status instead of a score. Durable
audit metadata is PII-free; email and display name exist only in the response.

Route authorization, response headers, and retention are defined in
[API_CONTRACTS.md](API_CONTRACTS.md) and [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md).

## Planned formats and routes

These are explicit release-plan work, not current interfaces:

- Canvas and Blackboard QTI profile export as background jobs with queued status and protected
  downloads (WP-RC6). No profile exporter has shipped.
- A future external QTI-JSONL adapter. PLE Question JSON remains the authoritative internal
  source contract; QTI-JSONL is not a current upload format.
- Broader scored H5P conversion. Current H5P remains ungraded key-free practice; any scored
  conversion requires a separate bounded, evidence-backed adapter contract.

YAML is not an input or output interface. A future human-editing format may compile to canonical
PLE Question JSON, but no YAML schema is accepted today. Generic PG, PGML, WebWork2, Open Problem Library,
LMS roster synchronization, and Canvas/Blackboard export are not current file interfaces.

The release scope and dependency order are maintained in
[release_completion_plan.md](active_plans/active/release_completion_plan.md); current package status is in
[implementation_status.md](active_plans/implementation_status.md).
