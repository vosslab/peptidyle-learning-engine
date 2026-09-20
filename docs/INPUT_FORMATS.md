# Input and exchange formats

This page is the file-I/O index for PLE. It distinguishes source-level adapter formats and
retained future contracts from interfaces exposed by the current server. The current route table,
including implemented authoring, roster, delivery, and invitation-export boundaries, is
[API_CONTRACTS.md](API_CONTRACTS.md). This page does not replace the linked schema, adapter, or
API contract.

## Browser and server boundary

The browser may receive answer-free question presentation, safe Question-import reports, and export
status or downloadable artifacts. It never receives answer keys, expected values, hidden correct
choices, private rubrics, grading code, provider credentials, raw provider results, Object
Addresses, or source archives. The complete allowlist and privacy boundary are in
[API_CONTRACTS.md](API_CONTRACTS.md).

Authorized server operations and Question Backends may handle private payloads inside their
own boundaries.
Private source bytes, Answer Keys, Question Feedback, Question Answer Explanations,
and format-specific Question Grading Input remain in their owning adapter or
object-store boundary;
they are not browser formats merely because an Instructor can initiate the operation.

## Live-demo operator input

The supported front door is `./launchers/run_live_demo.sh` with `start`, `open`, and `stop`. `--open` is a
compatible shorthand for `start --open`, while `start --open` creates and opens a fresh demo. `--headless`
remains an accepted explicit spelling of the default non-opening behavior. It uses a fixed disposable
target and does not accept a caller-selected project, identity, environment, SMTP configuration, or
skip-build option. See [USAGE.md](USAGE.md) and
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md).

The lower-level lifecycle receives a private ASCII `NAME=value` owner manifest closed to `OWNER`,
`PROJECT`, `ENV_FILE`, and `CAPABILITY_FILE`; `PROFILE` is required for the live-demo-browser
owner. The referenced files are current-user-owned regular files with mode `0600`. The generated
`runtime.yaml` record is operational evidence, not an authoring or browser-submitted format.

## Source-level authoring and import formats

| Format                 | Surface and media type                                                      | Implemented boundary                                                                                                                                                                     | Owner                                                                                                                                           |
| ---------------------- | --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| PLE Question JSON      | Private PLE Question JSON route; `application/vnd.peptidyle.question+json`  | One answer-bearing document with the closed eight Question Types: MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT; maximum 256 KiB                                                | PLE Question JSON adapter source (currently `crates/adapters/ple/src/question_json.rs`), [QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) |
| Canvas QTI 1.2 ZIP     | Deferred private QTI profile route; exact `application/zip`; maximum 32 MiB | Strict `canvas-qti-1.2-static-single-choice/v1` profile. Unsupported semantics refuse without loss; archive, answers, mappings, and QTI Import Package Checksum evidence stay private    | [crates/adapters/qti/src/profiles/canvas.rs](../crates/adapters/qti/src/profiles/canvas.rs)                                                     |
| Blackboard QTI 2.1 ZIP | Deferred private QTI profile route; exact `application/zip`; maximum 32 MiB | Strict `blackboard-qti-2.1-static-single-choice-pool/v1` profile. Unsupported semantics refuse without loss; browser reports are answer-free                                             | [crates/adapters/qti/src/profiles/blackboard.rs](../crates/adapters/qti/src/profiles/blackboard.rs)                                             |

QTI conversion produces one complete PLE Question JSON Draft Question through
the shared authoring contract. Workspace Import separately retains the original
archive, private mapping evidence, choice maps, and QTI Import Checksums. The
ordinary PLE Question Backend later produces the answer-free Question Presentation. See
[CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md) and [QUESTION_MODEL.md](QUESTION_MODEL.md).

## Private server source

PLE can publish a private immutable PG or PGML Question Source through a Source Object ID to the configured external
`webwork-pg-renderer`. The author declares the educational Question Type on the Published Question
Revision; PLE uses that immutable metadata for labeling and discovery, never by inspecting renderer
controls. The server sends source bytes, source path, seed, display and embed policy, PLE origin,
and asset-base parameters to `/render-api`. The browser receives an authenticated exact
backend-owned document through PLE and saves a bounded opaque canonical ordered-pair Student
Response. WeBWorK owns the HTML, controls, response interpretation, and grading. The local
Chapter 1 sources provide representative connected evidence; they do not restrict the integration
to four sources or control shapes, and they do not claim Open Problem Library breadth. See
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md).

iMathAS is a PLE-managed Question Backend, not a file format. Its iMathAS Question Backend Launch
and Result Verification tokens remain server-private; no unverified hosted MyOpenMath import is
accepted.

## Roster CSV import contract

The current server can preview and commit UTF-8 CSV at
`POST /api/course-instances/{course_instance_id}/roster`. An authorized Course Instructor uses roster import
to bulk add Students to a Course Instance. Student removal remains an individual operation; PLE
does not provide bulk Student removal. The accepted grammar is:

```csv
email,roster_id
student@example.edu,900123456
```

- The media type is `text/csv`; the body is at most 1 MiB and 500 data rows.
- Headers must be exactly `email,roster_id` in that order.
- Preview normalizes and classifies rows; commit selects preview row numbers against the current
  import state and its concurrency control. Raw CSV bytes are not retained after normalized
  staging.
- `roster_id` is course-scoped matching data, not an account key or authentication credential.

Roster import adds or reconnects Student Course relationships according to
[ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md). It does not replace an existing roster or provide a
bulk-removal path.

## Pilot grade export

Pilot grade export is a direct CSV or TSV download of point-based Assessment scores. It does not
apply Grade Categories, weighted categories, a Course Grade Scheme, Course percentage calculations,
LMS-specific mappings, synchronization, or a queued export lifecycle. The exact authorized route,
columns, and FERPA-safe download contract remain implementation work until the export is built.

## Planned formats and routes

These are retained future-contract work, not current interfaces:

- QTI export and archival interchange are product intent. No particular Canvas or Blackboard
  exporter profile, job model, queued status, or protected download workflow is approved or
  shipped by this page.
- A future external QTI-JSONL adapter. PLE Question JSON remains the authoritative internal
  source contract; QTI-JSONL is not a current upload format.
- H5P delivery is blocked on the product decision recorded in
  [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md#future-h5p-delivery-is-a-blocked-isolated-lumi-runtime):
  supported content types and pinned libraries, terminal xAPI/score semantics, and the role of
  scoreless activities. PLE currently has no H5P import, source, or runtime interface.

YAML is not a generic input or output interface. The checked-in pilot Question Set manifest is a
controlled source input owned and validated by
[pilot_content.rs](../crates/project-tools/src/pilot_content.rs); the generated private
`runtime.yaml` is a controlled runtime handoff owned and validated by
[runtime_manifest.py](../local_stack_control/runtime_manifest.py). No generic user-facing YAML
authoring or interchange schema is accepted today. A future human-editing format may compile to
canonical PLE Question JSON. Generic PG, PGML, WebWork2, Open Problem Library, LMS roster
synchronization, and Canvas/Blackboard export are not current file interfaces.

Durable release direction and unfinished-work routing are maintained in
[ROADMAP.md](ROADMAP.md) and [TODO.md](TODO.md).
