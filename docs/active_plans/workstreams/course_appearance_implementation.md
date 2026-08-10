# Course appearance implementation handoff

> **Historical workstream record.** This accepted package is retained as implementation evidence,
> not current task direction. Current authority is the [release completion plan](../active/release_completion_plan.md)
> and [implementation status](../implementation_status.md).

## Status

WP-CA1 through WP-CA7 and release package WP-RC1 are complete and accepted on 2026-08-09. The
implemented capability is one revisioned, instructor-managed appearance per course: one of 15
reviewed themes and at most one normalized banner shown only at course entry. The next dependency is
WP-RC2 production-seam closure.

The shared worktree remains mixed staged, unstaged, and untracked for owner review. This handoff did
not stage, reset, delete, or overwrite unrelated work.

## Delivered ownership

| Owner                   | Files                                                                                                                                                                                                           | Working behavior                                                                                                                                                                                                   | Success condition                                                                                                                                                               |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Browser-safe contract   | `crates/question_model/src/course_appearance.rs`, generated API types, `src/route_contract.ts`                                                                                                                  | Closed 15-theme ID, Grass default, positive decimal revision, alternative-text union, and atomic banner mutation                                                                                                   | Rust, generated TypeScript, decoder, mock, route, SQL, and theme registry accept the same closed values                                                                         |
| Object contract         | `crates/objects/`, `crates/learning-data-access/src/asset_delivery.rs`                                                                                                                                          | Tenant/course-bound temporary candidate and protected current-banner keys with no public signing path                                                                                                              | Candidate is non-signable; current delivery requires Store authorization and exposes no physical key                                                                            |
| Persistence and RLS     | `schemas/migrations/2026080907_course_appearance.sql`, `crates/learning-data-access/src/{course_appearance,in_memory/course_appearance,postgres/course_appearance}.rs`                                          | Transactional default creation, persisted authority, CAS, bytes-first promotion, exact-current delivery, bounded two-phase cleanup, and database-enforced same-tenant/course current pointer                       | Memory/PostgreSQL conformance passes; `ple_app` cannot write a pointer owned by another course                                                                                  |
| Server and image safety | `crates/server/src/course_appearance.rs`, `crates/server/src/course_appearance/image.rs`                                                                                                                        | No-store GET, author-only bounded raster upload, 1200 by 328 WebP normalization, strong-ETag PUT, current-only delivery, and bounded best-effort claim/delete/complete cleanup after successful appearance traffic | Hostile formats and roles refuse; stale and storage failures preserve the prior appearance; expired temporary and superseded bytes are reclaimed without deleting current bytes |
| Solid theme scope       | `src/features/course_appearance/course_theme_*`, `src/features/course_appearance/theme_catalog.ts`, course-owned pages                                                                                          | One authorized course projection themes course entry, assignment overview, run attempt, run summary, assignment editor, gradebook, and settings; navigation clears the old scope                                   | All seven surfaces render the expected course ID/theme; global pages and semantic status colors remain unchanged; unknown IDs fail closed                                       |
| Instructor settings     | `src/features/course_appearance/course_appearance_*`, production API client/decoder/runtime seams                                                                                                               | Native radio themes, exact wide/narrow previews, decorative/informative alternative text, one save action, conflict reload, replacement/removal, responsive recovery states                                        | Keyboard-only selection/upload/save/reload/replace/remove works; local draft state survives every recoverable failure; students issue no settings mutation transport            |
| Learner identity        | `src/features/course_appearance/course_entry_identity.tsx`, `src/pages/course_assignments_page.tsx`                                                                                                             | Course title stays text; one optional informative or decorative banner appears only at entry                                                                                                                       | No empty frame without a banner and no banner on assignment, run, summary, editor, gradebook, settings, or global pages                                                         |
| Acceptance              | `tests/test_course_appearance_settings.mjs`, `tests/test_course_theme_scope.mjs`, `tests/playwright/course_appearance*.ts`, `tests/playwright/course_theme_scope.spec.ts`, `tests/e2e/e2e_course_appearance.sh` | Behavior, accessibility, visual, PostgreSQL, RLS, MinIO, and combined cleanup oracles                                                                                                                              | Every command below passes and independent reviewers report no P0/P1/P2                                                                                                         |

## Accepted behavior

- In scope: 15 measured biome/habitat themes with Grass as default; exactly one course-entry banner;
  exact 1200 by 328 centered WebP normalization; keyboard-complete authoring; decorative or
  informative alternative text; revision conflicts with preserved local state and explicit reload;
  responsive and forced-color layouts; current-pointer protected delivery; 60-minute candidate and
  delivery ceilings; tenant-owned bounded object cleanup; rendered contrast and palette-dedup
  evidence.
- Out of scope: arbitrary colors, CSS, fonts, menu layouts, per-page themes, multiple banners, manual
  crop/edit tools, and a general media library. The version succeeds without them because the closed
  catalog and one derivative supply useful course identity while keeping contrast, layout, and
  lifecycle behavior reviewable.
- Out of scope: SVG, animated GIF/WebP, and unbounded source images. The version succeeds with
  JPEG/PNG/static WebP because those inert raster inputs cover ordinary banners and can be decoded,
  resized, stripped, and allocation-bounded safely.
- Out of scope: learner appearance editing. The version succeeds with instructor/administrator
  ownership because course identity is shared instructional configuration, while every learner gets
  the same read-only projection.
- Out of scope for WP-RC1: institutional OIDC, managed deployment, legal certification, object-store
  inventory reconciliation, and a fixed deletion service-level objective. The feature itself is
  complete under the implemented session, PostgreSQL, and S3 contracts: expiry is authoritative at
  60 minutes, active appearance traffic performs bounded idempotent cleanup, and the release plan's
  separately owned platform packages cover production activation without changing appearance
  semantics.

## Validation evidence

| Command                                                                                                         | Result and what it proves                                                                                                                                                                                                                                                                                                         |
| --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cargo test -p server_core course_appearance --lib`                                                             | PASS: six ordinary tests, with the two environment-dependent live tests intentionally ignored; proves image, authorization, mutation, current-pointer, route cleanup, and failure behavior                                                                                                                                        |
| `npx playwright test tests/playwright/course_theme_scope.spec.ts`                                               | PASS: 4/4; visibly proves all seven named course surfaces, entry-only banner behavior, cross-course/global cleanup, and rendered contrast                                                                                                                                                                                         |
| `tests/e2e/e2e_course_appearance.sh`                                                                            | PASS against isolated PostgreSQL and MinIO; migrations migrate/verify, real-role PostgreSQL/RLS/CAS/current-pointer negative probe, MinIO conformance, combined PostgreSQL claim to MinIO delete to completion, idempotency, current-object preservation, route upload/promotion/delivery/supersession, and exact project cleanup |
| `./check_codebase.sh`                                                                                           | PASS: 11/11 checks, including 183 Node tests, generated types/fixtures, both TypeScript checks, ESLint, Prettier, crate boundaries, rustfmt, strict workspace Clippy, Rust unit/integration/doc tests, and Wasm allowlist                                                                                                         |
| `./run_playwright_tests.sh --build`                                                                             | PASS: rebuilt Rust, Wasm, generated types, fixtures, and bundle `a1bc4362`; 62 passed and one opt-in visual generator skipped                                                                                                                                                                                                     |
| `PLE_CAPTURE_COURSE_APPEARANCE_VISUALS=1 npx playwright test tests/playwright/course_appearance_visual.spec.ts` | PASS: 1/1; regenerated every artifact below and rechecked rendered contrast plus OKLab differentiation                                                                                                                                                                                                                            |
| `source source_me.sh && python3 -m pytest -q tests/`                                                            | PASS: 1,743 repository-owned tests                                                                                                                                                                                                                                                                                                |
| `git diff --check` and `git diff --cached --check`                                                              | PASS: unstaged and staged whitespace checks are clean                                                                                                                                                                                                                                                                             |

The first final visual invocation was blocked before test execution by the restricted macOS Chromium
Mach-port sandbox. The exact command was rerun with browser-process permission and passed; this was
an environment launch restriction, not a product/test failure.

## Visual artifacts

The generated directory is intentionally ignored; the reproducible generator and these digests make
the reviewed evidence auditable without committing large binaries.

| Artifact                                                    | SHA-256                                                            |
| ----------------------------------------------------------- | ------------------------------------------------------------------ |
| `generated/ui/course_appearance/palette_metrics.json`       | `032a2eb1d86c92c41dc18ea2e2d01d2f1b86240655b6d12330a62032ef3ba1f4` |
| `generated/ui/course_appearance/settings_1920.png`          | `c16522942e52f529c5d9d2680406cb027b7a4ae0f168b42e7edf4a66183865e7` |
| `generated/ui/course_appearance/settings_320.png`           | `fed662c78e6dc8e1d6038237295cb960880c534618c733e4de81d4f599e81f52` |
| `generated/ui/course_appearance/settings_480.png`           | `4d248e2b66a4db6e66572010c717fbceda6285d39ff22d7ebd29c3461d3f3d9d` |
| `generated/ui/course_appearance/settings_768.png`           | `7f9c500780d1683252faa08163134ddeaf7fa3e91f5dd7a50a83eeb55fc7af4a` |
| `generated/ui/course_appearance/settings_forced_colors.png` | `afaf73e4122f7b308720e06b0beaca28c551069c210fc9d0320e3b0af07c0c24` |
| `generated/ui/course_appearance/theme_contact_sheet.png`    | `fa0530554160fd70e4444e6d9ae4d1ae9b9ad5257095a383a37dc7df01c03dc3` |

All normal-text pairs meet the 5.5:1 project target. Focus, selected outlines, and boundaries meet
3:1 against adjacent rendered surfaces. The closest role-matched pair, Coral reef and Salt marsh,
has mean OKLab distance 8.16 and is retained because its secondary and accent roles remain visibly
distinct; no redundant theme pair was found.

## Independent review

Three reviewers who changed none of the package files completed focused read-only review after the
final remedies:

- HCI/color/accessibility: PASS with no P0/P1/P2; keyboard behavior, alt choices, entry-only banner,
  forced colors, 320/480/768/1920 layouts, computed contrast, and differentiation were verified.
- Persistence/object/security: PASS with no P0/P1/P2 after the combined PostgreSQL/MinIO cleanup
  oracle and database current-pointer trigger/negative probe were added.
- Plan/route completeness: PASS with no P0/P1/P2 after the built browser traversed and asserted the
  assignment editor, gradebook, and settings in addition to the existing learner surfaces.

No implementation placeholder, empty artifact, fake test, disabled production path, or unresolved
course-appearance scope decision remains.
