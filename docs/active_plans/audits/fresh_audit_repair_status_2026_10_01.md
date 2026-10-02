# Fresh audit repair status

Status: repair and required verification are complete; ready for human review. The canonical Rust
gate passed workspace checks, all-feature Clippy, tests, doctests, and WebAssembly checks. The
TypeScript lane and all 531 Node tests passed (`/private/tmp/ple_repair_fast_final.log`). The final
full Python run passed 10,058 tests with nine warnings in 8.29 seconds
(`/private/tmp/ple_fast_pytest.log`). These receipts came from separate verification steps; they
were not one aggregate command. The earlier Python link and generated-target cache failures were corrected,
and the target directory was reduced to 1.2 GB within its limit. The final code-quality changes
were confined to existing tests, fixture data, helper placement, and documentation: two brittle
SSR label/status tests and their helper (about 130 lines) were removed while retaining 13 controller
tests; two brittle SQL source-string gates were removed; a Rust fixture initializer was corrected
for Clippy; and existing Pool GET/PUT token assertions were aligned to tokens 4/5 without changing
behavior. This records the October 1 audit and deeper follow-up. The latest `check_codebase.sh`
passed typecheck, lint, format, and all 531 Node tests, including the final getter-backed save-status
fix. The save-state correction also passed independent review and fresh runtime proof
at 06:01-06:03 UTC on October 2, restoring the saved receipt and Question 1 across tablet, phone,
and square sessions with privacy checkpoints intact. Schema and default Rust checks passed.
The full saved-response SQL runner passed all seven receipts; separate
installed-PostgreSQL probes passed F2's repeated-account, floor-advancement, and anonymous-retention cases. The latest-
feedback overfetch and both shortcut Course-override bugs are fixed, and eight rebuilt
Student-navigation staging captures passed. The final `--fresh` capture published all 246 screenshots and passed its focused storage budget at 17.52 GB;
the existing verifier passed for the manifest, receipt, and current image files, and the owned stack
stopped cleanly. These screenshot publication and visual review results remain accepted. The
manifest and receipt were updated October 2 at 01:10:08 CDT. Representative
review covered saved response on laptop, phone matching, and Sysadmin Accounts. See the
[screenshot atlas](../../SCREENSHOT_ATLAS.md) and
[capture receipt](../../screenshots/current_capture_receipt.json). The two temporary
classification-proof files are absent from the worktree. F7's independent review and focused
browser checks are accepted. Per-item evidence is recorded below.

Sources: [fresh audit](fresh_codebase_audit_2026_10_01.md), [deeper review](human_guidance_deeper_code_review_2026_10_01.md),
[coordinator coverage](fresh_codebase_audit_2026_10_01_coordinator.md), and per-area ledgers linked
from the fresh audit.

## Findings

| ID | Current evidence and repair scope | Dependencies / acceptance | Status |
|---|---|---|---|
| F1 Pool mutable-state concurrency | `question_pool_support.sql` and `question_pool_search_metadata.sql` now carry a distinct advancing version for mutable metadata/provenance while preserving member-list version semantics. | Shared design decision covers support, search metadata, and provenance replacement. Full saved-response SQL runner passed all receipts. | Source, independent review, and installed SQL acceptance passed. |
| F2 Shared statistics disclosure | `QuestionUsageTotals::into_shared_statistics` now gates each metric on its contributing cohort and suppresses dependent aggregates together. | Independent static review accepted the capped candidate/UPSERT floor and monotonicity design without retaining identities. PostgreSQL proofs passed the same-account/revision case (one graded plus four blank observations: issued 5, blank 4, answered 1, correct 1), floor advancement (5 to 9 observations with floor 5), and anonymous retention after unrelease assessment removed private receipts while aggregate totals and issued/correct floors stayed unchanged. Rollback scratch was clean. | Source, independent review, projection checks, and installed SQL privacy acceptance passed. |
| F3 No-effect Watch materializer | Removed the unused worker/store/grant/state path; recipient snapshots and inbox remain. | Reference removal and schema-style checks passed; final fast gate passed. | Source repair accepted; focused/schema checks passed. |
| F4 Brittle guidance gates | Narrowed checklist and visual/source-specific gates to concrete behavior; archived checklist inputs remain documented as active inputs. | Focused retained Pool hide/restore browser check passed 1/1; profile menu, preview resize, and four Instructor cases passed. | Source repair and focused checks complete; final fast gate passed. |
| F5 Discipline fulfillment | Fulfillment now uses one Sysadmin-authorized server command and database transaction with existing normalization/creation logic. | Disposable PostgreSQL proof forced resolution failure and verified rollback; success and replay created exactly one Discipline, and an active Instructor was denied. | Transactional PostgreSQL proof passed; final fast gate passed. |
| F6 Collection continuation | Starred Questions and Sysadmin Course lists now use cursor contracts aligned across SQL/domain/store/API/UI. | Independent review accepted; Node3 and Cargo checks passed. | Source repair and focused checks accepted; final fast gate passed. |
| F7 Visible internal UUIDs | Attempt, Watch, and Blueprint navigation now keep selection in app state or public-identity-scoped routes while retaining read authorization. | TypeScript, focused navigation tests, and route-scope E2E passed (8/8); independent final review accepted with no concrete defects. The full screenshot corpus was regenerated and passed static verification. | Source, focused browser proof, independent review, and screenshot publication accepted. |
| F8 Native text-answer invariants and editor loop | Browser and Rust validators now share Student response-length semantics; the multi-fill answer-entry path terminates at short limits. | Focused Node12 and Rust checks passed. | Source repair and focused checks accepted; final fast gate passed. |
| F9 Membership helper privilege claim | Follow-up traced the calls after `SET LOCAL ROLE ple_api_owner`, which has the required table SELECT and RLS policy. The alleged privilege defect is unsupported. | Do not change helper to SECURITY DEFINER. Unused EXECUTE grants are a separate cleanup only if independently demonstrated. | Withdrawn; no repair. |
| F10 Angelman content accuracy | Corrected canonical and active topic 01 Genetics Question content to maternal UBE3A loss of function, with manifest/source hash attribution aligned. | Pilot content validator passed for 2 chapters, 8 reviewed Questions, and 4 adapted PGML sources. | Content repair verified; no publication performed. |
| D1 Pool import membership intent (deeper-review addition) | The expected source membership version now travels from the reviewed Picker selection through the fork command and is checked under the source lock. | Full saved-response SQL runner passed all receipts, including the same-count membership-change stale-import case. | Source, independent review, and installed SQL acceptance passed. |
| D2 Classification parent context (deeper-review addition) | Async create/offer continuations are bound to their originating parent/context generation; stale UI effects are discarded while created vocabulary remains. | One-time browser proof passed stale create, stale offer creation, and stale association responses after parent change. Temporary proof files were removed. | Production fix accepted; focused browser proof passed. |

## Refinements

These are bounded review refinements, not equivalent to the concrete F findings. Preserve each item
in scope accounting; resolve by implementing, explicitly deferring with reason, or closing from
current evidence. "No evidence" is not an implementation claim.

| ID | Current evidence and repair/decision scope | Dependencies / acceptance | Status |
|---|---|---|---|
| R1 Installation-data gate | Removed the mutable-fixture Live Demo oracle from ordinary installation; stable installation invariants remain in apply. | Fixture checks remain part of fixture provisioning/acceptance. | Source repair accepted; final fast gate passed. |
| R2 Scoring drafts | Scoring now preserves the local draft/error and prevents Save from silently retaining the old score. | Temporary browser proof passed; independent review accepted the draft deletion and Save lifecycle. Proof files were removed after validation. | Source, focused browser proof, and independent review complete; final fast gate passed. |
| R3 Statistics teaching prose | Corrected chi-square decision wording and the p-value definition in canonical and active adapted content. | Pilot content validator passed for 2 chapters, 8 reviewed Questions, and 4 adapted PGML sources. | Content repair verified; no publication performed. |
| R4 Blueprint test fixture identity | Corrected the three Blueprint fixture Accounts to use distinct primary keys. The SQL oracle's unrelease fixture now uses a distinct checksum-derived Student ID rather than reusing `U00000009` for both Instructor and Student. | Fixture identity fixes were exercised by the full saved-response SQL runner; all seven receipts passed. | Source repair and fixture SQL acceptance complete. |
| R5 Curriculum publication replay | Both ordinary and parameterized publication paths now use immutable source checksum for replay identity, independent of mutable metadata. | One-time unit proof passed for both paths; temporary proof files were removed. | Source repair and focused proof complete; final fast gate passed. |
| R6 Pool provenance storage and authority | Consolidated one-to-one license/source persistence while preserving ordered authorship; no shared-Instructor restriction was added. | Source and independent review accepted. Full saved-response SQL runner passed all receipts. | Source, independent review, and installed SQL acceptance passed. |
| R7 Legacy duration contract | Base duration now uses whole minutes across schema/domain/editor; accommodation timing stays separate. Three unrelease fixture limits now use 5400 seconds (90 minutes), a valid whole-minute value; this changes fixture data, not the product rule. | Source and focused checks passed; the full saved-response SQL runner exercised the corrected fixture and all seven receipts passed. Final screenshot corpus passed publication and static verification. | Source, fixture SQL, and screenshot publication accepted. |
| R8 Legacy publication collision mapping | Removed the adapter's primary-key-error fallback after verifying the producer emits canonical `QP001`. | Producer trace and checks passed. | Source repair accepted; final fast gate passed. |
| R9 Unused StudentWork aggregate | `StudentWork::collect` had no production consumer and validated only some claimed relationships. Removed the public aggregate and its model-only tests; retained the actual Student Work record types and identifiers. | Consumer trace found no workflow needing a combined in-memory aggregate. | Complete; source-level only. |
| R10 Author JavaScript dependency authority | Removed external `script` resource syntax and the empty Rust/TS CDN allowlists. The closed `rdkit` author-library registry remains for local PLE-served content. | Existing source shape denies unknown resource kinds; author-library registry remains supported. | Complete; source-level only. |
| R11 Warm-build recovery | Build output is prepared before publication with directory bind-mount semantics accounted for. | Normal unshimmed production build passed; forced prepare failure preserved the published index hash and cleaned staging output. Independent review accepted. | Source and focused build acceptance passed; final fast gate passed. |
| R12 Unavailable timestamps | Settings-load failure now uses an explicit unavailable timestamp representation. | Focused formatter error regression passed. | Source repair and focused regression passed; final fast gate passed. |
| R13 Star mutation errors | Star mutations now show local actionable errors at the owning component. | Local star error source and Node checks passed. | Source repair and focused checks passed; final fast gate passed. |
| R14 Point-value bound reuse | Client validation now shares the existing domain point-value bound. | Focused source checks passed. | Source repair accepted; final fast gate passed. |
| R15 Identity types across Rust boundary | `ContentDisciplineRequest.requested_by_account_id` now remains `question_model::AccountId` through the store projection; the Postgres decoder validates the canonical ID and the HTTP seam serializes it to a string. | SQL `ple_data.account_id` and existing `requestedByAccountId` JSON remain the authorities. | Complete; source-level only. |
| R16 Saved-response oracle ownership | Split the former oracle into six SQL includes while retaining all seven receipts. | Corrected the active-Instructor fixture identity and gave the unrelease fixture Student a distinct checksum-derived ID after include 03 exposed a duplicate Instructor/Student `U00000009`. The minimal RLS policy repair for active-Instructor `FOR UPDATE` was independently accepted. The full saved-response SQL runner then passed all seven receipts, including the intended include-04 coupling to the include-03 replacement. | Source organization and full SQL runner acceptance complete. |
| R17 Duplicate schema comments | Removed 507 duplicate schema comments at their canonical source. | Schema docs regenerated and schema style check passed. | Source and schema checks complete; final fast gate passed. |
| R18 Screenshot captions | Corrected current captions at the screenshot manifest owner. | Final `--fresh` capture published 246 images; manifest/receipt/current-file checks passed, and representative saved-response, phone-matching, and Sysadmin Accounts images were inspected. | Capture and static corpus acceptance passed; representative visual review passed. |
| R19 `tools/rmpycache.sh` arguments | Quoted path handling now supports whitespace. | Temporary whitespace-path proof and `bash -n` passed; temporary proof files were removed. | Source repair and focused proof complete; final fast gate passed. |
| R20 Genetics manifest descriptions | Corrected manifest descriptions and attribution hashes for canonical and active PGML sources. | Pilot content validator passed for 2 chapters, 8 reviewed Questions, and 4 adapted PGML sources. | Content repair verified; no publication performed. |

### Deeper-review gate and maintenance refinements

| ID | Scope and disposition | Status |
|---|---|---|
| G1 Decoder/browser lane | Separated local decoder/client checks into Node and approval interaction into the browser lane. | Source organization complete; login/email product readiness remains deferred; final fast gate passed. |
| G2 Saved-response oracle scope | Also recorded as R16; split the oracle into six SQL includes while retaining all seven receipts. | Source organization and full SQL runner acceptance complete; all seven receipts passed. |
| G3 Database evidence labels | Updated documentation to label dispatched database evidence and standalone Assessment E2E entry points accurately. | Documentation repair accepted; final fast gate passed. |
| G4 TODO route claims | Updated TODO to distinguish existing personal-library routes from their remaining runtime/capture acceptance. | Documentation repair accepted; final fast gate passed. |
| G5 Archived checklist inputs | Documented `docs/archive/audits/hg_checklist_parts/` as active generator inputs and aligned the generator/input ownership. | Documentation/source repair complete; final fast gate passed. |

The deeper review also confirms three evidence boundaries: changed source does not equal runtime
acceptance; the proposed StudentWork loader is not warranted without a consumer; and no speculative
security, workflow, or compatibility mechanism should be introduced. F9 remains withdrawn.
