# WP-PROF-S4 Start and Closeout Checklist

## Codebase status snapshot (2026-08-19)

- WP-PROF-S3 and WP-PROF-S5 are accepted and closed; S4 consumes their current verdicts without
  rebuilding policy resolution or entitlement.
- WP-PROF-S4 is accepted on the final material tree. Migration 1805, learner projections, the central
  student route boundary, class-statistics privacy, and the visual/access evidence are complete.
- Multiple prior cuts were rejected until resolved where competing authority paths remained; these were repaired to enforce single-truth ownership for policy, entitlement, and course membership.
- This file preserves the start sequence as implementation history and records the completed closure.

## Permanent visual and access contract

- Use the exact CSS-pixel matrix: 1280 by 800 (16:10), 800 by 1280 (10:16), 393 by 852 (iPhone Pro
  aspect), and 800 by 800 (square). Planning weights are 40%, 30%, 20%, and 10%; they are not test
  quotas.
- Instructor and Sysadmin evidence uses exactly 1280 by 800 CSS pixels (16:10) on a laptop or
  desktop. Student evidence includes an allowed student surface and fail-closed denial of
  instructor-only routes.
- Put committed evidence under `docs/screenshots/` by instructor, student, and the student/access
  boundary; access artifacts use `docs/screenshots/student/access/`.
  `tests/playwright/ui_corpus_manifest.ts` is the sole screenshot ownership authority.
- Pair every access screenshot with no-transport assertions and direct route probes. Pixels alone
  cannot prove authorization. The route boundary is centrally derived and fails closed before
  instructor components or transport mount, including roster and gradebook.
- Email is unavailable. Use local-development credentials or invitations for live evidence without
  claiming email delivery. Fictional deterministic `example.invalid` fixture addresses are allowed,
  while real email and identifying records are prohibited. Keep public and private evidence boundaries
  separate.
- The accepted corpus contains fresh, inspected allowed-student and instructor-route-denial captures
  at all four viewports. Manifest/provenance verification covers 32 artifacts; direct route and
  no-transport tests remain the authorization proof because pixels alone are insufficient.

## Historical start sequence and completed closure

1) Startup lock-in (first 15 min)

- Read the authoritative source of truth files:
  - `docs/active_plans/implementation_plan.md`
  - `docs/active_plans/implementation_status.md`
  - `docs/TEST_EVIDENCE_MODEL.md`
- Confirm that `implementation_status.md` records accepted WP-PROF-S4, immutable migration 1805, and
  the sole handoff to dependency-ready WP-PROF-S6.

2) Scope freeze (next 10 min)

- Write a one-line scope declaration before coding:
  - In scope: disclosure and learner-projection behavior consumption only.
  - Out of scope: S3/S5 resolver changes, entitlement authority rewrites, and migration packages not reserved by S4.
- Reconfirm immutable owners from previous packages:
  - S3 = effective policy/resolution authority.
  - S5 = entitlement and membership authority.

3) Inventory pass (30-45 min)

- Map all disclosure consumers (run list, run detail, summaries, grade-facing learner projections, any retention views).
- Ensure each path reads only through the accepted owners (S3/S5 + designated disclosure projection owners).
- Remove/neutralize any direct legacy policy-timing authority reads; no compatibility fallback behavior.
- Verify the centrally derived role boundary denies every instructor-only route before transport,
  with direct roster and gradebook probes and no-transport assertions under a student session.

4) Patch pass (work loop)

- Keep changes narrow and additive.
- Only touch the minimal files required for S4 projection behavior and evidence.
- If a migration is needed, use only the S4-reserved package and include full contract/fixture updates atomically.

5) Validation sequence (do not skip required gates)

- `./check_rust.sh`
- `./check_codebase.sh`
- `source source_me.sh && python3 -m pytest tests/test_markdown_links.py tests/test_ascii_compliance.py tests/test_whitespace.py`
- Any required focused conformance suites for S4 paths.
- `git diff --check`
- `git diff --cached --check`
- Playwright or live E2E suites only if S4 scope touches UI/network surfaces required by the test model.
- Fresh student/access visual capture and human inspection at the required matrix; the accepted run
  produced and inspected all eight access artifacts.

6) Closure steps

- Update:
  - `docs/active_plans/implementation_status.md`
  - `docs/CHANGELOG.md`
  - any touched contract/architecture/docs files.
- Record command output and any skipped gates (must be justified).

7) Completion acceptance before end of day

- All required gates pass on final material tree.
- No competing authority path introduced.
- Run required review(s); no blocking findings.
- If blocked, stop and document exact blocker + owner before proceeding.
