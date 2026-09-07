# Test evidence model

PLE uses evidence that matches the claim being made. Fast checks protect narrow
logic; a restored canonical browser suite will prove visible product behavior;
service oracles prove the service boundaries that a browser cannot distinguish.
This document classifies that evidence. Each bounded work item owns its exact command list;
[ROADMAP.md](ROADMAP.md) owns durable release acceptance.

Read [PYTEST_STYLE.md](PYTEST_STYLE.md) before adding a Python test and
[PLAYWRIGHT_TEST_STYLE.md](PLAYWRIGHT_TEST_STYLE.md) before changing a browser
test. [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) defines the disposable live-demo
baseline used by the browser suite.

## Validation test suite

Every work package names its complete Validation suite before completion. A
goal is complete only when every required gate is green on the final material
tree, including required live gates and independent review. `SKIP`, unrun, or
unavailable required gates are not green.

The repository aggregate front door is `./launchers/all_test.sh`. It fails fast through
these four gates, in this order:

```bash
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest tests/
source source_me.sh && python3 local_stack.py acceptance
```

The Rust gate precedes the codebase gate because it owns generated TypeScript
inputs consumed by the latter. `local_stack.py acceptance` currently runs its
two declared real-service lanes: the disposable PostgreSQL schema, authority,
and persistence oracle, and the Course Appearance PostgreSQL and MinIO
coherence oracle. It does not invoke a browser suite.

The complete canonical production-browser path remains a release-blocking M19
requirement. M5 supplies one owned live-browser scenario for its Question
Library task, but a passing aggregate or M5 scenario does not establish the
remaining Instructor, Student, Sysadmin, visual, or serial-workflow acceptance.

Run the full named suite again after any material change that affects a gate.
When a plan requires repeat-run or cleanup evidence, rerun all four gates on
the final material tree in the listed order; the second run is evidence only
when it reports its own result and the required cleanup state.

Record commands, results, environment assumptions, one-time evidence, and
intentional optional skips in the package handoff and
[CHANGELOG.md](CHANGELOG.md). Do not report a goal complete while a required
gate is red.

## Evidence classes

- Permanent behavior, contract, or security tests are fast, deterministic
  gates for maintained local behavior. They do not prove a dependent real
  service.
- Permanent architecture or hygiene gates protect durable engineering rules.
  They do not prove a user workflow.
- One-time implementation probes answer a narrow investigation or
  reconstruction question. They do not protect behavior from regression.
- Disposable acceptance proves its named boundary in a declared real-stack
  environment. It does not prove every deployment or provider.
- Independent review evaluates a stated artifact and criterion. It does not
  make every later change correct.

A report identifies its class, exact claim, and environment. One evidence class
does not gain the scope of another because it uses similar data or code.

## Account hardening and optional passkey evidence

Create Instructor Account has permanent behavior coverage for Active Sysadmin
actor binding, denial and rollback without partial Account, Authentication Email,
or audit state, immutable Product Role, session-role derivation, and Account
State revocation. The connected PostgreSQL migration acceptance is separate
one-time mechanism evidence for the fresh schema, forced RLS, grants, narrow
audit writer, and immutable qualified event relation. Neither category proves a
browser teaching workflow.

The passkey lane is explicitly deferred. Its schema and typed-contract
foundations are not accepted passkey behavior, and passkey setup, ceremony,
management, real-stack, browser, and manual-passkey evidence do not join a
baseline-green claim. The deferred lane cannot invalidate ordinary health,
session/logout, or seeded-demo evidence.

Seeded-demo transport and UI conformance tests permanently protect the closed
persona response shape and bounded degraded status. They do not replace a
real-stack or visible-browser acceptance claim.

### Permanent-test admission

Before a check becomes part of a permanent test lane, it must protect a behavior
that can plausibly regress, have a stable contract independent of incidental
names or file layout, produce a meaningful result, and run offline,
deterministically, without sleeps, random values, current-time dependence, or
real service/CLI calls. It writes only to test-owned temporary storage and is
small enough for its owning lane. A check that merely inventories current
source, counts artifacts, confirms an implementation choice, or records a
migration snapshot is one-time closure evidence instead. Keep that evidence in
the implementation handoff or acceptance receipt, then remove the probe when
the investigation is complete. When in doubt, remove the test.

The current permanent suite contains callable unit, contract, security, and
hygiene behavior checks, plus its declared real-service gates. The restored
production-browser owner will be a separate required release gate. The suite
does not preserve a superseded browser application's source inventory or a
dated screenshot-path inventory as a regression contract.

## Focused unit evidence

Keep permanent tests small, deterministic, and close to the behavior they
protect. Python, Node, and Rust unit or conformance tests own decoder,
serialization, strict transport, failure mapping, validation, and other narrow
logic. They may use inline fake values or isolated dependencies when those are
part of the contract under test.

Focused tests do not prove the browser-to-server path, real authorization,
PostgreSQL/RLS, object delivery, renderer behavior, or visible user outcome.
They complement the required restored canonical browser suite; they never
provide a substitute browser runtime.

Fast Python tests stay in `tests/test_*.py`; pure Node tests stay in the
repository Node test lane; Rust tests stay with their owning crate. Slow
browser and service work stays outside the pytest fast collection. A temporary
probe belongs in ignored scratch space and is removed when its investigation
ends unless it independently meets the permanent-test standard.

## Production browser evidence

PLE retains one intended production `dist/` browser artifact and fixed
disposable real-stack browser path. M5 owns
`bash tests/e2e/e2e_live_demo_question_library.sh --browser`, which enters the
fixed HTTPS stack through the seeded Instructor session and proves visible
Ribbon navigation, search, and Question Details. It is deliberately a focused
milestone scenario. M6 owns `bash tests/e2e/e2e_live_demo_authoring.sh
--publish`, which creates and saves a private Draft Question, reviews and
publishes it, then returns through the real Question Library to open the new
Published Question. Those focused scenarios are not the serial
`./devel/run_playwright_tests.sh --build` owner required for M19 release
acceptance.

M7 owns `bash tests/e2e/e2e_live_demo_blueprint_course.sh --service` and
`--browser`. They are focused disposable acceptance, not permanent pytest
tests: the service path proves Blueprint Course Owner and Active Instructor
read access plus immutable successor-revision behavior, and the browser path
proves the visible Ribbon, Question picker, creation, publication, and list
return. Neither establishes Course Instance, roster, Assignment delivery,
Student, grading, or M19 serial-browser acceptance.

M8 owns `bash tests/e2e/e2e_live_demo_course_instance.sh --authority` and
`--browser`. They are focused disposable acceptance, not permanent pytest
tests: the authority path proves the exact published Blueprint Revision source,
immutable Course Origin, initial Assigned Instructor Course Membership, and no
ambient Sysadmin Course access or Student Record/Assignment creation. The real
Chromium path proves visible Instructor Course Instance creation and its
Teaching Team. Neither establishes roster, invitations, Assignment delivery,
Student work, grading, or M19 serial-browser acceptance.

M9 owns `bash tests/e2e/e2e_live_demo_roster.sh --import` and `--browser`.
They are focused disposable acceptance, not permanent pytest tests: the
authority path proves idempotent Student Authentication Email resolve-or-create,
pending Course Invitation, exact Student Record and Student Course Membership
claim, and immediate access revocation without deletion. The real Chromium
path proves Instructor Course Roster Import and its protected pending roster
projection. Neither establishes email delivery, Assignment delivery, Student
work, grading, export, or M19 serial-browser acceptance.

M10 owns `bash tests/e2e/e2e_live_demo_assignment_release.sh`. It is focused
disposable acceptance, not a permanent pytest test: the service path proves
direct Instructor Course Membership authority, exact Assignment Edit Number
conflict handling, release validation, and immutable Assignment Revision
snapshotting without Student work. The real Chromium path proves visible
Assignment creation, Available Published Question selection, save, validation,
answer-free Assignment Preview, and release. It does not prove Student View
Scenario evaluation, Assignment delivery, Student identity or work, grading,
or M19 serial-browser acceptance.

M11 owns `bash tests/e2e/e2e_live_demo_assignment_attempt.sh`. It is focused
disposable acceptance, not a permanent pytest test: the service path proves
Student-only public `C-`/`A-` access, the exact active Student Record boundary,
deadline refusal before issue, initial issuance/resume of a full answer-free
QuestionPresentation pinned to the exact Question Revision, and the narrow
forced-RLS released-snapshot policy. It covers the private immutable one-to-one
QuestionPresentation binding, which retains only nonce and full descriptor
checksum; server-only source/S3 resolution and reproduction details remain
private Question Attempt/source-binding facts. Resume reproduces the same public
presentation. The real Chromium
path proves visible Student access and initial start/resume. It does not prove
response controls, response persistence, submission, grading, feedback, Student
View Scenario evaluation, or M19 serial-browser acceptance.

M18 owns `bash tests/e2e/e2e_live_demo_invitation_export.sh --route` and
`--dry-run`. They are focused disposable acceptance, not delivery evidence:
the route path proves the Instructor-only no-store attachment export of pending,
unexpired Student Course Invitations in existing mailer JSON, and the browser
downloads only that attachment. The mailer dry run proves no delivery claim.
The fixed demo has one Instructor persona, so foreign-Instructor enforcement is
procedure/catalog evidence rather than browser evidence. Neither path proves
email delivery or M19 serial-browser acceptance.

M12 has focused decoder/cardinality and asset-free native-control evidence for
seven render-only controls. It remains active, not complete: a real issued
HOTSPOT needs an authority-compatible managed Question Asset registry, delivery,
and rendition path. Direct SQL/S3 seed data is not acceptance evidence.

The restored owner will regenerate the fixed disposable stack, serve the
production bundle through its HTTPS gateway, and run its selected real-stack
scenarios serially. The browser path will travel through the same-origin
gateway to the real API, PostgreSQL, MinIO, worker, renderer, authentication,
authorization, and seeded live-demo data. It will accept focused scenario,
file, or grep selection only through that owner and its declared scenario
contract. Each focused run will receive a fresh baseline; a complete run will
share one fixed stack while scenario namespaces keep product state independent.

The restored browser suite will create and change product state through visible
PLE workflows and assert visible, accessible behavior. The frozen baseline,
private bootstrap inputs, and induced infrastructure faults are harness setup,
not product-state shortcuts. Favor reload, a second authorized session, or an
authorized observer as the persistence proof for a user-visible result.

An inventory of legacy behavior identifies the user or contract behavior worth
keeping and assigns it to a canonical scenario, a focused unit test, or a
browser-free service oracle. It does not require retention of a former runtime
path merely because that path once exercised the behavior.

Legacy source/consumer inventories, migration matrices, and the one-time
mapping of superseded screenshot paths are closure evidence for this redesign.
They are not recurring pytest or Node tests. The retained test protects the
successor behavior; the inventory proves that the retired path no longer owns a
claim.

## Visual evidence

`./devel/capture_screenshots.sh` is the single current Live Demo capture command.
It regenerates the fixed disposable Browser Suite, serves the production bundle
through its HTTPS gateway, enters through the visible seeded Account selector,
and rebuilds the six declared desktop, tablet, phone, selected-state, and
invitation-email images under `docs/screenshots/live_demo/`.

This command is one-time rendered evidence, not a permanent test or full product
browser-acceptance lane. Its remaining showcase capture is historical developer
evidence, not Live Demo completion; M19 retires it before M20 creates the
declared safe capture corpus. Retained images elsewhere under `docs/screenshots/`
remain historical visual reference, and unavailable teaching workflows remain
unclaimed until their Server Routes and production-browser scenarios are restored.

## Service-only acceptance

Some claims need a browser-free oracle because visible UI behavior cannot
identify the underlying boundary. `local_stack.py acceptance` currently runs
exactly two complementary service lanes: the disposable PostgreSQL schema,
authority, and persistence oracle, and the Course Appearance PostgreSQL and
MinIO coherence oracle. They are service evidence, not a suffix after a
browser invocation and not proof of a visible user journey.

Other named service oracles belong to their specific package or future
capability; they are not current aggregate browser evidence:

- Question Library publication and replay use a named publication oracle for private
  source and Question Library installation, not a user journey.
- PostgreSQL migrations, forced RLS, and disclosure semantics use a named
  database oracle or a declared ignored database test. This is a disposable
  database boundary, not deployment availability.
- Course-appearance object storage uses the leased `course_appearance_cross_store`
  profile. It proves typed candidate and current Course Banner addresses against real MinIO. The
  database-backed current-pointer, promotion, and cleanup oracle is required when that Course
  Appearance capability is implemented in the current applied schema.
- Renderer render, grade, cache, outage, and redaction use a named renderer or
  worker oracle. This is a provider/service contract, not general
  compatibility.
- Replica restart uses two API replicas against one disposable PostgreSQL and
  verifies exact durable replay after the serving replica is replaced. This is
  a persistence and stateless-API oracle, not a second browser journey or a
  concurrent stack.
- Lifecycle, origin, and cleanup use suite receipts and narrow owner tests.
  They prove harness ownership, not a second browser workflow.

Read-only database, object-store, worker, renderer, or network receipts appear
only for a requirement about that service boundary. A service receipt does not
replace the user-visible workflow; a successful browser journey does not prove
an unrelated service guarantee.

## Reviews and records

Independent review names its artifact or environment, criteria, conclusion,
limitations, and follow-up work. A dated review, screenshot, or probe applies
to its reviewed snapshot rather than all future changes. Preserve concise
accepted decisions in the appropriate plan, handoff, or durable policy record;
use repository process documentation for review and release workflow details.
