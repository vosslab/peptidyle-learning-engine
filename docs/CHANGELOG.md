# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> September 16 entries are archived in [CHANGELOG-2026-09l.md](CHANGELOG-2026-09l.md).

## 2026-09-17

### Behavior or Interface Changes

- Refreshed the complete canonical screenshot corpus (76 captures) from a fresh Live Demo through
  `./devel/capture_screenshots.sh`, including the new Inactive Courses list and complete Student
  coverage of all eight native Question Types plus WeBWorK at laptop and phone widths. The
  `published_question_result` and `invitation_accepted` captures are retired; the invitation
  scenario now captures an invited Student who never joins so it replays on the same stack.
  Static corpus verification and the atlas regeneration passed.
- `./devel/capture_screenshots.sh` now runs against the already-running Live Demo by default
  (`--fresh` stops, starts, and stops an owned stack; `--only=<scenario,...>` stages selected
  scenarios without publishing). `./launchers/run_live_demo.sh` no longer clears a running or
  starting suite; it reports the entry URL, and only `stop` clears. A first start prints a
  30-second heartbeat and keeps supervisor output in `local_stack_state/live_demo_browser/supervisor.log`.
- B3 records source-approved Bloom Question Library discovery: two independent exact filters combine
  with every existing predicate, remain bound through saved searches, URL handoff, and opaque cursor
  continuation, and preserve existing sorts. Server facets describe the whole matching set with all
  six Cognitive Process and four Knowledge Dimension counts, including zeros and empty results.
  Connected multi-page, role, and browser proof remains open, so the Bloom discovery checklist row
  remains open.
- B2 records source-backed exact Question and Pool Revision Bloom correction
  routes. Active vetted Instructors, without an owner restriction, submit the
  complete pair under classification Edit Number CAS; stale `412` precedes
  no-op handling, a changed pair advances once, and no correction creates a
  content Revision. Sysadmins remain read-only. The client reloads the same
  exact Revision, retains the draft, and requires explicit resave without
  automatic retry or merge. Connected two-Instructor, denied-role, and browser
  proof remain open.
- B5 documents the implemented source boundary for Bloom Assessment sorting: fixed Entries project
  their exact pinned Question pair, Pool Entries project their exact Assessment-owned fork pair,
  and the stable order is cognitive process, knowledge dimension, then prior position. Sorting is
  blocked for an unavailable exact pair and remains an ordinary whole-Assessment Save under the
  existing Edit Number CAS. Source implementation is present, but review still awaits connected
  mixed-entry sort/save/reload and concurrent-save proof; Library filters and reporting also remain
  open, so the combined Bloom search-and-sorting checklist row stays open.
- B1 projects each exact Question or Pool Revision's required two-value Bloom
  Classification and independent classification Edit Number through answer-free
  Library reads. It adds exact Pool Revision GET and documents that an exact
  Question-detail response's legacy `latestQuestionRevision` field identifies the
  requested Revision. Source/model/decoder evidence is present; the connected
  actual-role read proof remains open, so this does not close the Bloom milestone.
- Clarified that Bloom Classification supports Question Library search and Assessment item sorting.
  The database stores only the two closed enum dimensions. Publication now requires a private,
  one-use receipt bound to the exact Bloom-relevant candidate content; the browser cannot supply
  either the receipt or classification. Fresh PostgreSQL 17 actual-role proof covered preparation
  privileges, kind/content binding, rollback, one-use consumption, deferred completeness, and
  Question and Pool Library admission. A configured real AI classifier remains required before
  connected publication can pass.
- Added answer-free native response previews after the Published Question prompt for MC, MA,
  FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT. The Library descriptor omits author response IDs,
  grading policy, answers, and submission state; controls remain inactive and MATCH displays one
  shared choice bank. WeBWorK keeps its backend-owned preview. Rust model, strict TypeScript,
  focused decoder checks, production client build, and temporary Chromium checks at 390/1280 passed.
  Canonical connected acceptance and the official screenshot refresh remain milestone checks.
- Added the Course tools action to create a new Blueprint from a Course Instance. Its compact accessible dialog pre-fills only new Blueprint short name, long name, and classification; explains copied reusable structure and unchanged source/first Adoption; validates the canonical Course reference; sends only the metadata DTO with retry-stable idempotency; requires `201`/`no-store`; retains values and reports accessibly; returns focus on cancel; and opens the owner-visible Private Revision-1 receipt. `cargo tsgen`, focused Node, TypeScript, ESLint, Prettier, and `./check_codebase.sh` (369 Node tests) passed. The backend atomically derives ordered reusable structure, forks Course-owned Pools, and records immutable source provenance. PostgreSQL 17 actual-role proof passed authorization/no-write, stale rollback, idempotency, source preservation, exact pins, Adoption/student counts, and lifecycle rollback. Canonical HTTPS C420 proof made one child-route POST from `CI0QR41X` to Private Revision-1 `BPJD8H28`, showed Adoption count 1, counted source Students only in the statistic, copied no roster/delivery state, and left the source addressable and unchanged.
- Accepted C522 connected Student delivery and Work issuance through the production
  server/data-access/access-page chain, including denied access. Accepted C879 connected
  Question-fork proof with two Instructors, exact source attribution, private denial,
  retry/concurrency, distinct server-issued identity, and prevalidation-publication denial. A separate
  3-by-3 PostgreSQL proof preserved all three compatible CC source licenses and rejected every
  mismatch through the approved minimal SQL. Temporary probes were removed.
- Implemented the settled Discipline and Library improvement-activity source boundaries. Disciplines
  use stable UUID create/rename/retire/restore with no delete, active-only new choice, retained
  visible references, exact inheritance/copy, and concurrency locks. Vetted-Instructor retained
  threads and owner/Sysadmin Question or Sysadmin-only Pool impact administration are present with
  Sysadmin read-only Library content controls. Final review, major-milestone SQL/browser proof, and
  four-event private Watch delivery remain open.
- Accepted I08's bounded 33-line, three-file Question Library correction: explicit Subject loading/ready/empty/error states explain settled zero Subjects and route discovery to Tags, Question Types, and Search while preserving ordinary facets, cascade, keyboard use, and detail return. The compact top-aligned `12rem` scroll region passed temporary actual-component Chromium at 1280/390 without overflow; the harness was removed. Strict TypeScript, 51 focused Library/Ribbon Node tests, ESLint, scoped Prettier/diff checks, and independent review passed. Official screenshots and authenticated connected Library acceptance remain deferred; no global density-row or SQL-lock claim follows.

### Fixes and Maintenance

- Live Demo start is now visible and progress-based instead of a silent 600s deadline. Root
  causes: `start_stack` ran the launch child with `capture_output=True`, so
  `local_stack_state/live_demo_browser/supervisor.log` stayed empty for the whole build while the
  heartbeat pointed at it; the fixed `DEVELOPER_START_WAIT_SECONDS = 600` covered a cold
  Podman-machine build that measured over ten minutes, and the resulting SIGTERM only set a flag,
  so the build ran to completion and was then purged. Now the launch child's output streams into
  the supervisor log line by line (`stream_launch`), the supervisor writes `[phase]` markers and
  `lifecycle` writes `Step:` lines before every captured Compose, host-build, wait, migration, and
  provisioning step, the CLI heartbeat (every 15s) prints elapsed time, phase, and the last log
  line plus a `tail -f` hint, and the parent keeps waiting while the log keeps growing, giving up
  only after 300s of silence (`stalled during <phase>`), a 3600s ceiling, a child exit, or Ctrl-C.
  Termination now forwards SIGTERM to the launch child's process group so a stall, ceiling, or
  Ctrl-C stops the build within seconds before the suite reset; `start` also probes `podman info`
  first so a stopped engine fails in seconds with `podman machine start` named. The parent-side
  wait moved to `local_stack_control/browser_suite_developer_start.py` to keep the supervisor
  module under the 1000-line gate.
- Launch failures now keep their evidence. The first streamed run showed the earlier 16:32 start
  had not timed out at all: it failed at "waiting for the complete stack to report ready" with the
  message `PostgreSQL is starting`, which `unavailable_report` emitted for any failed gateway
  health probe, and the purge then destroyed the stack before anyone could look. The probe now
  reports `gateway health probe at <url> failed: <curl detail>`; the launch child prints redacted
  `compose ps` and 60-line service log tails to the supervisor log before the purge
  (`print_launch_failure_evidence`); `_launch_diagnostic` prefers the child's own `ERROR:` line,
  so the operator message no longer shows cpanm's harmless `GD` bail-out (apt `libgd-perl` serves
  the renderer at runtime) with the real error truncated off the end.
- Controller curl probes pin `--ipv4`. The second streamed run's evidence showed every service
  healthy while `curl https://localhost:<port>/health` reported `Recv failure: Connection reset
  by peer`; a bare Caddy container reproduced it: `http://127.0.0.1:<port>/` answered 200 and
  `http://localhost:<port>/` was reset because `localhost` resolves to `::1` first and the Podman
  machine forwarder resets IPv6 loopback instead of refusing, so curl never falls back to IPv4.
  Browser URLs keep `https://localhost:` for the internal certificate; browsers fall back on
  their own.
- Found the actual cause of today's unreachable gateway: Homebrew upgraded Podman 6.1.1 -> 6.1.2
  at 16:07 while the machine VM and its `gvproxy` forwarder kept running the deleted 6.1.1
  binaries. With that mismatch a container on two networks (the gateway sits on `gateway_api`
  plus the browser overlay's `default`) publishes a port that the host cannot reach, while a
  single-network container works; the real gateway image with the real Caddyfile reproduced both
  outcomes in isolation. `podman machine stop; podman machine start` fixed it: the two-network
  probe answered 200 afterwards. A client/server version gate was tried and removed the same
  hour: the machine image keeps its own server version (still 6.1.1 after the restart), so the
  mismatch is normal on macOS and would block every post-upgrade start. The preflight stays a
  plain reachability check.
- The machine restart was not the fix either; the live stack still reset every host connection
  while `podman exec` inside the gateway answered 200 and even the VM could not reach the
  container on either IP. The gateway's two networks (`gateway_api`, browser `default`) are both
  `internal: true`, and this netavark drops host-side port forwarding into a container whose
  every network is internal; a standalone gateway on one `--internal` network reproduced the
  reset, on a normal network it answered 200. `containers/compose.yaml` now attaches the gateway
  to a non-internal `gateway_edge` network for its published port, and the port binds on all
  host interfaces (the operator wants Tailscale access) instead of `127.0.0.1` only. Firefox's
  `PR_END_OF_FILE_ERROR` on the same URL was this reset, not a certificate problem. Verified
  live: `/` and `/health` answer 200 on loopback and the port is open on the Tailscale address;
  over Tailscale Caddy still answers with a TLS alert because the site block is
  `https://localhost:8080`. Deferred by the operator; the one-line change is `https://:8080`.
- Removed every `from __future__ import annotations` (16 files); Python 3.12 evaluates the same
  annotations natively. `tests/test_no_future_imports.py` keeps them out.
- Repaired drift that blocked the Live Demo and screenshot replay: the base schema still granted
  the pre-Bloom-receipt `publish_question_revision` signature (psql exit 3 on install); the Live
  Demo seeder rejected the Course summary's new `lifecycleState` field; the Question Library
  decoder refused `bloom: null` ("The library could not load"); `build.sh` now regenerates the
  ignored `generated/api/QuestionIdSyntaxContract.ts` instead of only checking it. Screenshot
  scenarios follow the current UI (Create Course Instance and Create a Template disclosures,
  Practice Question Assignment labels, Course-name invitation heading, gradebook roster cells)
  and stop asserting Student completion state or byte-level details the captures do not depend on.
- Reconciled the SQL Human Guidance audit and generator-owned checklist parts with the current
  Closed SQL ledger. Bloom preparation/admission, Question/Pool Watches and notifications,
  Blueprint Promoted and Change Proposal persistence, and retained lifetime totals no longer appear
  as missing SQL mechanisms. Their application, browser, worker, and connected proof remains open.
- Removed the accidental native Ollama dependency from the PLE product runtime,
  Compose environment, installation guidance, and Live Demo contract. PLE now
  composes ordinary non-publication service paths without an AI backend. The
  provider-neutral Bloom preparation boundary remains unconfigured, and the
  SQL/data-access prepared-receipt dependency remains an explicit follow-up
  before new Question or Pool publication can complete. Optional developer-side
  Graphify model support is separate and unchanged.
- Threaded trusted, one-use Bloom preparation receipts through Question Pool creation,
  Assessment Pool import/append, Blueprint Pool materialization, Course adoption, source updates,
  and daughter-Course propagation. Browser DTOs remain unchanged; missing prepared receipts stop
  Pool publication. PostgreSQL-feature compilation, focused receipt consumption, and data-access
  Clippy passed. AI preparation orchestration and connected publication acceptance remain open.
- Replaced stale short Blueprint, Course Instance, and Assessment reference
  validators with one canonical SQL predicate. It accepts only the exact
  seven-random-character, checksum-bearing `BP`, `CI`, `A`, and `U` forms.
  A disposable PostgreSQL 17 base-schema install confirmed canonical acceptance
  and rejection of legacy-short, lowercase, wrong-prefix, and bad-checksum IDs.
- Removed the watered-down BIOL 301 RNA/DNA Question variant from the active Fall 2026 Genetics
  pilot selection. The upstream BiologyProblems.org source and its source mirror remain intact;
  the nonpublished import inventory records the explicit exclusion. Course-fixture labels now use
  actual Fall 2026 pilot Courses.
- Updated inline Rust fixtures that still used pre-cutover short public-ID values or unhyphenated
  Question IDs to canonical checksum-bearing values.
- Regenerated the two chromosome-shape PGML delivery copies with their dedicated matching and
  which-one generators after the shared colored-span spacing repair. The local manifest hashes now
  identify regenerated bytes; immutable committed upstream URL/hash pins remain unchanged.

### Decisions and Failures

- The pre-build image prune is now `podman image prune -f` (dangling layers only), not `-a`.
  The `-a` form deleted every image without a running container before each start: the reviewed
  WeBWorK renderer (about eight minutes to rebuild on this Podman machine), the pulled postgres
  and minio images, and any of the operator's unrelated tagged images whose containers happened
  to be stopped. Every start was therefore a cold start, and the second streamed run today spent
  its first nine minutes rebuilding an image the first run had already built. Stale layers from
  rebuilds still go; tagged images are kept because they are either expensive inputs or not this
  controller's to remove.
- Removed the unreferenced Question-ID reinitialization preflight script. PLE is pre-production,
  and the canonical public-ID cutover changes the authoritative base schema directly rather than
  preserving a second migration or data-rewrite path.
- Clarified the universal canonical public-ID invariant: exact canonical values persist unchanged
  across all boundaries, use public SHA-256 checksum characters, reserve globally, and are never
  reused. The closed human-entry forms normalize before canonical syntax/checksum validation;
  stored, transmitted, displayed, copied, and generated values stay canonical. Implementation
  inventory and the separately planned cutover remain open, so this documentation change claims no
  production behavior.
- Clarified that internal non-user-facing objects retain native UUIDs and public references exist only
  for required human-facing workflows. Existing `R`, `W`, `D`, `M`, and `I` short-reference concepts
  are implementation drift and must be removed rather than replaced.
- Added a Human Guidance deferred-product section that overrides implementation language elsewhere.
  iMathAS and H5P remain desired but deferred secondary backends. Future H5P use is limited to
  Regular Assignments, Bonus Assignments, and Practice Question Assignments and is excluded from
  Quizzes and Exams because its runtime exposes answers and correctness to the Student browser.
- Reconciled the Human Guidance plan and audit ledgers with current evidence: current checklist
  accounting is 1,018 bullets (472 verified, 498 open, 48 N/A); Deferred product behavior is
  authority but excluded from current implementation counting; only the two HG-unlocked complete
  Ribbon layouts may remain unresolved at closeout; closed SQL rows are distinct from pending
  application acceptance; high-priority UI findings now state their current rendered disposition;
  and Course Instance-to-Blueprint documentation uses Create Blueprint from Course Instance with a
  new Private Revision 1, immutable source provenance, first Adoption, and an unchanged source Course.
  The plan also records completed removal of dormant H5P placeholders. Current production Backends
  are PLE and WeBWorK; desired iMathAS/H5P behavior is deferred and implies no current backend work.
- Recorded tests as liabilities as well as assets: bounded work runs narrow gates and removes
  temporary probes; full Podman and `source source_me.sh && ./launchers/all_test.sh` acceptance is
  reserved for major milestones.
- Recorded C528 as open and decision-blocked: text-entry completeness needs raw-versus-trimmed-nonempty-versus-optional semantics, and opaque backend capture needs a product choice between successful capture and an adapter-supplied trusted completeness verdict. No implementation, test, or checklist completion is claimed.
