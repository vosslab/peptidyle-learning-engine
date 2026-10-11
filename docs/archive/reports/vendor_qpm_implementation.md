# Vendored QPM implementation

Source of intent: [vendor_qpm_plan.md](../vendor_qpm_plan.md), with the approved per-question image,
individual-request failure, ordinary Draft-validation, and PLE-only HOTSPOT clarifications.
The later naming clarification uses QPM for PLE-owned integration paths, while preserving
upstream-generated filenames inside the vendored distribution.

The supported browser-import integration is complete under the 2026-10-11 autonomous gate
correction. The original Canvas QTI 1.2 and Blackboard QTI 2.1 successful-import cases were not met;
both engines are writer-only. Independent agent assessment and a fresh actual-WASM check support
the corrected capability gate: the picker excludes them and real exported packages return
`unsupportedDirection`. All 18 vendor hashes still match the earlier browser/full-suite evidence.
Blackboard export ZIP remains a distinct supported reader. No new QTI-reader behavior is claimed.
The new proof is retained in `test-results/qpm-import/ple_qpm_autonomous_gate.log`; earlier live
evidence retains its original dates. No new permanent test was needed for this documentation-only
scope correction; the approved single browser journey remains the durable contract.

## Implemented boundaries

- QPM is vendored from checkout commit `9e4674c81d0381dc5086450db7b66b0cb608b115`.
  `assets/qpm_wasm/source.json` records SHA-256 hashes of all 18 supplied artifacts.
- PLE hashes actual distribution bytes into the shipped asset URL. Only opening Import loads
  QPM's wrapper and WASM. The format picker uses QPM's readable inventory.
- QPM emits Native JSON locally. PLE stores image-free items sequentially with the existing Draft
  client and metadata defaults. Created means saved, with validation in preview and publication.
- Images are detected in each item's display content before creating its Draft. Shared package
  media does not skip unrelated questions. Per-item HTTP and network failures preserve earlier
  successes and do not stop later requests or trigger automatic retries.
- The unused QTI adapter and imported-source helper, exclusive constructors, fixtures, and Cargo
  dependencies are removed. Source searches found no consumers outside that removed code.
- HOTSPOT stays in PLE's GUI. Its image upload, region editor, and correct-region selection are
  explicitly recorded in [DESIGN_DECISIONS.md](../../DESIGN_DECISIONS.md#hotspot-authoring-stays-in-ple).

## Verification

- Vendor refresh: passed. Source commit, all artifact hashes, and the shipped license verified.
- PLE complete debug build: passed.
- Remaining workspace `cargo check --workspace`: passed.
- Focused WASM conversion: all seven BBQ kinds emit Native JSON accepted by PLE's browser decoder.
- Focused media fixture: Blackboard export ZIP preserves four items with one shared image used in
  a prompt and a choice; its two image-free items remain independent.
- PLE final fast checks after the naming follow-up: passed, including 11,063 Python tests, Rust
  workspace tests, TypeScript, and schema checks.
- Permanent production-browser journey: passed. All seven BBQ kinds import and open their saved
  Draft preview through the real Draft API and decoder.
- Focused live checks: passed. Ordinary pages load no QPM wrapper or WASM; Import loads both.
  Converter warnings appear. An HTTP rejection and a network failure between successful saves
  produce individual results, preserve both successes, and issue exactly four requests with no
  retries. A Blackboard export ZIP with four questions and one shared prompt/choice image saves
  both image-free questions and skips the two image-bearing questions.
- Publication: all seven imported kinds published through ordinary metadata and publication
  controls after supplying the required fields; each published Question opened successfully.
- Screenshot capture: passed using `./devel/capture_screenshots.sh --only qpm_import` with a
  temporary scenario. Visually reviewed the import screen, results, and published Question.
  Captures and session logs are retained under ignored `test-results/qpm-import/`.
- Final `all_test.sh`: passed, including all 19 PostgreSQL acceptance groups, installation-data
  provision/replay, and Course Appearance PostgreSQL/MinIO coherence. The owned disposable stacks
  were cleaned up.
- QPM naming follow-up: renamed PLE's directory and refresh script to `assets/qpm_wasm/` and
  `devel/vendor_qpm_wasm.py`. All 18 upstream artifact hashes remain unchanged. Rebuilding produces
  the same `999e2216` browser bundle used for the live journey, focused checks, and screenshots.

The permanent test is [question_import.spec.ts](../../../tests/playwright/e2e/question_import.spec.ts).
All other plan-specific probes and the temporary capture registration were removed after use.

## Discovered limitations

- QPM has no Canvas QTI 1.2 or Blackboard QTI 2.1 reader. Both real exported packages return
  `unsupportedDirection` when read with their registered engine names. Blackboard export ZIP is
  a separate, supported reader. Adding readers changes stable QPM engines and is deferred.
- Inline-image binding and broader HTML rendering remain deferred. Source HTML is preserved.
- QPM currently expects XML-compatible HTML; for example, an unclosed `img` tag yields a source
  warning and omits that source item. PLE displays these converter warnings unchanged.
- No QPM engine or PLE decoder compatibility changes were needed. The plan's conditional QPM
  `cargo test` gate therefore does not apply; PLE's existing decoder accepted every emitted kind.

## Gate maintenance

The first fast run found broken historical links, including pre-existing links to retired
Bloom, Pool-fork, and discussion files. Historical source references now link to verified Git
revisions containing the files, including the QTI code removed by this task. No historical finding
text or runtime behavior was changed. The Markdown-link suite passes (551 checks).

The first full run passed its offline gates but correctly refused live acceptance while the
owned Live Demo was active. The final run passed after browser verification and Demo shutdown.
Its log is retained at `test-results/qpm-import/ple_qpm_all_test_final.log`.
The naming follow-up's first fast run found 10.8 GiB of local Rust artifacts after full acceptance.
With no compiler active, a dry-run followed by `cargo clean -p server_core` reclaimed that package's
rebuildable artifacts. The repeated fast gate passed with `target/` at 9.8 GiB. Its log is retained
at `test-results/qpm-import/ple_qpm_renamed_fast_checks_final.log`.
