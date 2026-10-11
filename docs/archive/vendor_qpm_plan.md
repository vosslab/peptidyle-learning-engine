# Plan: incorporate Rust QPM Native JSON into PLE through vendored browser WASM

## Execution status

Completed for supported bank imports. On 2026-10-11 the user directed autonomous plan completion
through manager/subagent assessments and automated behavior checks. An independent review and a
fresh actual-WASM probe reconciled milestone 4 with QPM's available readers.

The original requirement that Canvas QTI 1.2 and Blackboard QTI 2.1 ZIPs import successfully was
not met: both engines are writers only. The corrected gate below verifies that unsupported
formats are absent from the picker and return `unsupportedDirection`, while readable Blackboard
export ZIP imports exercise package handling. These outcomes are distinct, not a claim that QTI
imports succeeded. New readers remain deferred in [TODO.md](../TODO.md#future-product-capabilities).
See the [implementation report](reports/vendor_qpm_implementation.md) for evidence and limitations.

## Goal and context

An Instructor selects a question-bank file. QPM's WASM converts it to Native JSON in the
browser. PLE's existing Draft API stores each converted item as a Draft, which then previews
and publishes through the existing workflow.

Ownership:
- QPM owns conversion and validation of the source package. It knows nothing about PLE
  storage.
- PLE's existing decoder is the one acceptance gate. PLE owns persistence and publication.

Why browser WASM:
- It reuses BPW's established QPM distribution.
- Parsing runs on the instructor's machine.
- The PLE server gains no QPM dependency.

Evidence:
- QPM's `packages/qti-wasm` exports `initialize`, `formats` and `convert`. With
  `outputFormat: "ple_native_json"`, `convert` returns a directory artifact of
  `item_NNNNN.json` plus `media/*`.
- BPW's `devel/vendor_qti_wasm.py` vendors the built `dist/`.
- PLE:
  - The decoder is `crates/adapters/ple/src/question_json*`.
  - The Draft API is `crates/server/src/authoring.rs:86-118`.
  - `pipeline/build.mjs` copies WASM into `dist/wasm/<hash>/`.
  - `crates/adapters/qti` is never called by the server.

## Scope control

- **Required:** exactly the outcomes in the five milestones below.
- **Permitted when needed for compatibility:**
  - edits to QPM's `crates/qti-engines/src/ple_native_json/**`, with QPM tests;
  - edits to PLE's existing Native JSON decoder.
- **Deferred:** image binding beyond what PLE's existing Draft image API already does, and
  any other limitation found during the work. Record each in `docs/TODO.md`.
- **Outside this plan; record demonstrated needs as separate future work:**
  - new backend endpoints or server routes;
  - database or schema changes;
  - new abstractions or shared helpers;
  - any user-facing behavior beyond milestone 2, such as retry, bulk edit, metadata mapping,
    import history or scheduling;
  - changes to stable QPM outside the `ple_native_json` engine.

## Autonomous execution

The manager and fresh implementation, test, and review agents own every milestone. Resolve
engineering choices from the authorities, existing APIs, and observable behavior; independent
review replaces human sign-off. Use local disposable services and seeded Accounts for browser and
publication checks. External credentials, production deployment, and human visual review are
outside completion. Failed behavior checks return to the owning implementation boundary for repair
and focused re-verification. Retain earlier successful imports and do not add retry workflows.
If a desired format has no reader, verify the capability and clear unsupported result, preserve
working imports, and record separate upstream work. No milestone waits for a human response.

## Milestones

1. **Vendor QPM.**
   - Copy BPW's `devel/vendor_qti_wasm.py` as PLE's `devel/vendor_qpm_wasm.py` and change only:
     - the output path, to tracked `assets/qpm_wasm/`;
     - the `-q/--qpm-root` flag;
     - recording the QPM commit in `source.json`.
   - In `pipeline/build.mjs`, copy `qti_wasm_bg.wasm` into a content-addressed directory, the
     same way the `ple_bridge` copy works. Load the QPM wrapper only from the import screen.
   - Exclude `assets/qpm_wasm/` in `eslint.config.local.js` and `.prettierignore.local`.
2. **Import questions.**
   - Add one Import action next to Draft creation. The screen has a file picker and a format
     picker filled from `formats()`.
   - On submit:
     1. Call `convert` with `outputFormat: "ple_native_json"` and mixed kinds enabled.
     2. Show QPM warnings.
     3. Create one Draft per `item_*.json` with the existing Draft API.
   - Detect images in each question's display content. Until inline-image binding exists, skip
     affected questions before creating their Drafts. Shared media does not skip unrelated items.
   - Finish with one list showing each item's result: created (linking the Draft), rejected
     (with the available server message), skipped (reason "contains images"), or failed (the
     individual request error). Continue after each failure, preserve earlier successes, and do
     not retry automatically. Created means saved; Native JSON validation remains in ordinary
     preview and publication.
   - Fix Native JSON mismatches found here within the permitted scope, judged against
     `docs/QUESTION_SPECS/NATIVE_JSON_SPEC.md`.
3. **Remove redundant code.** Confirm `crates/adapters/qti` and
   `crates/adapters/ple/src/question_json/imported.rs` have no consumers outside their own
   tests. Then remove them and their `Cargo.toml` entries.
4. **Verify.**
   - A BBQ bank covering all seven response kinds imports into Drafts that preview and
     publish.
   - A readable Blackboard export ZIP verifies per-question image skips alongside successful
     image-free Drafts. Focused HTTP/network probes verify individual rejected and failed results
     and later successful saves without retries.
   - Check the registry for Canvas QTI and Blackboard QTI. Writer-only formats stay out of the
     picker; actual exported packages return `unsupportedDirection` on read. If a future refresh
     adds a reader, verify its Draft/preview behavior before claiming support. Reader development
     remains outside this integration's scope.
   - Ordinary PLE pages do not request QPM's WASM.
5. **Document.**
   - `docs/DESIGN_DECISIONS.md`: one entry, "QPM is a vendored browser WASM distribution; QPM
     converts, PLE's decoder accepts".
   - Refresh procedure in `docs/INSTALL.md` and `docs/USAGE.md`.
   - Update `docs/TODO.md:305-324` with the deferrals.
   - Record the transport in `NATIVE_JSON_CONVERTER_HANDOFF_SPEC.md`.
   - Update `docs/FILE_STRUCTURE.md` and `docs/CHANGELOG.md`.

## File scope

- New: `devel/vendor_qpm_wasm.py`, `assets/qpm_wasm/**`, and the import screen module in
  `src/`.
- Edited: `pipeline/build.mjs`, `eslint.config.local.js`, `.prettierignore.local`, and the
  Draft-creation screen (to add the Import action).
- Compatibility fixes only: QPM `crates/qti-engines/src/ple_native_json/**` and the PLE
  Native JSON decoder.
- Removed: `crates/adapters/qti/`, `crates/adapters/ple/src/question_json/imported.rs`, and
  the related `Cargo.toml` entries.
- Docs listed in milestone 5.

## Completion checks

- `source source_me.sh && python3 devel/vendor_qpm_wasm.py` refreshes `assets/qpm_wasm/` and
  records the QPM commit.
- One permanent Playwright journey in `tests/playwright/`: a fixed BBQ bank (one item per
  response kind) imports into Drafts that preview.
  - It protects the QPM-to-PLE contract across refreshes.
  - On failure: fix the QPM engine or the PLE decoder against `NATIVE_JSON_SPEC.md`.
- `./devel/capture_screenshots.sh` (run once) shows the import screen, the results list, and a
  published imported Draft.
- QPM's `cargo test` passes after engine changes.
- In PLE, `source ./source_me.sh && ./launchers/run_fast_checks.sh` passes. Run
  `./launchers/all_test.sh` before closeout.
