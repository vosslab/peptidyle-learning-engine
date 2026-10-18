# Frontend ownership cleanup plan

## Objective

Place shared collection controls and feature workflows beside their owners, then align current
architecture documentation with the resulting source tree. Preserve route shells, API boundaries,
and per-record behavior.

## Ownership boundaries

- Shared collection controls use the `data_sheet_*` names: `record_fields`, `page_controls`,
  `page_sizes`, `collection_state`, `interactions`, `filter_controls` and its CSS,
  `sort_controls` and its CSS, and `column_visibility`.
- The shared four controls use DataSheet names. Shared search types move from `search_session` to
  `search_definition`.
- Per-record item, content, action, and reorder behavior remains with its record owner.
- Blueprint workflow helpers and collection move into the existing
  `features/blueprint_course/` feature. Route shells remain in `pages/`.
- Library workflow and helpers move into `features/question_library/`; the Library, My Questions,
  and My Pools route shells remain in `pages/`.
- The API layer and its contracts remain unchanged.

## Milestones

1. **Name shared components.** Move and rename shared collection files and their CSS. Preserve
   DataSheet component naming and keep per-record behavior with record owners.
   - Owner: shared DataSheet implementation owner.
   - Success: imports resolve, component names identify DataSheet ownership, and per-record files
     remain with their existing owners.
   - Check: focused TypeScript, formatting, and import/reference checks.
   - Status: implementation, SPEC, and QUALITY reviews are accepted.

2. **Move Blueprint discovery.** Place Blueprint-owned search definitions, fields, result helpers,
   and collection assembly in `features/blueprint_course/`. Keep actual route shells in `pages/`.
   - Owner: Blueprint feature implementation owner.
   - Success: feature owns its discovery workflow; page route compositions own PageFrame and route
     identity.
   - Check: focused source checks and the existing relevant application checks.
   - Status: files moved; focused TypeScript, ESLint, and Prettier checks pass. SPEC and QUALITY
     reviews accept page-owned PageFrame and route identity.

3. **Move Library workflows.** Place Library discovery, My Questions, and My Pools workflow helpers
   in `features/question_library/`. Keep their route shells in `pages/`.
   - Owner: Question Library feature implementation owner.
   - Success: feature owns Library workflow logic; the four route compositions own PageFrame and
     route identity for Library Search, Browse, My Questions, and My Pools.
   - Check: focused source checks, existing relevant application checks, and independent review.
   - Status: files moved; TypeScript, lint, and formatting pass. SPEC and QUALITY reviews accept
     PageFrame and route identity in all four page compositions.

4. **Reconcile imports and docs.** Update imports, current architecture/file-structure/search docs,
   relevant live links, and the changelog. Correct links to moved files when they describe current
   behavior; preserve historical prose and mark a historical link retired only where needed.
   - Owner: documentation and integration owner.
   - Success: current documentation points to final paths, historical records remain historical,
     and no API boundary changed.
   - Check: Markdown link and guidance-format checks, ASCII/source whitespace checks, and diff
     review.
   - Status: current architecture links are reconciled; focused Markdown link, guidance-format,
     ASCII, and diff checks pass. Final canonical fast checks pass (9,427 tests plus Rust,
     TypeScript, lint, and formatting). Maintained Blueprint and Library runtime capture passes;
     final integration review accepts the integrated source, logs, and artifacts.

## Acceptance

Complete the focused checks for each milestone, run the repository's quick compliance checks after
integration, and obtain independent implementation/specification and quality review. Record review
outcomes and any remaining limits in this plan and the changelog. No milestone requires human
approval.

## Review follow-up

SPEC review found that `LibraryWorkspace` still selected page identity through its inner dynamic
class. The correction now leaves PageFrame and route identity in the four existing route
compositions, and fresh SPEC and QUALITY rereviews accept it.

QUALITY review also found PageFrame ownership remaining in Blueprint list/detail workspaces. The
correction lifts those frames into page route compositions, and QUALITY rereview accepts it. Keep
the reactive return link with its Blueprint domain navigation; do not add callbacks only to preserve
exact markup placement.

## Acceptance evidence

- SPEC and QUALITY reviews accept shared naming, feature ownership, and route PageFrame ownership.
- Canonical fast checks pass: 9,427 tests plus Rust, TypeScript, lint, and formatting.
- Runtime capture passes for the maintained `instructor_public_blueprint_search` and
  `instructor_library_results` journeys (`/tmp/ple_ownership_library_public_capture.log`) and the
  `instructor_blueprint` journey (`/tmp/ple_ownership_blueprint_capture_final.log`). Eleven WebP
  captures remain in ignored `tests/_temp/ownership_verification/instructor/`; no screenshot was
  published as a tracked artifact.
- The Human Guidance checklist's global `--consistency` check continues to report pre-existing
  source/generated drift (1,336 Human Guidance bullets versus 1,261 checklist bullets). The moved
  Blueprint owner paths are synchronized through its maintained source part and generated output.
- Final integration review accepts the integrated source, logs, and artifacts. All milestones are
  complete; the known checklist source/generated drift remains documented above.
