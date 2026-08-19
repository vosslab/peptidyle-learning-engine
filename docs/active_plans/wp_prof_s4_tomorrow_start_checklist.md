# WP-PROF-S4 Tomorrow Start Checklist

## Codebase status snapshot (2026-08-19)

- WP-PROF-S3 is accepted and closed in active-plan/docs truth (legacy timing contract removed).
- S3 1804 migration + normalized effective-policy receipts/current-pointer behavior is complete, with PG baseline pass evidence already recorded in prior handoff notes.
- WP-PROF-S5 entitlement/membership redesign has broad in-memory/PG/store coverage and consumer cutover work largely complete, with the main remaining coordination work now to verify any final S5 acceptance/closeout handoff details if not already done in `implementation_status.md`.
- Multiple prior cuts were rejected until resolved where competing authority paths remained; these were repaired to enforce single-truth ownership for policy, entitlement, and course membership.
- No new edits have been run on this turn; this file is a restart packet only.

## Tomorrow start sequence

1) Startup lock-in (first 15 min)

- Read the authoritative source of truth files:
  - `docs/active_plans/implementation_plan.md`
  - `docs/active_plans/implementation_status.md`
  - `docs/TEST_EVIDENCE_MODEL.md`
- Confirm that WP-PROF-S4 is the active package and that `implementation_status.md` has the correct current/immutable package order and handoff chain.

2) Scope freeze (next 10 min)

- Write a one-line scope declaration before coding:
  - In scope: disclosure and learner-projection behavior consumption only.
  - Out of scope: S3/S5 resolver changes, entitlement authority rewrites, and migration packages not reserved by S4.
- Reconfirm immutable owners from previous packages:
  - S3 = effective policy/resolution authority.
  - S5 = entitlement and membership authority.

3) Inventory pass (30–45 min)

- Map all disclosure consumers (run list, run detail, summaries, grade-facing learner projections, any retention views).
- Ensure each path reads only through the accepted owners (S3/S5 + designated disclosure projection owners).
- Remove/neutralize any direct legacy policy-timing authority reads; no compatibility fallback behavior.

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
