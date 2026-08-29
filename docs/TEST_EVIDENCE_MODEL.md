# Test evidence model

PLE uses evidence that matches the claim being made. Fast checks protect narrow
logic; the canonical browser suite proves visible product behavior; service
oracles prove the service boundaries that a browser cannot distinguish. This
document classifies that evidence. The active
[real_stack_browser_suite_plan.md](active_plans/active/real_stack_browser_suite_plan.md)
owns each work package's exact acceptance scope and command list.

Read [PYTEST_STYLE.md](PYTEST_STYLE.md) before adding a Python test and
[PLAYWRIGHT_TEST_STYLE.md](PLAYWRIGHT_TEST_STYLE.md) before changing a browser
test. [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) defines the disposable live-demo
baseline used by the browser suite.

## Validation test suite

Every work package names its complete Validation suite before completion. A
goal is complete only when every required gate is green on the final material
tree, including required live gates and independent review. `SKIP`, unrun, or
unavailable required gates are not green.

The repository aggregate front door is `./all_test.sh`. It fails fast through
these four gates, in this order:

```bash
./check_rust.sh
./check_codebase.sh
source source_me.sh && .venv/bin/python -m pytest tests/
source source_me.sh && .venv/bin/python local_stack.py acceptance
```

The Rust gate precedes the codebase gate because it owns generated TypeScript
inputs consumed by the latter. `local_stack.py acceptance` owns one canonical
browser-suite invocation, then only distinct browser-free service oracles.

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

The permanent suite contains callable unit, contract, security, and hygiene
behavior checks, plus the explicitly owned real-stack browser and service
gates. It does not preserve a superseded browser application's source inventory
or a dated screenshot-path inventory as a regression contract.

## Focused unit evidence

Keep permanent tests small, deterministic, and close to the behavior they
protect. Python, Node, and Rust unit or conformance tests own decoder,
serialization, strict transport, failure mapping, validation, and other narrow
logic. They may use inline fake values or isolated dependencies when those are
part of the contract under test.

Focused tests do not prove the browser-to-server path, real authorization,
PostgreSQL/RLS, object delivery, renderer behavior, or visible user outcome.
They complement the canonical browser suite; they never provide a second
browser application or a substitute browser runtime.

Fast Python tests stay in `tests/test_*.py`; pure Node tests stay in the
repository Node test lane; Rust tests stay with their owning crate. Slow
browser and service work stays outside the pytest fast collection. A temporary
probe belongs in ignored scratch space and is removed when its investigation
ends unless it independently meets the permanent-test standard.

## Production browser evidence

PLE has one production `dist/` browser artifact and one fixed disposable
real-stack browser path. [playwright.config.ts](../playwright.config.ts) is the
canonical Playwright configuration. `./run_playwright_tests.sh --build` is the
focused selector: the suite owner regenerates the fixed disposable stack,
serves the production bundle through its HTTPS gateway, and runs the selected
real-stack scenarios serially.

The browser travels through the same-origin gateway to the real API,
PostgreSQL, MinIO, worker, renderer, authentication, authorization, and
seeded live-demo data. The suite accepts focused scenario, file, or grep
selection only through that owner and its declared scenario contract. Each
focused run receives a fresh baseline; a complete run shares one fixed stack
while scenario namespaces keep its product state independent.

Playwright creates and changes product state through visible PLE workflows and
asserts visible, accessible behavior. The frozen baseline, private bootstrap
inputs, and induced infrastructure faults are harness setup, not product-state
shortcuts. Favor reload, a second authorized session, or an authorized observer
as the persistence proof for a user-visible result.

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

`./capture_screenshots.sh` invokes the same suite owner with `--screenshots`.
Screenshots therefore use the same disposable HTTPS origin, production `dist/`
bundle, scenario contract, real UI-created state, and privacy boundary as
browser acceptance.

`tests/e2e/browser_screenshot_corpus.json` is the canonical nested artifact
corpus. The TypeScript `tests/playwright/ui_corpus_manifest.ts` and Python
`tests/e2e/e2e_browser_screenshot_contract.py` are strict consumers of that
source; neither defines a competing artifact list. Capture stages artifacts,
then the publisher atomically publishes them after verifying origin, bundle
provenance, scenario metadata, paths, coverage, and privacy requirements.
Screenshots are scenario evidence for the production browser path, not a
separate application or visual test lane.

`./capture_screenshots.sh` is the separate explicit publication gate whenever
the UI, corpus, or viewport contract changes. `./all_test.sh` validates
behavior and contracts without rewriting checked-in documentation artifacts.
Both commands use the same fixed `ple-live-demo-browser` stack and suite owner.

## Service-only acceptance

Some claims need a browser-free oracle because visible UI behavior cannot
identify the underlying boundary. These commands remain distinct from the one
browser invocation in `local_stack.py acceptance`:

- Catalog publication and replay use a named publication oracle for private
  source and catalog installation, not a user journey.
- PostgreSQL migrations, forced RLS, and disclosure semantics use a named
  database oracle or a declared ignored database test. This is a disposable
  database boundary, not deployment availability.
- Course-appearance cleanup coherence uses the leased
  `course_appearance_cross_store` profile. It joins the real PostgreSQL current-pointer state to
  real MinIO candidate and promoted objects, then proves that cleanup removes superseded bytes and
  preserves the exact current Student-deliverable object.
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
