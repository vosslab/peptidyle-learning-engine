# Live demo delivery checklist

## Product contract

The live demo is a fully functional PLE instance, not a fixed walkthrough or read-only
demonstration. It supports the normal Instructor, Student, and Sysadmin perspectives.

The human-approved [LIVE_DEMO_SPEC.md](../../LIVE_DEMO_SPEC.md) is the product authority for this
checklist. This checklist turns that specification into ordered delivery and evidence work.

From the Instructor perspective, people should be able to create courses, create assignments,
create problems, and add students to their courses. From the Student perspective, people should be
able to enter courses, complete assignments, submit answers, view permitted feedback and grades,
and repeat assignments where allowed. From the Sysadmin perspective, the demo provides the full
normal Sysadmin experience, including adding and approving Instructors and other normal Sysadmin
functions. A Sysadmin has no ambient Instructor or course authority unless ordinary PLE contracts
grant it. Course Instructors may invite an already approved Instructor into a course; Instructor-role
approval belongs to the Sysadmin.

Seeded instructors, students, courses, assignments, problems, and student activity are normal live
data. Fresh database and object-storage regeneration restores the baseline. Retained stores
preserve edits and new data across ordinary restarts.

This checklist applies the [professor capability architecture plan](../active/professor_capability_architecture_plan.md),
the [implementation plan](../implementation_plan.md), and the current package handoff in
[implementation status](../implementation_status.md). It records delivery work; those documents
remain the scope and acceptance authorities.

## P0: seed only fresh state

- [x] Allocate the `WP-PROF-LD1` schema package and migration through
  [implementation_status.md](../implementation_status.md) before durable install-state work begins.
  `WP-PROF-LD2` now owns allocated migration `2026081809` only for the least-privilege PostgreSQL
  broker function required for safe Sysadmin approval-candidate discovery. Its claim, passkey, and
  selector work remain non-schema; `WP-PROF-T3` remains separate and non-schema.
- [x] Give each fresh baseline installation a durable `installing`/`complete` state and generation,
  serialize its installer, and bind its PostgreSQL rows and object-storage receipt to that
  installation.
- [x] Prove an interrupted `installing` run retries the same deterministic baseline and exact
  generation-bound receipt. Prove a pre-marker database or mixed PostgreSQL/object-storage state
  fails closed and directs fresh regeneration of both stores.
- [x] Make an ordinary start with retained `complete` volumes run migrations and readiness only. A
  completed start calls no seed writer and performs no storage inspection or equality scan.
- [x] Keep explicit fresh database-and-storage regeneration as the one path that installs baseline
  data.
- [x] Prove that an ordinary retained-volume start neither rewrites nor restores edited Base Course
  accounts, memberships, assignments, problems, activity, or grades.

- [x] Extract the production installer into the focused product crate
  `crates/base-course-installation/`, imported as `base_course_installation`. The crate owns the typed request, receipt, ordinary
  Base Course recipe, and deterministic installation orchestration. `learning-data-access` alone
  owns SQL, the PostgreSQL advisory lock, durable install-state transitions, migrations, and Store
  implementation. `project-tools` is the direct `cargo tools base-course` CLI adapter only. The
  product crate owns the baseline recipe, install-state transitions, and command contract. The
  product crate has no HTTP route or server-start hook.

The host-side `base-course.json` receipt is diagnostics only; the generation-bound storage receipt
is the lifecycle binding. The required repair is the fresh-state boundary above, not an expanded
exact reconciliation. See
[lifecycle.py](../../../local_stack_control/lifecycle.py) and the
  direct `base-course` CLI adapter.

## Test artifact boundary

- [x] Build ordinary `dist/` with HTTP transport and HTTP local credential login only.
- [x] Build browser-test coverage in a separately named browser-test artifact with its own static
  server; that server supplies bytes and SPA fallback only.
- [x] Keep browser-test assets and test-double transport inside the browser-test artifact.
- [x] Add an artifact-graph regression check proving ordinary and local live builds exclude the
  browser-test transport and login modules.

Use the repository fixture policy exactly:

- Fast pytest inputs stay inline.
- File-shaped pytest inputs use `tmp_path`.
- Browser, network, and subprocess flows live in `tests/playwright/` or `tests/e2e/`.
- Existing shipped files are reusable when their real shape is under test.
- New committed shared fixture data requires owner approval.

The browser-test artifact does not create a shared fixture corpus. The installed Base Course remains
production baseline data and its ordinary HTTP journey is the live-demo proof. See
[PYTEST_STYLE.md](../../PYTEST_STYLE.md#fixture-policy).

## Seeded account authentication (WP-PROF-LD2)

`WP-PROF-LD2` starts after `WP-PROF-LD1` has accepted the baseline lifecycle. Its only `WP-RC8`
dependency is the necessary existing account-session/passkey/origin contract: LD2 can implement
and validate the seeded-entry seams against those contracts while unrelated provider, mailbox,
multi-replica, security, and HCI gates remain open. It adds convenient entry for seeded Student
and Instructor accounts while preserving `WP-RC8` as the production authentication owner.

The live-demo handoff order is `WP-PROF-LD1` -> `WP-PROF-LD2`; `WP-PROF-T3` stays separate and
parked until the approved live-demo goal is delivered.

- [ ] Provide a deployment-controlled selector for the seeded Student and Instructor personas.
  The selector sends only a closed known persona to the server.
- [ ] Resolve each selected persona on the server to a known seeded PLE account and create the
  ordinary account session. Derive roles, tenant context, memberships, and authorization from
  persisted PLE state.
- [ ] Continue from the ordinary account session through normal course/role selection to the
  ordinary PLE session, cookie, RLS, and capability path.
- [ ] Prove seeded Student and Instructor selection through normal sessions and their normal role
  capabilities. The selector replaces only passwordless identity verification.
- [ ] Seed an ordinary, unclaimed Sysadmin account. First access completes the normal account
  ownership and passkey-enrollment flow; later access uses the normal Sysadmin authentication and
  session path.
- [ ] Require a deployment-controlled, server-verified ownership proof bound to the configured
  seeded Sysadmin account before its first claim. Browser-supplied account, role, and persona data
  remain insufficient to authorize that claim.
- [ ] Atomically commit successful ownership proof, first passkey enrollment, claimed state, and
  the ordinary account session. One valid claim wins; replayed, invalid, and concurrent attempts
  receive safe results without creating a new role.
- [ ] Prove real WebAuthn enrollment and authentication: server-issued challenges bind the relying
  party and origin, require the configured user-verification policy, resist replay, and establish
  the normal protected session.
- [ ] Prove the seeded Sysadmin exercises full normal Sysadmin workflows, including the ordinary
  instructor-discovery and approval workflow. Preserve the normal Student, Instructor, and
  Sysadmin role boundaries.
- [ ] Prove fresh database and object-storage regeneration restores the seeded Sysadmin to its
  original unclaimed state and invalidates its ownership proof, enrolled passkey, claim, and
  session state.

Email is unavailable for this proof. Connected evidence uses seeded real accounts and normal
sessions. Fresh database and storage regeneration restores the complete seeded baseline.

## Connected live-authoring proof

Run one ordinary-stack proof against the local HTTP stack and retained PostgreSQL/object-storage
volumes, with separate role subsections and normal sessions.

### Instructor path

- [ ] Create and publish a new native or flat problem.
- [ ] Record its displayed Question ID from the Library/catalog.
- [ ] Create a new course.
- [ ] Create an assignment and add that newly issued Question ID.
- [ ] Add an existing seeded live-demo Student and confirm normal membership/enrollment.

### Student path

- [ ] Select the seeded Student account, enter the course, complete the new assignment, and submit
  answers through the ordinary Student session.
- [ ] View the permitted feedback and grades, then repeat the assignment where its policy allows.

### Sysadmin path

- [ ] Include a named, explicitly selectable seeded Student account outside the Base Course roster
  and without Instructor authority as the Instructor-approval candidate. Its selector maps to the
  same normal account session before and after approval. Avery remains the same selectable seeded
  account; approval changes only eligibility.
- [ ] Use the normal Sysadmin path to discover only the public account reference and display label
  needed for approval.
- [ ] Approve that user as an Instructor; prove that Instructor-role approval is Sysadmin-only.
- [ ] Have the course Instructor invite the approved account from the Teaching Team. Invitation
  acceptance creates the ordinary course Instructor membership; normal course selection then exposes
  it. Approval alone neither creates that membership nor turns the selector into an Instructor persona.

This preserves the fresh problem -> Question ID -> course -> assignment -> Student path. New
account creation and email enrollment are separate scope. See the
[course routes](../../../crates/server/src/course/routing.rs),
[course list page](../../../src/pages/course_list_page.tsx),
[workspace editor pages](../../../src/pages/editor_live_pages.tsx),
[assignment editor](../../../src/pages/assignment_editor_live_page.tsx), and
[teaching team panel](../../../src/pages/teaching_team_panel.tsx).

## Persistence and regeneration proof

- [ ] Start the ordinary stack with fresh volumes and record the regenerated Base Course baseline.
- [ ] Complete the connected live-authoring proof with Student, Instructor, and Sysadmin sessions.
- [ ] Run a second ordinary start with the same volumes.
- [ ] Confirm created data and edited Base Course data remain stable, while project application
  container identities are replaced.
- [ ] Confirm the owned image inventory is stable with no dangling or accumulated images and no
  accumulated containers.
- [ ] Explicitly regenerate fresh volumes and confirm the baseline returns; prior demo changes are
  intentionally absent from this new instance.

## Final gates

- [ ] The production-installer owner keeps non-browser lifecycle evidence in
  `tests/e2e/e2e_live_demo_baseline.py`: fresh installation, retained restart, fresh regeneration,
  and representative interrupted or mixed-state recovery demonstrate lifecycle semantics without
  asserting global record totals.
- [ ] Keep installer evidence KISS: pure `base_course_installation` crate tests cover its typed
  request, receipt, recipe, and deterministic convergence; the existing
  `learning-data-access` PostgreSQL live oracle covers schema and lock behavior; the existing
  `tests/e2e/e2e_live_demo_baseline.py` covers the connected full lifecycle. Do not add a second
  product-specific PostgreSQL harness or an exhaustive live matrix.
- [ ] The browser-evidence owner keeps the ordinary role journey in
  `tests/playwright/e2e/live_demo.spec.ts`; `local_stack.py acceptance` runs that journey against
  the disposable local stack. It verifies visible normal sessions and role-authorized outcomes,
  including representative recovery, rather than an exhaustive live matrix.
- [ ] The integrator runs this deduplicated Validation suite on the final material tree, in order:

  ```bash
  ./check_rust.sh
  ./check_codebase.sh
  source source_me.sh && python3 -m pytest tests/
  source source_me.sh && python3 tests/e2e/e2e_live_demo_baseline.py
  source source_me.sh && python3 local_stack.py acceptance
  git diff --check
  git diff --cached --check
  ```

- [ ] The independent reviewer examines the production-installer boundary, seeded-auth boundary,
  lifecycle evidence, and rendered role journey. The review records scope, environment, criteria,
  conclusion, limitations, and any follow-up before acceptance.

[TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md) defines the complete Validation suite. A
static browser-test run, readiness probe, API setup, or historical receipt does not substitute for
the connected ordinary-site proof.
