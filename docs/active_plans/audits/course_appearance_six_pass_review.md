# Course Appearance code audit

Date: 2026-09-09. Scope: the complete working-tree change from `HEAD`, including staged,
unstaged, and untracked files, for
[cryptic_foraging_hennessy.md](../../archive/cryptic_foraging_hennessy.md).

Six fresh independent reviewers covered plan compliance, tests, implementation style,
documentation, legacy/dead code, and comments. All six returned findings. The coordinator
deduplicated them and checked the disputed claims against current source and the named authorities.

Status: changes requested. The earlier passing aggregate and completed-plan receipt do not
establish the missing behavior below. This audit supersedes the earlier unconditional completion
assessment for these specific requirements.

## Open findings

### High: theme response loses banner

The theme mutation in
[course_appearance.rs](../../../crates/server/src/course_appearance.rs) returns
`CourseAppearanceView { theme, banner: None }` on success. The page in
[course_appearance_page.tsx](../../../src/pages/course_appearance_page.tsx) replaces its complete
cached appearance with that response. Saving a theme after a banner therefore hides the saved
banner until reload, although the Banner record remains stored. Both the Plan and Legacy auditors
independently found this violation of independent saves.

- Owner: Course Appearance server implementation.
- Implementation: return the persisted theme with the currently authorized Banner in the mutation
  response, using the same complete appearance composition as the read boundary. Keep the scalar
  mutation independent of Banner storage and define the post-write read-failure behavior explicitly.
- Success: changing a theme on a Course with a saved banner preserves that banner in both the HTTP
  response and the rendered route cache without requiring reload.
- Validation: add the banner-first/theme-second order to the existing server and connected browser
  behavior evidence, then run the aggregate gate. The existing connected journey saves theme first,
  so its pass does not cover this order.

### High: expired uploads lack cleanup

Upload staging persists an object and work record, but expiry is checked only when attempting
promotion in
[2026090903_course_banner_renditions.sql](../../../schemas/migrations/2026090903_course_banner_renditions.sql).
No executable worker consumes expired, unpromoted upload work. A client that stages successfully and
then abandons promotion leaves the temporary object indefinitely. The accepted
[course_appearance_banner_storage_and_sizing.md](../decisions/course_appearance_banner_storage_and_sizing.md)
requires expired uploads to enter the persisted cleanup path.

- Owner: Course Banner persistence and worker lifecycle implementation.
- Implementation: connect expired, unpromoted uploads to bounded, claimed cleanup execution using
  the existing typed job/storage-check/cleanup-manifest boundaries. Recheck eligibility under the
  claim so cleanup cannot remove an active or successfully promoted object. Preserve uncertain
  deletion outcomes as actionable repair work.
- Success: an expired abandoned upload is deleted, or a failed/uncertain deletion has durable repair
  state reachable by an executable consumer; current and unexpired assets remain intact.
- Validation: extend the disposable PostgreSQL/MinIO saga acceptance with synthetic expiry,
  repeat execution, deletion failure, and promotion/cleanup eligibility cases. No real-time wait or
  human step is needed.

### Medium: production accessibility evidence

[course_appearance_m9_accessibility_evidence.mjs](../../../tests/playwright/course_appearance_m9_accessibility_evidence.mjs)
builds and serves a component harness with source CSS. It provides useful keyboard, axe, and
reflow evidence for the mounted component, but does not exercise the shipped application bundle
and route composition required by the Playwright load model. The existing production-browser
propagation journey does not perform those accessibility checks.

- Owner: production-browser Course Appearance evidence.
- Implementation: exercise keyboard theme selection, banner input, non-color state, supported
  reflow profiles, and axe on the served production Appearance route through the existing browser
  suite owner. Retain narrowly useful harness transitions as component evidence.
- Success: the production route passes the named accessibility behaviors with no serious or
  critical axe violations.
- Validation: run the registered production-browser scenario from a fresh disposable stack and
  record the exact evidence boundary.

## Corrected test classification

The initial missing-runner finding is withdrawn following the user's permanent-test clarification.
Not every implementation check warrants permanent suite membership. The documented direct commands
already own the focused browser checks; their absence from the fast/service aggregate is intentional.
No new runner is needed merely to retain rebuild evidence.

The checklist in [PYTEST_STYLE.md](../../PYTEST_STYLE.md) distinguishes meaningful behavior from
inventories, defaults, and implementation snapshots. One-time palette/control inventories, preview
height and Ribbon geometry snapshots, SQL-body strings and schema/default inventories, and trivial
response-constructor tests are removed. Existing receipts retain their implementation-time results.
Narrow state-transition, disclosure, authorization, and failure-ordering checks remain in their
appropriate fast, browser, or disposable-service lanes. The classification and retention rationale
are recorded in [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md).

The three findings above remain open. Classifying evidence does not repair the theme response,
add expired-upload cleanup, or establish production-route accessibility.

## Low-risk corrections

The audit cleanup addresses these concrete issues without changing the open lifecycle and mutation
findings:

- Add the required visible Course entry and Course card labels to pending and saved banner previews.
- Use `CourseBannerRendition::dimensions()` for server normalization instead of duplicate literals.
- Replace execution milestone labels in changed permanent code comments and diagnostic strings with
  descriptive capability names; retain established file/symbol identifiers.
- Add missing PostgreSQL Banner Store rustdoc and explain saved-preview URL ownership.
- Correct the obsolete upload-reference comment to name independent Banner promotion.
- Refresh route, architecture, file-ownership, data-classification, concurrency, and visual-review
  documentation to describe the implemented boundaries accurately.

## Review judgments

- The original review retained preview-height assertions because sizing was documented. The user's
  later instruction supersedes that lifetime decision: exact sizing and rebuild snapshots are
  one-time evidence and are removed from the retained checks.
- API-map omissions are documentation findings, not evidence that the executable routes themselves
  are missing authorization. The durable maps were corrected during audit cleanup.
- The style reviewer suggested archiving the accepted sizing decision and specimen. The current
  decision directory is an allowed location for decision records, and the record supplies detailed
  evidence linked by the durable Design Decisions entry. A further relocation is organizational
  cleanup, not a product blocker; it is left unchanged by this audit.
- Historical changelog milestone references remain because the approved plan expressly requested
  milestone receipts. Permanent code comments no longer rely on those execution labels.
- No new fragile pytest, unused dependency, or disabled Banner saga acceptance was found. The
  ignored Rust database test is explicitly executed by the disposable service owner.

## Validation record

Before the audit cleanup, `source source_me.sh && ./launchers/all_test.sh` passed with exit code 0:
generated contracts and fixtures, Rust checks/Clippy/tests/doctests/Wasm, frontend checks, 6,090
Python tests, PostgreSQL fresh/no-op/catalog/restricted-role/iMathAS acceptance, and MinIO/banner
saga acceptance. Both disposable service lanes cleaned their resources.

Post-cleanup validation is recorded in [CHANGELOG.md](../../CHANGELOG.md). A passing test command
does not close the three open findings above without their stated additional behavior evidence.
