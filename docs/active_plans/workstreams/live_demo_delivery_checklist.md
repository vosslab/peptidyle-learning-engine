# Live demo delivery checklist

> **Historical lifecycle record plus current T6 evidence ledger.** This checklist preserves its dated
> delivery evidence and records the current WP-PROF-T6 assignment-workspace evidence state. The
> supported browser owner, catalog, commands, and lane truth are defined by the
> [real-stack browser suite plan](../active/real_stack_browser_suite_plan.md),
> [implementation status](../implementation_status.md), and
> [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md).

## Product contract

The live demo is a fully functional PLE instance, not a fixed walkthrough or read-only
demonstration. It supports the normal Instructor, Student, and Sysadmin perspectives. Its current
public entry is a deployment-gated closed selector for five fixed personas: Elena Instructor, Mary
Student, Jack Student, Avery Student, and Morgan Sysadmin. A selection establishes the ordinary
account session, then visible course selection establishes the ordinary tenant session. The server
continues to own stored roles, memberships, and authorization.

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

## WP-PROF-T6 assignment workspace

The current Instructor workflow follows the assignment as a teaching object. Selecting an assignment
title opens that assignment's Overview. Assignment-local navigation then provides separate Overview,
Questions, Policies, and Student view pages, each with a focused task and current state.

- [x] Questions owns fixed questions, pools, ordering, reuse, and focused content saves.
- [x] Policies owns learner instructions, delivery, lifecycle, and focused policy saves.
- [x] Instructor Student view keeps the Instructor identity and course authority while rendering the
      current assignment through the answer-free learner projection. It is a stable-identity inspection
      surface and creates no run, attempt, submission, receipt, or grade.
- [x] Ordinary demo Student entry remains the graded path. It creates real learner work through the
      visible Student workflow, and the Instructor sees the resulting score and authorized evidence in
      the gradebook after a fresh read.
- [x] Instructor and Sysadmin evidence uses exactly 1280 by 800 CSS pixels (16:10) on a laptop or
      desktop. Student profiles remain variable across the declared laptop, portrait-tablet, iPhone
      Pro, and square profiles.
- [x] Publish and inspect the two current T6 PNG artifacts declared by
      `tests/e2e/browser_screenshot_corpus.json`: `instructor_authoring_assignment_policies` at
      `docs/screenshots/instructor/assignment_workspace/01_assignment_policies.png` and
      `instructor_authoring_student_view` at
      `docs/screenshots/instructor/assignment_workspace/02_student_view.png`.

The two T6 PNGs are published under their corpus-declared paths and included in the Instructor
gallery. The corpus manifest and provenance record remain authoritative for the current screenshot
set.

The current direct-role evidence is independently runnable: `direct_role_entry` selects Morgan,
selects Genetics, proves Sysadmin authorization, and performs generic visible passkey enrollment and
sign-in; `auth_authorization` does the same generic passkey proof for Elena before its ordinary
Instructor and course-boundary checks. The fixed owner retains reset and cleanup responsibility;
the scenarios share neither credentials nor setup state.

This checklist applies the [professor capability architecture plan](../active/professor_capability_architecture_plan.md),
the [implementation plan](../implementation_plan.md), and the current package handoff in
[implementation status](../implementation_status.md). It records delivery work; those documents
remain the scope and acceptance authorities.

## P0: seed only fresh state

- [x] Allocate the `WP-PROF-LD1` schema package and migration through
      [implementation_status.md](../implementation_status.md) before durable install-state work begins.
      `WP-PROF-LD2` accepted immutable migration `2026081809` owns exactly two least-privilege
      execute-only PostgreSQL brokers: safe normal Sysadmin approval-candidate discovery and read-only
      completed live-demo installation-generation lookup.
      Its separately accepted immutable `2026081810` is only the narrow Student pre-tenant account-course
      context retention-boundary repair. Selector, passkey, account, and session data and semantics remain
      non-schema; the generation-read broker is the accepted narrow auth-owned installation-state read.
      `WP-PROF-T3` remains separate and current.
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

The live-demo handoff order was `WP-PROF-LD1` -> `WP-PROF-LD2` -> `WP-PROF-T3`; all three packages
are accepted. [implementation_status.md](../implementation_status.md) owns the active handoff.

- [x] Provide a deployment-controlled selector for five fixed seeded personas: Elena Instructor,
      Mary Student, Jack Student, Avery Student, and Morgan Sysadmin.
      The selector sends only a closed known persona to the server.
- [x] Resolve each selected persona on the server to a known seeded PLE account and create the
      ordinary account session. Derive roles, tenant context, memberships, and authorization from
      persisted PLE state.
- [x] Continue from the ordinary account session through normal course/role selection to the
      ordinary PLE session, cookie, RLS, and capability path.
- [x] Repair the pre-tenant account-course discovery boundary so `ple_auth` returns active Student
      contexts, preserves archived/deleted/started-retention concealment, leaves Instructor behavior
      unchanged, and proves connected Student login.
- [x] Prove each seeded persona enters through normal sessions and only its stored normal role
      capabilities. The selector replaces only passwordless identity verification.
- [x] Directly select Morgan Sysadmin, choose Genetics, and prove ordinary server authorization from
      the selected account's stored role and session state.
- [x] Prove real generic WebAuthn enrollment and authentication for Morgan Sysadmin and Elena
      Instructor inside their respective scenarios: server-issued challenges bind the relying party and
      origin, require the configured user-verification policy, resist replay, and establish the normal
      protected session.
- [x] Prove the seeded Sysadmin exercises full normal Sysadmin workflows, including the ordinary
      instructor-discovery and approval workflow. Preserve the normal Student, Instructor, and
      Sysadmin role boundaries.
- [x] Prove fresh database and object-storage regeneration restores all five seeded personas to the
      baseline account and stored-role state. Each browser scenario starts from that reset baseline and
      performs its own visible passkey work; it does not require credential replay.

Email is unavailable for this proof. Connected evidence uses seeded real accounts and normal
sessions. Fresh database and storage regeneration restores the complete seeded baseline.

## Connected live-authoring proof

Run one ordinary connected HTTPS role journey with normal sessions. Browser evidence and lifecycle
evidence are complementary: the browser journey proves visible role behavior, while lifecycle
evidence proves retained-state and regeneration behavior; they do not require one giant restart
browser scenario.

### Instructor path

- [x] Create and publish a new native or flat problem.
- [x] Record its displayed Question ID from the Library/catalog.
- [x] Create a new course.
- [x] Create an assignment and add that newly issued Question ID.
- [x] Add an existing seeded live-demo Student and confirm normal membership/enrollment.

### Student path

- [x] Select the seeded Student account, enter the course, complete the new assignment, and submit
      answers through the ordinary Student session.
- [x] View the permitted feedback and grades, then repeat the assignment where its policy allows.

### Item-pool continuation (WP-PROF-T5)

- [x] Elena uses visible controls and public Question IDs to create a mixed fixed-plus-pool
      assignment. She configures draw count, points, and ordering while the v1 algorithm label remains
      read-only.
- [x] Elena previews a server-generated pool draw as an ephemeral no-store computation over the
      current assignment revision. Durable learner activity remains unchanged.
- [x] An ordinarily enrolled Student receives the fixed item plus the selected pool item, submits
      both responses, sees permitted deterministic grading and feedback, and resumes the exact issued
      selection.
- [x] A permitted next run follows the selected variation policy. Evidence records the
      policy-correct basis and freezes each issued selection.
- [x] Elena inspects the resulting work and immutable evidence. A structural edit after first issue
      presents the visible new-assignment recovery action and preserves issued work.

### Sysadmin path

- [x] Include a named, explicitly selectable seeded Student account outside the Base Course roster
      and without Instructor authority as the Instructor-approval candidate. Its selector maps to the
      same normal account session before and after approval. Avery remains the same selectable seeded
      account; approval changes only eligibility.
- [x] Use the normal Sysadmin path to discover only the public account reference and display label
      needed for approval.
- [x] Approve that user as an Instructor; prove that Instructor-role approval is Sysadmin-only.
- [x] Have the course Instructor invite the approved account from the Teaching Team. Invitation
      acceptance creates the ordinary course Instructor membership; normal course selection then exposes
      it. Approval alone neither creates that membership nor turns the selector into an Instructor persona.

This preserves the fresh problem -> Question ID -> course -> assignment -> Student path. New
account creation and email enrollment are separate scope. See the
[course routes](../../../crates/server/src/course/routing.rs),
[course list page](../../../src/pages/course_list_page.tsx),
[assignment workspace](../../../src/pages/assignment_workspace/assignment_workspace_live_page.tsx),
[Questions page](../../../src/pages/assignment_workspace/assignment_workspace_questions_page.tsx),
[Policies page](../../../src/pages/assignment_workspace/assignment_workspace_policies_page.tsx),
[Student-view page](../../../src/pages/assignment_workspace/assignment_workspace_student_view_page.tsx), and
[teaching team panel](../../../src/pages/teaching_team_panel.tsx).

## Persistence and regeneration proof

- [x] Start the ordinary stack with fresh volumes and record the regenerated Base Course baseline.
- [x] Complete the connected live-authoring proof with Student, Instructor, and Sysadmin sessions.
- [x] Run a second ordinary start with the same volumes.
- [x] Confirm created data and edited Base Course data remain stable, while project application
      container identities are replaced.
- [x] Confirm the owned image inventory is stable with no dangling or accumulated images and no
      accumulated containers.
- [x] Explicitly regenerate fresh volumes and confirm the baseline returns; prior demo changes are
      intentionally absent from this new instance.

## Final gates

- [x] The production-installer owner keeps non-browser lifecycle evidence in
      `tests/e2e/e2e_live_demo_baseline.py`: fresh installation, retained restart, fresh regeneration,
      and representative interrupted or mixed-state recovery demonstrate lifecycle semantics without
      asserting global record totals.
- [x] Keep installer evidence KISS: pure `base_course_installation` crate tests cover its typed
      request, receipt, recipe, and deterministic convergence; the existing
      `learning-data-access` PostgreSQL live oracle covers schema and lock behavior; the existing
      `tests/e2e/e2e_live_demo_baseline.py` covers the connected full lifecycle. Do not add a second
      product-specific PostgreSQL harness or an exhaustive live matrix.
- [x] The browser-evidence owner keeps the ordinary role journey in
      `tests/playwright/e2e/live_demo.spec.ts`; `local_stack.py acceptance` runs that journey against
      the disposable local stack. It verifies visible normal sessions and role-authorized outcomes,
      including representative recovery, rather than an exhaustive live matrix.
- [x] The canonical `item_pool_delivery` scenario and its two corpus artifacts,
      `item_pool_delivery_pool_preview` and `item_pool_delivery_learner_delivered_pool`, ran through the
      fixed production HTTPS owner. Capture provenance/privacy checks and independent visual approval
      cover those published artifacts; the final aggregate suite proves the material tree.
- [x] The integrator ran this complete Validation suite on the corrected final material tree,
      after the 1809 scope correction. Historical post-repair runtime evidence ran in
      order: `./check_rust.sh` passed; `./check_codebase.sh` passed five checks and 322 Node tests;
      pytest passed 6,017 tests; the baseline E2E and all eight `local_stack.py acceptance` lanes
      passed; and both diff checks passed with no Python bytecode artifacts. The terminal HTTPS
      Playwright journey passed once under `ple-live-demo-browser-d0ff0e97f4ac`; typed cleanup left
      zero labeled containers, volumes, and networks. The 256 MiB `createbuckets` repair received
      independent acceptance. This documentation closeout does not itself prove final-goal completion;
      final-goal completion additionally requires the complete final-material-tree Validation after
      these record edits.

  ```bash
  ./check_rust.sh
  ./check_codebase.sh
  source source_me.sh && python3 -m pytest tests/
  source source_me.sh && python3 tests/e2e/e2e_live_demo_baseline.py
  source source_me.sh && python3 local_stack.py acceptance
  git diff --check
  git diff --cached --check
  ```

- [x] Independent reviewers examined the production-installer boundary, seeded-auth
      boundary, lifecycle evidence, rendered role journey, and the reconciled 1809 scope after the
      package Validation. Their records name scope, environment, criteria, conclusion, limitations, and
      any follow-up. The subsequent documentation closeout does not itself prove final-goal completion:
      final-goal completion additionally requires the complete final-material-tree Validation after these
      record edits.

[TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md) defines the complete Validation suite. A
static browser-test run, readiness probe, API setup, or historical receipt does not substitute for
the connected ordinary-site proof.
