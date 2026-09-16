# Architecture And Implementation Changes

## Current heading reconciliation

Current verbatim Human Guidance coverage is 998 bullets: 449 verified, 499 open
(490 owning), and 50 N/A, at SHA256
`e81d5bb0a63cfb7d3ca5f4f34287e5155dc9d20b0b91cb856fdbefa1ef6fa82b`.
The generated checklist owns occurrence status and current first-owner pointers. All nine part
gates, identity diff, and consistency pass for this snapshot. Unchanged scoring, timing,
Blueprint, terminal-Attempt, and bounded MATCH evidence is retained. Product compliance remains
unfinished; these gates establish inventory fidelity, not acceptance of the open requirements.

Part 01 now owns Product vocabulary and glossary, including its five topical subheadings;
new product definitions remain open absent independently accepted evidence. Part 03 owns Profile
avatar interface, Student avatars, and Instructor and Sysadmin Profile images. Account-creation
avatar persistence has a bounded source/SQL receipt, not deployed gallery/upload/cropping or
all-location acceptance. Part 06 owns the shared Content classification requirements; Part 07
owns Library metadata, Part 08 Course classification, and Part 09 Assessment classification.
The shared system requires exactly one Discipline per content object, optional narrower levels,
and a Subject associated with one or more Sysadmin-managed Disciplines. The prior single-parent
four-table foundation receipt does not satisfy this latest association shape or establish commands,
normalization, content attachments, hierarchical selection, or discovery. Those gaps remain open.
KISS/design constraints are audited N/A where not independently closable, but still bind reviews.

Accepted R-4 desktop/phone terminal receipts hide active navigation and visibly label three
no-response records Unanswered, incorrect `0 / 1`; the four exact MATCH pairs remain correct
`1 / 1`, total `1 / 4`. Native diagnostic `AZA01TD` / `R-5` follow-up saved/reloaded FIB, MA,
MULTI-FIB, NUM, and ORDER, then submitted the whole Attempt: four correct `1 / 1` responses,
deliberately partial-reordered ORDER incorrect `0 / 1`, total `4 / 5`. Practice-default permitted
correct answers are displayed separately from retained responses. The supplied ledgers and
`submitted-review-1280.png` / `submitted-review-390.png` under
`/private/tmp/ple-student-types-proof/` are bounded receipts, not all-eight-type, complete keyboard,
touch, or contrast acceptance. HOTSPOT and WeBWorK coverage remain open. Earlier topical
inventories/correction IDs and superseded contradictions below are historical provenance.

## Scope

This is a fresh topical implementation-audit inventory. It collects currently open
Human Guidance checklist records relevant to architecture and implementation. The bullet text is
copied verbatim from the generated checklist, and its source location is recorded beside it.
This report does not establish exhaustive or disjoint topical coverage; the checklist remains the
authority for each record's status.

The authoritative exhaustive record is the
[generated checklist](../../audits/human_guidance_implementation_checklist.md).

## Storage composition receipt

The object-storage cutover removes the direct `aws-config`/`aws.rs` fallback.
Server composition accepts only explicit `disposable-local` MinIO configuration;
the API/worker and publisher use separate configured credentials. The retained
S3-compatible SDK graph follows the existing latest-first dependency policy and
adds no dependency pin. This receipt changes no checklist status.

Focused offline evidence passed: a fresh root `cargo test -p server_core --lib`
ran 99 tests; `objects` with the `s3` feature ran 38 library tests, three
conformance tests, and one archive test. The isolated
`minio_object_store_conforms` run passed once against unique loopback MinIO.

Container-composition evidence is separate from that object-store conformance
result: the owned MinIO used a current locally available image with pull policy
`never`, a read-only root, a data tmpfs, and zero persistent volumes. Its health
check passed. Ordinary exact `stop --rm` cleanup removed the owned container and
temporary secrets; no owned container or temporary secret remained, and the
shared stack was untouched. This is not PLE HTTP, full-cloud, cloud IAM, or
production-deployment evidence.

## Topical inventory

### Development principles -- Codebase development rules

- PLE is pre-production with no users. Fix the design directly rather than preserving legacy behavior.
  - Source: `docs/HUMAN_GUIDANCE.md:46`

- PLE is pre-production with no users or durable production data. Improve the design directly.
  - Source: `docs/HUMAN_GUIDANCE.md:50`

- Every source file should stay below 1000 lines. Split complete capabilities into focused modules.
  - Source: `docs/HUMAN_GUIDANCE.md:45`

- Use readable `snake_case` whenever possible; see [NAMING_CONVENTIONS.md](/docs/NAMING_CONVENTIONS.md) for details.
  - Source: `docs/HUMAN_GUIDANCE.md:51`

- Adaptability should be a focus so the software can evolve as requirements and insights change.
  - Source: `docs/HUMAN_GUIDANCE.md:52`

- Cargo, Node, and PyPI dependencies should use the latest versions to include security fixes.
  - Source: `docs/HUMAN_GUIDANCE.md:53`

- If an interface is measured as too slow, consider moving the slow code to Rust/WebAssembly.
  - Source: `docs/HUMAN_GUIDANCE.md:54`

- Do not create or leave placeholder database tables, states, APIs, workers, or compatibility scaffolding before the feature has an approved product design.
  - Source: `docs/HUMAN_GUIDANCE.md:55`

### Development principles -- PLE development rules

- The polished PLE Live Demo is the top priority; see [LIVE_DEMO_SPEC.md](/docs/LIVE_DEMO_SPEC.md).
  - Source: `docs/HUMAN_GUIDANCE.md:65`

Latest Student response distinction rewrite: live HG requires visually distinct current Question,
saved-response status and keyboard focus, plus response-effect labels distinguishing Save/Clear/
change from whole Coursework submission. Both rows are open; earlier navigation styling receipts
are retained as partial proof, not blanket acceptance of native response controls/actions.
