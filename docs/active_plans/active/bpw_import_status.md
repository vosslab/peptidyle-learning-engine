# BPW import status

Updated 2026-10-14. This ledger tracks work against [bpw_import_plan.md](bpw_import_plan.md).
Implementation and offline checks are recorded separately from live acceptance. Catalog counts
are temporary evidence from the current BPW checkout, not permanent gates or acceptance totals.
Implementation is complete. The final full compliance and live database gate passed; complete
screenshot-corpus acceptance remains separately deferred.

Follow-up (2026-10-14): PG/PGML import now requests saved-Draft preview diagnostics before
publication, reusing the configured renderer's `/render-api`. Renderer errors stop publication;
PG warnings remain nonfatal. Focused Rust checks and temporary importer probes passed, and the
real renderer exercised valid, warning, and error cases through the PLE adapter. Earlier full
installation receipts predate this additional validation. A sampled BPW PGML failure is recorded
in the deferred findings report; no fresh complete catalog import is claimed for this follow-up.

| Task | Owner | Status | Evidence or dependency |
| --- | --- | --- | --- |
| Public Blueprints | `finish_blueprints` | Specification accepted; quality correction complete | Fresh specification review accepted the Public/Archived implementation. The quality pass found an obsolete `published` watch-notification enum member, corrected in the base schema. Schema generation and style pass. Full live installation succeeded. Screenshot capture passed selected workflows; complete screenshot acceptance remains deferred with the pre-existing privacy-profile mismatch. |
| Native content | `native_content` | Corrections accepted; representative runtime proof passed | Generic Native JSON HTML, CSS, and media receiving/rendering are implemented. Fresh review accepted the support-image position correction. Live publication exposed a redundant prompt limit; the Rust/browser correction passes the full converted catalog. Fresh prompt specification and quality reviews accepted the correction. The real HTTP import completed. Native Student rendering, D8 disclosure, and published current/revision image-binding receipts passed. |
| Importer | `importer_v2` | Corrections accepted; real HTTP import completed | Ordinary HTTP import, direct PG/PGML handling, and BBQ-to-Native-JSON conversion are implemented. Final QPM conversion passed all 225 selected BBQ banks after upstream CSS repair. The full HTTP mock passed with 11,149 Questions, 225 Pools, four Blueprints, and 570 images. The real HTTP import completed all 487 selected sets and created four Blueprints. Demo finalization correction is accepted; the final full live installation succeeded. |
| Installation | `installation_v2` | Full live and content-only installation passed | Host installation invokes the shared importer with private session handoff and revocation. Fresh review accepted the source/tool configuration and wording corrections. The full live HTTP import and Biochemistry adoption succeeded. Release exposed a missing adoption due date; the ordinary-edit correction is accepted. Full live installation and a fresh content-only installation passed. Representative Native browser proof and the final aggregate/live database gate passed. |
| Plan and ledger | `implementation_docs` | Evidence refreshed | This ledger preserves the approved boundaries and separately labels focused, aggregate, catalog, and live evidence. |
| Fresh specification review | Root manager | Prior, prompt, and finalization correction reviews accepted | Blueprint and Native specifications, Native support-position correction, importer/installer configuration corrections, and the prompt correction are accepted. The final screenshot exact-Open locator re-review and demo finalization correction review also accepted their corrections. |
| Quality and integration review | Root manager | Implementation complete; final aggregate/live gate passed | Combined quality review accepted the prior corrections. Fresh aggregate Rust, TypeScript/Node, and 9,062 pytest cases passed. The finalization correction also passes fresh Rust checks and five temporary actual-function HTTP probes. Full live and content-only installations and representative Native browser proof passed. The final aggregate/live database gate passed; full screenshot acceptance is not claimed. |

Implementation owners should restate applicable decisions before relying on them, apply them in
their owned code or contract, and report results with evidence. Escalate conflicts with the approved
plan to the root manager. PG/PGML enters PLE directly as WeBWorK content. BBQ passes through the
configured QPM CLI to Native JSON, then through ordinary PLE HTTP operations.

## Decision record

| ID | Decision | Propagation state |
| --- | --- | --- |
| D1 | Reuse the existing email-code session; keep Python authentication in memory; use a temporary protected installer session and revoke it. | Implemented; importer/installer correction review passed. Full live installation and content-only session-revocation proof passed. |
| D2 | Import PG/PGML directly as WeBWorK content; convert BBQ with the configured QPM CLI to Native JSON and use ordinary PLE HTTP. | Implemented; source accounting and correction review passed. Real HTTP import and representative Native browser proof passed. |
| D3 | Import all seven usable collections. Genetics, Biochemistry, Biotechnology, and Biostatistics are Public Blueprint Courses; the other three collections are Library-only. | Implemented; temporary source probe accounted for the selected sets. Final conversion and the real HTTP import passed; four Blueprints were created. |
| D4 | PLE owns generic Native JSON HTML, CSS, and image binding in its ordinary receiver/rendering path. | Implemented; specification accepted. Representative Native Student and published rendering receipts passed. |
| D5 | Preserve source-relative image names using the existing optional `X-PLE-Source-Relative-Name` upload header and an HTML image-binding vector; leave source content unchanged. | Implemented; focused media checks passed. Real HTTP publication and actual BPW image browser delivery passed. |
| D6 | Use trusted prepare, finalize, and revoke phases. The installer invokes the shared Python command on the host; protect the session over stdin; retain imported content when `--without-live-demo` is used. | Implemented; focused handoff, revocation, and content-only checks passed. Full live and fresh content-only installations passed; the opt-out retained four Blueprints with no fictional Student or Course graph and no active sessions. |
| D7 | Map explicit filenames according to actual source modes; send PG directly. Optional literal `n`/`num_choices` metadata may support editable scoring; report unknown values by default. Do not parse or convert PG through QPM. | Implemented; specification corrections accepted. Catalog conversion, real HTTP import, and representative completed Student Work passed. |
| D8 | Disclose support-hint and feedback images only when the corresponding hint or feedback is actually disclosed. | Implemented; support-position correction accepted. Student D8 runtime receipt passed before/after disclosure, unrelated-account denial, and history image delivery. |

## Evidence log

The root manager supplied these receipts. They describe the checked baseline and do not claim
that every check or the complete screenshot corpus is accepted. Full live installation and
representative runtime results and the passing final aggregate/live database gate are recorded below.

- Aggregate offline checks: `source source_me.sh && ./check_rust.sh` passed before the final small
  corrections; `source source_me.sh && ./check_codebase.sh` passed, including 23 Node tests;
  the full pytest suite passed 9,061 cases after cache cleanup. Later corrections retain separate
  focused review and check receipts.
- Fresh Public Blueprint specification review passed. Its quality review found an obsolete
  `published` watch-notification enum member, corrected at the owning base-schema declaration. Unrelated
  pre-existing history-shape and cursor findings remain outside this import task and are recorded
  in [bpw_import_deferred_findings.md](../reports/bpw_import_deferred_findings.md); those files
  remain untouched by this correction.
- Fresh Native content specification review passed. Its quality review found a support-image
  position off-by-one; the owning SQL correction passes schema style and its fresh correction
  review accepted it. The subsequent Student D8 runtime receipt passed.
- Fresh importer/installer correction review passed 15 temporary cases, an HTTP probe, and a
  canonical edit-number probe. One retained unit check protects the result's stable six fields:
  `blueprints`, `questions`, `pools`, `pointDefaults`, `complete`, and `errors`.
- Fresh importer/installer configuration correction reviews and the combined quality review
  accepted the prior implementation and corrections.
- The source probe selected 487 sets: 189 PGML, 73 PG, and 225 BBQ. All 482 BBQ banks were accounted
  for: 225 selected and 257 superseded by algorithmic sources. These are observations from this
  checkout, not fixed product inventory requirements.
- Eight script-dependent BBQ banks passed conversion through QPM's existing `--html-to-image`
  flag, yielding 370 Questions and 570 images. Sixteen malformed-CSS banks were repaired at the
  owning Biology Problems source and regenerated into BPW with 50 Questions each. The final full
  catalog conversion probe passed all 225 selected banks with zero failures.
- The final QPM probe emitted 10,887 Native Questions: 6,891 single-choice, 1,450 numeric,
  1,269 multiple-answer, 770 fill-in, 256 matching, 150 multi-fill-in, and 101 ordering. Together
  with 262 algorithmic Questions, the expected import is 11,149 Published Questions, 225 Pools,
  and four Blueprint Courses. The full HTTP mock passed with these Question, Pool, and Blueprint
  totals and 570 images. Mock success does not establish live acceptance.
- The actual fresh-stack import failed at set 28, father HARD 9, during publication: HTTP 422,
  `X-Request-ID: k5nhoftsPab89XsVd4G0wA`. The Native prompt exceeded the redundant 65,536-character
  limit while its source remained within the 256 KiB budget. The stack was shut down after failure.
- The small Rust/browser correction removes only the prompt cap and retains nonblank prompt
  validation and the source budget. Temporary catalog checks passed all 225 banks and 10,887
  items through both Rust and browser validation with zero failures. Fresh specification
  and quality reviews accepted the correction. The subsequent real HTTP import completed all
  487 selected sets and created four Blueprints; Biochemistry adoption returned HTTP 201.
- Demo finalization then failed release with HTTP 422 because the adoption had no due date.
  The correction uses an ordinary inline PUT only when the due date is missing, sets the existing
  BCHM 355 term end (`2026-12-11T23:59:00.000`), and uses the returned canonical edit number for
  release. A secondary correction handles top-level workspace entries. Fresh specification and
  quality reviews accepted both corrections; five temporary actual-function HTTP probes passed.
  Fresh full Rust checks passed again (`/tmp/ple_import_finalization_rust.log`).
- Fresh aggregate checks passed Rust, TypeScript/Node, and 9,062 pytest cases. The final live
  database lane was blocked by a held browser lease; this is not a product failure or a database
  acceptance receipt. The final screenshot exact-Open locator re-review accepted its correction.
- Final full live installation succeeded (`/tmp/ple_import_live_final_retry.log`), including
  Biochemistry adoption and representative Student Work preparation.
- The fresh content-only installation passed
  (`/private/tmp/ple_content_only_report_escalated.json`): ordinary HTTP confirmed four Blueprints
  and no Course Instances; SQL corroborated no Students, Sysadmins, authentication emails, or
  active sessions. Cleanup completed. These counts describe the temporary probe, not new gates.
- Representative Native runtime receipts passed with no errors in
  `tests/_temp/native_content_proof/student_d8_runtime_receipt.json` and
  `tests/_temp/native_content_proof/published_runtime_receipt.json`. Student proof covers pedigree
  positioning, MathML, prompt/choice images, actual BPW images, and support-image disclosure.
  Hint/feedback assets returned 404 before disclosure, 200 for their owner after disclosure, and
  404 for an unrelated account; history disclosed images loaded. Published current and selected
  revisions bind images, actual BPW images load, and authored scripts remain inert.
- Corrected ordinary published-detail image registration and the screenshot helpers for the
  imported content. Selected screenshot workflows passed: Course Progress, Due Soon, Completed
  Coursework, and Response Stats at all four viewports. Course Attempt History capture failed on
  the pre-existing privacy-profile mismatch with authorized post-submit Native answers. The
  [deferred findings](../reports/bpw_import_deferred_findings.md) document provenance and scope.
  This partial result is not complete screenshot acceptance and adds no import requirements.
- The final `source source_me.sh && ./launchers/all_test.sh` exited 0
  (`/tmp/ple_import_final_gate.log`). Rust, browser checks including 23 Node tests, and 9,062
  pytest cases passed. The database baseline E2E passed; the disposable PostgreSQL schema
  authority persistence oracle passed; complete live acceptance reported green.
- Documentation integration regenerated schema documentation through its owning generator;
  schema style passed against the current base schema.
  ASCII and Markdown checks passed 2,739 cases.
- Removed the obsolete executable BPW probe sources from `tests/_temp` and the superseded
  Playwright component probe. Ignored runtime receipts, screenshots, and catalog reports remain
  deliberately available as temporary evidence; they are not permanent tests or gates.
- Implementation and the final aggregate/live database gate are complete. Full screenshot
  acceptance is not claimed; the pre-existing capture mismatch is deferred separately. Retained
  temporary verification receipts do not create new product requirements or gates.
