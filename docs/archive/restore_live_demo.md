# Live Demo restoration blueprint

## Status and purpose

Status: planned restoration.

This blueprint restores the Live Demo as one disposable, production-shaped PLE
walkthrough. It does not claim that a teaching, grading, or administration
workflow exists before its Store, Server Route, Browser Surface, and declared
evidence exist together.

The final journey uses the production dist bundle, fixed HTTPS gateway,
PostgreSQL, MinIO, private WeBWorK renderer, worker, and seeded data. It does
not fulfill product APIs from browser fixtures or grant authority from a
seeded-selector choice.

The post-entry path is the real Instructor, Student, or Sysadmin task for the
ordinary PLE application. It has no alternate presentation surface or fallback
for an unavailable teaching capability.

## Authority hierarchy

Every milestone follows this order. A lower-ranked item supplies evidence, not
permission to contradict a higher-ranked item.

| Rank | Authority                                                  | Use in this blueprint                                              |
| ---- | ---------------------------------------------------------- | ------------------------------------------------------------------ |
| 1    | [docs/HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md)             | Product intent, teaching philosophy, privacy, and role boundaries. |
| 2    | [docs/TERMINOLOGY_CONTRACT.md](../TERMINOLOGY_CONTRACT.md) | Canonical records, relationships, lifecycle names, and vocabulary. |
| 3    | Current subsystem contracts and inherited baselines        | Current capability, security, Ribbon, and export boundaries.       |
| 4    | Current implementation and evidence                        | Present capability inventory and executable evidence.              |
| 5    | Historical commit forensics                                | Lost-capability discovery only.                                    |

If a lower-ranked source conflicts with either primary authority, preserve the
primary authority, record the conflict in the package handoff, and stop that
package at the affected boundary. Historical code supplies algorithms and
scenario clues only. It never supplies current terminology, route shape,
authorization, browser chrome, or completion evidence.

Every work package inherits the same hierarchy. A package may use a narrower
current contract only after confirming that it preserves the two primary
authorities.

## Inherited baselines

| Code | Baseline                                                                                                                                                                                                           | Required rule                                                                                                         | Rejected restoration pattern                                                             |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| L    | [docs/TERMINOLOGY_CONTRACT.md](../TERMINOLOGY_CONTRACT.md)                                                                                                                                                         | Use current PLE terms across schema, API, code, browser copy, tests, and documentation.                               | Institution, tenant, Base Course, workspace-era, or registration-era names.              |
| S    | [docs/AUTHORIZATION_CONTRACTS.md](../AUTHORIZATION_CONTRACTS.md) and [connection_contract.rs](../../crates/learning-data-access/src/postgres/connection_contract.rs)                                               | Resolve Authenticated Session, authorize the exact durable relationship, then validate and commit in one transaction. | Ambient access, schema-owner bypass, broad worker access, or browser-only authorization. |
| R    | [src/application_shell.tsx](../../src/application_shell.tsx), [src/ribbon/ribbon_contract.ts](../../src/ribbon/ribbon_contract.ts), and [docs/ux/RIBBON_DESTINATION_LEDGER.md](../ux/RIBBON_DESTINATION_LEDGER.md) | Fit a restored route inside the current shell and enable a Ribbon destination only after its task works end to end.   | Pre-Ribbon chrome, disconnected pages, or enabled destinations without tasks.            |
| E    | [launchers/send_invitations.py](../../launchers/send_invitations.py) and the Instructor export boundary                                                                                                            | Keep email delivery outside the browser. The browser offers only protected export for the local dry-run mailer.       | Browser send, direct legacy send code, or PII and invitation tokens in URLs or logs.     |

## Demo completion contract

The restoration is complete only when one serial command sequence drives these
observable outcomes against one fresh disposable stack:

- The seeded selector enters separate Instructor, Student, and Sysadmin Accounts
  by issuing ordinary Authenticated Sessions.
- The Instructor publishes a Question, creates a Blueprint Course, creates a
  Course Instance, imports a roster, releases an Assignment, and reads the
  Gradebook.
- The Student starts an Assignment Attempt, answers all four seeded native
  Questions, submits, receives deterministic Student Feedback, and retains the
  accepted submission across a worker interruption without resending it.
- The Sysadmin creates an Instructor Account and performs one Instructor-issued,
  time-bounded support operation without ambient FERPA access.
- At least one WeBWorK Question completes issue, submission, grading, and
  display through the private renderer.
- Every enabled Ribbon destination completes its named task. All other
  destinations remain unavailable and truthful.

M19 owns the serial browser sequence. M20 produces rendered artifacts after M19
passes. M21 reconciles documentation after the same final material tree passes
its declared gates.

## Security and privacy constraints

The current contracts require opaque server-validated Authenticated Sessions,
exact relationship authorization, forced RLS, answer secrecy, and privacy-safe
audit evidence. The restoration preserves those boundaries.

| Boundary                    | Required rule                                                                                                              | ASVS planning reference                                   |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| Browser session             | Use a host-only Secure HttpOnly first-party cookie, backend token validation, token rotation, and server revocation.       | ASVS 3.3.1, 3.3.4, 7.2.1-7.2.4, 7.4.1-7.4.2               |
| Browser and gateway         | Use HTTPS, fixed CORS, CSP, HSTS, nosniff, referrer policy, and denied-by-default framing.                                 | ASVS 3.1.1, 3.4.1-3.4.6, 3.5.1-3.5.3, 4.1.1-4.1.4, 12.1.1 |
| Authentication              | Keep seeded entry deployment-gated and isolated from ordinary credential ceremony; document rate and enumeration controls. | ASVS 6.1.1, 6.1.3, 6.3.1, 6.3.4, 6.3.8                    |
| Authorization               | Enforce function, record, and field scope inside trusted services and protected PostgreSQL operations.                     | ASVS 8.1.1-8.1.2, 8.2.1-8.2.3, 8.3.1                      |
| Student and credential data | Keep sensitive values out of URLs, browser storage, caches, screenshots, and audit payloads.                               | ASVS 14.1.1-14.1.2, 14.2.1-14.2.7                         |
| Services and runtime        | Document dependencies, timeouts, connection limits, recovery, secrets, worker limits, and renderer topology.               | ASVS 12.3.1-12.3.4, 13.1.1-13.1.4, 15.1.1-15.1.5          |
| Evidence                    | Record authentication, authorization denial, capability, lease, and security-control events without protected values.      | ASVS 16.1.1, 16.3.1-16.3.4                                |

The product supports passwordless passkey or email-code authentication. Seeded
entry remains a disposable local identity-verification substitute, not an
additional general authentication path. This blueprint does not claim ASVS
Level 3 authentication conformance.

## Mapping

L applies to every row because terminology is primary.

| ID  | Capability                               | Depends on   | Baselines  | Primary evidence              |
| --- | ---------------------------------------- | ------------ | ---------- | ----------------------------- |
| M0  | Foundation findings                      | none         | L, S, R, E | bounded reports               |
| M1  | Worker topology                          | M0           | L, S       | disposable topology runner    |
| M2  | Readiness and startup diagnosis          | M1           | L, S       | dependency-outage runner      |
| M3  | Grading role, lease, and procedures      | M0, M1       | L, S       | PostgreSQL and worker receipt |
| M4  | Seeded baseline installer                | M1, M3       | L, S, R    | replay manifest runner        |
| M5  | Question Library browse and detail       | M4           | L, S, R    | Instructor scenario           |
| M6  | Draft authoring and publication          | M4           | L, S, R    | publication scenario          |
| M7  | Blueprint Course lifecycle               | M4           | L, S, R    | Blueprint scenario            |
| M8  | Course Instance and teaching team        | M7           | L, S, R    | Course scenario               |
| M9  | Roster, invitations, and Student Records | M8           | L, S, R, E | roster scenario               |
| M10 | Assignment authoring and release         | M8, M5       | L, S, R    | release scenario              |
| M11 | Assignment Attempt issuance              | M9, M10      | L, S, R    | Student start scenario        |
| M12 | Native response controls                 | M11          | L, S, R    | keyboard response scenario    |
| M13 | Submission, grading, and recovery        | M3, M11, M12 | L, S, R    | recovery scenario             |
| M14 | WeBWorK delivery and grading             | M3, M11      | L, S, R    | renderer scenario             |
| M15 | Gradebook and Student Work               | M13          | L, S, R    | Instructor evidence scenario  |
| M16 | Instructor Account management            | M4           | L, S, R    | Sysadmin account scenario     |
| M17 | Support-capability operations            | M8, M16      | L, S, R    | scoped-support scenario       |
| M18 | Invitation export                        | M9           | L, S, R, E | protected export runner       |
| M19 | Connected browser owner                  | M5-M18       | L, S, R, E | serial production-browser run |
| M20 | Screenshot corpus                        | M19          | L, S, R, E | capture manifest              |
| M21 | Authority and documentation close-out    | M19, M20     | L, S, R, E | final reconciliation suite    |

## Migration and compatibility policy

PLE is pre-production and Live Demo state is disposable. Foundational schema,
contract, and ownership defects should be corrected at the baseline that creates
them when no accepted environment retains data. The correction includes migration
source, schema documentation, Store contract, fixtures, and clean-database
acceptance.

The current [docs/DATABASE_STRUCTURE.md](../DATABASE_STRUCTURE.md) also
states that accepted migrations are immutable. M3 first records whether the
target migration has reached an accepted data-bearing environment. If it has,
M3 uses the current migration-allocation rule. If it has not, M3 corrects the
baseline source and recreates the disposable volume. This follows both the
current database contract and the pre-production design guidance.

No compatibility reader, legacy route, vocabulary alias, parallel permission
model, or feature selector is added merely to preserve deleted behavior.

## Assumptions

| Item                     | State               | Treatment                                                                                              |
| ------------------------ | ------------------- | ------------------------------------------------------------------------------------------------------ |
| Private WeBWorK renderer | resolved            | Current compose topology and probe support an automated disposable renderer lane.                      |
| Worker runtime           | implementation work | M1-M3 provide topology, least-privilege role, lease, and procedure boundaries before Student delivery. |
| Production-browser owner | absent              | M19 creates one serial production-browser owner.                                                       |
| Email delivery           | outside completion  | M18 stops at protected export consumed by the current dry-run mailer.                                  |
| Screenshot corpus        | evidence artifact   | M20 checks capture reachability and manifest completeness without treating images as task proof.       |
| Historical commits       | discovery source    | M0 records useful lost behavior without copying obsolete names or boundaries.                          |

## Open questions

| Question                                                                   | Answering milestone | Decision rule                                                                                        |
| -------------------------------------------------------------------------- | ------------------- | ---------------------------------------------------------------------------------------------------- |
| Does a target migration qualify for direct baseline correction?            | M3                  | Use direct correction only when no accepted data-bearing environment exists.                         |
| Which retained browser specifications describe a current product contract? | M0                  | Retain a scenario only when the primary authorities and current contract support it.                 |
| Which current browser clients can survive a route restoration unchanged?   | M0                  | Reuse only a strict decoder and client whose expected DTO matches the current Server Route contract. |

## Status tracker

| Milestone | Status   | Required completion evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| M0        | complete | [foundation findings report](../active_plans/audits/live_demo_foundation_findings.md); fresh disposable acceptance passed on 2026-09-06, but it is not browser evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| M1        | complete | `bash tests/e2e/e2e_live_demo_worker_topology.sh` passed on 2026-09-06; topology and lifecycle only, no Job or lease behavior claimed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| M2        | complete | `bash tests/e2e/e2e_live_demo_readiness.sh` passed on 2026-09-06; the fixed stack proves bounded healthy and per-dependency 503/recovery states, not Job execution or browser-task completion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| M3        | complete | `bash tests/e2e/e2e_live_demo_grading_lease.sh` and each named mode passed on 2026-09-06 against a fresh isolated PostgreSQL 17 runtime; procedure/lease evidence only, not a dispatcher or Student delivery                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| M4        | complete | `bash tests/e2e/e2e_live_demo_seeded_baseline.sh --replay` passed on 2026-09-06; it proves fixed disposable Accounts and a private-source Published Question baseline replay, not Question Library or Student browser delivery                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| M5        | complete | `bash tests/e2e/e2e_live_demo_question_library.sh` passed on 2026-09-06 against the fixed HTTPS stack; it proves Instructor Ribbon navigation, search, and answer-free detail plus Student/anonymous concealment, not later Course or Student tasks                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| M6        | complete | `bash tests/e2e/e2e_live_demo_authoring.sh --draft` and `--publish` passed on 2026-09-06 against the fixed HTTPS stack; the latter includes Chromium authoring, publication, and Question Library handoff, while later Course and Student tasks remain unimplemented                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| M7        | complete | `bash tests/e2e/e2e_live_demo_blueprint_course.sh --service` and `--browser` passed on 2026-09-06 against the fixed HTTPS stack; M7 proves Blueprint Course Owner and Active Instructor read access, immutable Blueprint Revision preservation, and visible Instructor creation/publication, not Course Instance delivery                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| M8        | complete | `bash tests/e2e/e2e_live_demo_course_instance.sh --authority` and `--browser` passed on 2026-09-06 against the fixed HTTPS stack; M8 proves exact published Blueprint Revision source, immutable Course Origin, initial Assigned Instructor Course Membership and Teaching Team, no ambient Sysadmin Course access, and visible Instructor Course Instance creation/opening, not roster, Student Record, Assignment, or delivery                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| M9        | complete | `bash tests/e2e/e2e_live_demo_roster.sh --import` and `--browser` passed on 2026-09-06 against the fixed HTTPS stack; M9 proves idempotent Course Roster Import, Student Authentication Email resolve-or-create, pending Course Invitation, exact Student Record/Student Course Membership claim, immediate access revocation, and visible Instructor roster projection, not email delivery, Assignment delivery, grading, or M19 serial-browser acceptance                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| M10       | complete | `bash tests/e2e/e2e_live_demo_assignment_release.sh` passed on 2026-09-06 against a final fresh fixed HTTPS stack; M10 proves direct Instructor Course Membership authority, exact Assignment Edit Number conflict handling, release validation, immutable Assignment Revision snapshotting without Student work, and visible Assignment creation, Available Published Question selection, save, answer-free Assignment Preview, and release. It does not implement Student View Scenario evaluation, Assignment delivery, Student identity or work, grading, or M19 serial-browser acceptance.                                                                                                                                                                                                                                                                                                                                                        |
| M11       | complete | `bash tests/e2e/e2e_live_demo_assignment_attempt.sh` passed again on 2026-09-07 against a fresh controller-managed fixed HTTPS stack. M11 proves Student-only public `C-`/`A-` Assignment Access, exact active Student Record authorization, released-snapshot deadline refusal before issue, initial issuance/resume of an exact Question Revision as a full answer-free QuestionPresentation, and narrow forced-RLS snapshot-entry access. The private one-to-one immutable QuestionPresentation binding retains only nonce and full descriptor checksum; source/S3 resolution and reproduction details remain private Question Attempt/source-binding facts, and resume reproduces the same public presentation. The M12 format-only presentation omits M13 submission, grading, and feedback controls. M13 response persistence/submission/grading/recovery, Student View Scenario evaluation, and M19 serial-browser acceptance remain unclaimed. |
| M12       | complete | `bash tests/e2e/e2e_live_demo_native_controls.sh` passed on 2026-09-07 against a fresh controller-managed fixed HTTPS stack. M12 proves authorized Question Asset retrieval returns an immutable redirect while anonymous, foreign-Student, absent, and malformed references receive indistinguishable concealment; all eight issued native PLE response formats strictly decode; and all eight controls, including the fixed HOTSPOT, reach valid local states by keyboard without submission. This is whole-system plan acceptance/live browser evidence, not a new pytest. M13 response persistence/submission/grading/feedback/recovery and M19 serial-browser acceptance remain unclaimed.                                                                                                                                                                                                                                                        |
| M13       | complete | `bash tests/e2e/e2e_live_demo_submission_recovery.sh` passed on 2026-09-07 against a fresh controller-managed fixed HTTPS stack. M13 proves one format-valid Student Response is accepted once with closed pending grading state, and an interrupted leased native PLE evaluation recovers one terminal result and receipt. This is whole-system plan acceptance evidence, not a new pytest. M15 Gradebook/Student Work and M19 serial-browser acceptance remain unclaimed.                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| M14       | complete | `bash tests/e2e/e2e_live_demo_webwork.sh --render` previously accepted the render-issuance boundary. On 2026-09-07, `bash tests/e2e/e2e_live_demo_webwork.sh --grade` passed on a fresh controller-managed stack: `WeBWorK grade authority: deterministic renderer grade commit and bounded renderer failure complete`; `Live Demo WeBWorK grade: PASS`. The renderer fault is a bounded terminal local outcome. This service/browser cadence does not claim M19 serial production-browser acceptance.                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| M15       | complete | `bash tests/e2e/e2e_live_demo_gradebook.sh --api` and `--browser` passed on 2026-09-07 against fresh controller-managed fixed HTTPS stacks. M15 proves current Course Instructor access to answer-free immutable Gradebook evidence, foreign-Course 404 concealment, and the visible Gradebook task. Individual Student Work remains unavailable; M19 serial-browser acceptance remains unclaimed.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| M16       | complete | `bash tests/e2e/e2e_live_demo_instructor_accounts.sh` passed on 2026-09-07 against a fresh controller-managed fixed HTTPS stack. M16 proves the Sysadmin-only Instructor Account lifecycle service, account concealment, and session revocation; the visible Sysadmin Ribbon task creates an Instructor Account, then deactivates and reactivates it. The disposable migration correction gives only `ple_private_owner` the Instructor-only lock policy needed by the existing `SELECT ... FOR UPDATE` boundary; it grants no direct application/API table access. This is whole-system plan acceptance/live browser evidence, not a new pytest. Course or Student Record access, passkey feature work, and M19 serial-browser acceptance remain unclaimed.                                                                                                                                                                                           |
| M17       | complete | `bash tests/e2e/e2e_live_demo_support_capability.sh --issue` and `--browser` passed on 2026-09-07 against a fresh controller-managed fixed HTTPS stack. M17 proves exact-Course registered roster-support capability issuance, concealment, and revocation without ambient Sysadmin Course or Student Record access; the visible Sysadmin scoped roster task also completed. This is whole-system plan acceptance/live browser evidence, not a new pytest. M19 serial-browser acceptance remains unclaimed.                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| M18       | complete | `bash tests/e2e/e2e_live_demo_invitation_export.sh --route` and `--dry-run` passed on 2026-09-06 against a fresh stack; M18 proves current direct-Instructor no-store attachment export of pending, unexpired Student Course Invitations in existing mailer JSON and dry-run-only consumption. Browser behavior is download-only and no send/delivery is claimed. The fixed demo has one Instructor persona, so foreign-Instructor procedure/catalog enforcement is recorded rather than browser-tested.                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| M19       | complete | The final `./devel/run_playwright_tests.sh --build` re-run passed on 2026-09-07 against a fresh controller-managed fixed HTTPS stack. Its serial owner exercised the connected auth, Instructor authoring, Student recovery, Assignment release, WeBWorK render, Sysadmin Instructor Account, scoped-support, and invitation-export journeys; focused milestone receipts remain their own narrower evidence.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| M20       | complete | `./devel/capture_screenshots.sh` rebuilt eight manifest-listed public and role-owned captures through visible navigation on 2026-09-07, then stopped its owned stack. The canonical set includes a desktop Ribbon capture for Instructor, Student, and Sysadmin plus the declared Student responsive surfaces. `./devel/capture_screenshots.sh --verify` passed; the manifest declares only safe current artifacts and the capture runner rejects protected screen text, filled Authentication Email fields, and answered Student Question controls.                                                                                                                                                                                                                                                                                                                                                                                                   |
| M21       | complete | Manual authority reconciliation followed Human Guidance, Terminology Contract, current subsystem contracts, source, and evidence in order. On the formatter-final material tree, `source source_me.sh && ./launchers/all_test.sh` passed 5,953 offline tests and both disposable live-service acceptance oracles; `./devel/run_playwright_tests.sh --build` passed the serial production-browser owner.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |

## Risk register

| Risk                                             | Early signal                                      | Containment                                                                | Owner milestone |
| ------------------------------------------------ | ------------------------------------------------- | -------------------------------------------------------------------------- | --------------- |
| Legacy vocabulary returns through copied code    | old identifiers enter a diff                      | run terminology scans and replace copied boundaries before integration     | M0, M21         |
| Worker bypasses RLS                              | role needs broad table access                     | use exact procedures, typed lease claims, and PostgreSQL denial checks     | M3              |
| Browser claims a route before service completion | Ribbon destination is enabled without a real task | preserve unavailable state until M19 scenario evidence                     | M5-M19          |
| Student response is exposed during recovery      | recovery repeats a response                       | retain one accepted Question Submission and recover from its receipt       | M13             |
| Renderer outage widens scope                     | fallback changes backend or exposes grader input  | fail one Question locally with bounded state and preserve receipt boundary | M14             |
| Seeded state drifts across scenarios             | later scenario observes prior state               | rebuild fixed baseline and namespace each scenario                         | M4, M19         |
| FERPA data reaches Sysadmin or artifacts         | support view contains unrelated Student fields    | use registered projection and scan browser payload plus capture manifest   | M17, M20        |

## Patch plan

Each work package follows the same small delivery order:

1. Read the primary authorities and baseline links from its Mapping row.
2. Characterize the present boundary with a focused check or disposable probe.
3. Change one owned capability across schema, Store, Service, route, browser,
   and evidence as needed.
4. Run the package acceptance command and record the exact result in its handoff.
5. Run the broader required gates before the milestone changes status.

A package does not take ownership of another package's migration set, seeded
data, compose topology, browser suite, or screenshot publication. It waits on
the declared dependency when it needs that resource.

## Milestones and work packages

### Milestone M0: Foundation findings

Depends on: none.

Parallel-plan ready: yes. The three bounded findings have no shared write
surface and complete before dependent capability work begins.

Exit command:

```bash
test -s docs/active_plans/audits/live_demo_foundation_findings.md
```

### Work package: WP-M0-1 RLS baseline finding

Owner: database investigator.

Touch points: migration catalog, PostgreSQL probe, and M3 handoff.

Depends on: none.

Acceptance criteria:

```bash
source source_me.sh && python3 local_stack.py acceptance
```

Expected outcome: the findings report records tables, forced-RLS state,
policies, and default-deny results from the disposable PostgreSQL lane.

Obvious follow-ons: send any schema correction requirement to M3.

### Work package: WP-M0-2 Browser specification classification

Owner: browser-contract investigator.

Touch points: tests/playwright, terminology contract, and M19 scenario inventory.

Depends on: none.

Acceptance criteria:

```bash
rg --files tests/playwright -g '*.spec.ts'
```

Expected outcome: the findings report classifies each retained specification as
retain, rename, or retire.

Obvious follow-ons: send the current-contract scenario inventory to M19.

### Work package: WP-M0-3 Client and decoder finding

Owner: browser API investigator.

Touch points: src/api, src/pages, strict decoders, and M5-M10 handoffs.

Depends on: none.

Acceptance criteria:

```bash
rg -n 'decode|fetch|request' src/api src/pages
```

Expected outcome: the findings report names reusable client paths and one owner
for each decoder mismatch.

Obvious follow-ons: assign each mismatch to a feature milestone without a facade.

### Milestone M1: Worker service and topology

Depends on: M0.

Parallel-plan ready: no. The worker identity, compose service, and private
network form one topology boundary that M2-M4 consume.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_worker_topology.sh
```

### Work package: WP-M1-1 Compose worker identity

Owner: runtime topology engineer.

Touch points: containers, local-stack compose owner, and private runtime inputs.

Depends on: M0.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_worker_topology.sh --service
```

Expected outcome: one worker Service Identity, private dependency access, and no
public gateway route.

Obvious follow-ons: expose only M2 readiness facts through the gateway.

### Work package: WP-M1-2 Worker process contract

Owner: worker runtime engineer.

Touch points: worker entry module, fixed owner lifecycle, and shutdown handling.

Depends on: WP-M1-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_worker_topology.sh --lifecycle
```

Expected outcome: start, stop, and replace the worker without a retained owner lease.

Obvious follow-ons: M3 supplies typed claim and commit operations.

### Milestone M2: Honest readiness and startup diagnosis

Depends on: M1.

Parallel-plan ready: no. Readiness is the public status boundary for the new
topology and settles before dependent feature scenarios start.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_readiness.sh
```

### Work package: WP-M2-1 Dependency readiness model

Owner: local-stack controller engineer.

Touch points: local-stack controller, health target, and startup diagnostics.

Depends on: M1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_readiness.sh --healthy
```

Expected outcome: report checked API, database, object store, renderer, and worker states.

Obvious follow-ons: retain bounded diagnostic fields only.

### Work package: WP-M2-2 Dependency outage transitions

Owner: local-stack controller engineer.

Touch points: fault harness, gateway response mapping, and readiness tests.

Depends on: WP-M2-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_readiness.sh --outages
```

Expected outcome: each stopped dependency returns its declared bounded 503 result
and recovery state.

Obvious follow-ons: M19 consumes this harness for browser outage scenarios.

### Milestone M3: Grading role, lease, and procedures

Depends on: M0, M1.

Parallel-plan ready: no. It owns migration decisions, database role, and exact
worker procedures consumed by M4 and M13-M14.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_grading_lease.sh
```

### Work package: WP-M3-1 Baseline migration decision

Owner: PostgreSQL migration engineer.

Touch points: schema migrations, database structure contract, and clean-volume setup.

Depends on: WP-M0-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_grading_lease.sh --catalog
```

Expected outcome: prove selected migration policy and clean schema with forced RLS.

Obvious follow-ons: keep migration sequence and database documentation coherent.

### Work package: WP-M3-2 Grading Service Identity

Owner: database authorization engineer.

Touch points: PostgreSQL roles, grants, protected procedures, and connection contract.

Depends on: WP-M3-1, M1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_grading_lease.sh --role
```

Expected outcome: worker performs only declared claim and commit operations.

Obvious follow-ons: deny direct table access and schema-owner impersonation.

### Work package: WP-M3-3 Typed lease lifecycle

Owner: grading runtime engineer.

Touch points: typed Job records, lease tokens, generation fences, and receipt commit.

Depends on: WP-M3-2.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_grading_lease.sh --replay
```

Expected outcome: reject stale, foreign, and duplicate claims while retaining one receipt.

Obvious follow-ons: M13 and M14 use these lease rules.

### Milestone M4: Demo-data installer and seeded baseline

Depends on: M1, M3.

Parallel-plan ready: no. The installer owns the fixed dataset and replay
manifest, so all feature scenarios consume one authoritative baseline.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_seeded_baseline.sh
```

### Work package: WP-M4-1 Seeded Account and content set

Owner: demo-data engineer.

Touch points: disposable seed installer, Account mappings, Questions, and Course source data.

Depends on: M1, M3.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_seeded_baseline.sh --install
```

Expected outcome: create declared persona Accounts and answer-free seed manifest.

Obvious follow-ons: do not surface opaque seed references in browser content or URLs.

### Work package: WP-M4-2 Replay and reset contract

Owner: demo-data engineer.

Touch points: controller cleanup, manifest verification, and fixed owner lifecycle.

Depends on: WP-M4-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_seeded_baseline.sh --replay
```

Expected outcome: recreate baseline without duplicate Published Questions or Accounts.

Obvious follow-ons: M19 requests reset before serial scenarios.

### Milestone M5: Question Library browse and detail

Depends on: M4.

Parallel-plan ready: yes. It uses seeded Published Questions and does not share
write ownership with M6 or M7.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_question_library.sh
```

### Work package: WP-M5-1 Question Library Store and route

Owner: Question Library API engineer.

Touch points: Question Library Store, Server Route, browser DTO, and strict decoder.

Depends on: M4.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_question_library.sh --api
```

Expected outcome: Instructor success and Student plus anonymous concealment.

Obvious follow-ons: preserve Question Revision Reference in every result.

### Work package: WP-M5-2 Ribbon Library task

Owner: Question Library browser engineer.

Touch points: application shell, Ribbon registry, library page, and client.

Depends on: WP-M5-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_question_library.sh --browser
```

Expected outcome: visible navigation, search, and detail complete from enabled destination.

Obvious follow-ons: retain unavailable destinations without placeholder handlers.

### Milestone M6: Draft authoring, preview, and publication

Depends on: M4.

Parallel-plan ready: yes. It owns private Authoring Workspace behavior and
shares no Course or Assignment write surface with M7-M10.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_authoring.sh
```

### Work package: WP-M6-1 Draft Question workflow

Owner: authoring service engineer.

Touch points: Authoring Workspace Store, Draft Question Source Binding, and validation.

Depends on: M4.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_authoring.sh --draft
```

Expected outcome: exact workspace authority, Edit Number conflicts, and private source boundary.

Obvious follow-ons: expose validation issues and preview-safe content only.

### Work package: WP-M6-2 Publication path and browser task

Owner: authoring browser engineer.

Touch points: publication Service, Server Route, My Question Drafts, and library handoff.

Depends on: WP-M6-1, M5.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_authoring.sh --publish
```

Expected outcome: create immutable Question Revision and expose it through Question Library.

Obvious follow-ons: no Draft Question identity or source path enters published DTOs.

### Milestone M7: Blueprint Course lifecycle

Depends on: M4.

Parallel-plan ready: yes. Blueprint Course work is reusable Instructor content
and remains separate from Course Instance delivery in M8.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_blueprint_course.sh
```

### Work package: WP-M7-1 Blueprint Course Store and Service

Owner: course-model engineer.

Touch points: Blueprint Course records, revisions, publication, and Instructor authority.

Depends on: M4.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_blueprint_course.sh --service
```

Expected outcome: exact owner and Active Instructor read paths with revision preservation.

Obvious follow-ons: retain Blueprint Course Read Access terminology.

### Work package: WP-M7-2 Blueprint Course browser task

Owner: course browser engineer.

Touch points: Ribbon destination, client, Blueprint Course pages, and strict decoding.

Depends on: WP-M7-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_blueprint_course.sh --browser
```

Expected outcome: Instructor creates and publishes seeded Blueprint Course through visible controls.

Obvious follow-ons: M8 uses the exact published Blueprint Revision.

### Milestone M8: Course Instance creation and teaching team

Depends on: M7.

Parallel-plan ready: no. Course Instance creation establishes exact durable
scope consumed by roster, Assignment, Gradebook, and support packages.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_course_instance.sh
```

### Work package: WP-M8-1 Course Instance creation authority

Owner: course authorization engineer.

Touch points: Course Instance Creation, Course Origin, Course Membership, and audit event.

Depends on: M7.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_course_instance.sh --authority
```

Expected outcome: assigned Instructor gains teaching authority and creator gains no ambient access.

Obvious follow-ons: expose no Student Record or delivery state during bootstrap.

### Work package: WP-M8-2 Teaching-team browser task

Owner: course browser engineer.

Touch points: Course pages, teaching-team projection, Ribbon capability, and client.

Depends on: WP-M8-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_course_instance.sh --browser
```

Expected outcome: assigned Instructor enters new Course Instance through visible task.

Obvious follow-ons: M9 owns roster and M10 owns Assignment content.

### Milestone M9: Roster import, invitations, and Student Records

Depends on: M8.

Parallel-plan ready: no. Roster import establishes Student Record and membership
scope that M11 uses for Assignment Access.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_roster.sh
```

### Work package: WP-M9-1 Course Roster Import transaction

Owner: roster service engineer.

Touch points: Student Authentication Email resolution, Course Membership, Student Record, and RLS.

Depends on: M8.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_roster.sh --import
```

Expected outcome: idempotent resolution, exact course scope, and immediate revocation.

Obvious follow-ons: retain Student Authentication Email immutability.

### Work package: WP-M9-2 Invitation and roster browser task

Owner: roster browser engineer.

Touch points: Course Invitation routes, roster page, invitation audit, and export handoff.

Depends on: WP-M9-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_roster.sh --browser
```

Expected outcome: Instructor imports seeded roster and receives authorized course projection.

Obvious follow-ons: M18 consumes protected Instructor-issued export.

### Milestone M10: Assignment authoring, policy, release, and preview

Depends on: M8, M5.

Parallel-plan ready: no. Assignment release binds immutable teaching snapshot
that M11 issues to a Student.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_assignment_release.sh
```

### Work package: WP-M10-1 Assignment Workspace Service

Owner: Assignment service engineer.

Touch points: Assignment, Assignment Edit Number, Assignment Release Validation, and revisions.

Depends on: M8, M5.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_assignment_release.sh --service
```

Expected outcome: Instructor Course Membership authority and immutable release behavior.

Obvious follow-ons: preserve Assignment Status separately from Assignment Access.

### Work package: WP-M10-2 Assignment Workspace browser task

Owner: Assignment browser engineer.

Touch points: Assignment Workspace pages, Question Picker, Assignment Preview, and Ribbon task.

Depends on: WP-M10-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_assignment_release.sh --browser
```

Expected outcome: Instructor selects Published Questions, validates, previews, and releases Assignment.

The M10 Assignment Preview is Instructor-only and answer-free. It is not the
separate retained Student View Scenario contract and creates no Student work.

Obvious follow-ons: M11 reads released Assignment Revision only.

### Milestone M11: Assignment Attempt lifecycle and issuance

Depends on: M9, M10.

Parallel-plan ready: yes. It establishes backend issue and access decisions while
M12 builds response controls against the answer-free presentation contract.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_assignment_attempt.sh
```

### Work package: WP-M11-1 Assignment Access and start Service

Owner: Student delivery service engineer.

Touch points: Assignment Access, Effective Assignment Policy, Assignment Attempt, and Issued Question.

Depends on: M9, M10.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_assignment_attempt.sh --start
```

Expected outcome: exact Student Record scope, deadline behavior, and one allowed presentation.

Obvious follow-ons: retain Answer Keys and Question Grading Input on trusted server.

### Work package: WP-M11-2 Student entry browser task

Owner: Student delivery browser engineer.

Touch points: Student route, attempt client, strict presentation decoder, and application shell.

Depends on: WP-M11-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_assignment_attempt.sh --browser
```

Expected outcome: seeded Student starts released Assignment through keyboard-reachable control.

Obvious follow-ons: M12 supplies Question Response Controls.

### Milestone M12: Eight native Question Response Controls

Depends on: M11.

Parallel-plan ready: yes. Each control shares M11 presentation contract and can
be delivered as an isolated browser module.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_native_controls.sh
```

### Work package: WP-M12-1 Response format and keyboard contract

Owner: question interaction engineer.

Touch points: Question Presentation Response Format, strict decoder, and control models.

Depends on: M11.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_native_controls.sh --format
```

Expected outcome: accept canonical response shapes and reject malformed shapes before submission.

Obvious follow-ons: preserve Question Type neutrality at generic client boundary.

### Work package: WP-M12-2 Native control modules

Owner: question interaction engineer.

Touch points: response components, keyboard instructions, focus order, and responsive layout.

Depends on: WP-M12-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_native_controls.sh --keyboard
```

Expected outcome: each seeded control is keyboard operable and reaches valid response state.

Obvious follow-ons: M13 receives strict Student Response only.

### Milestone M13: Submission, grading commit, and recovery

Depends on: M3, M11, M12.

Parallel-plan ready: no. It joins Student submission lifecycle to worker-owned
grading receipt and owns the recovery boundary.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_submission_recovery.sh
```

### Work package: WP-M13-1 Submission acceptance transaction

Owner: Student submission service engineer.

Touch points: Question Attempt State, Question Submission, receipt, and job preparation.

Depends on: M3, M11, M12.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_submission_recovery.sh --submit
```

Expected outcome: accept one format-valid response and reject a repeat for that Question Attempt.

Obvious follow-ons: return only Student Question Submission Grading State.

### Work package: WP-M13-2 Receipt-based recovery

Owner: grading recovery engineer.

Touch points: typed lease, Automated Grading Receipt, recovery status, and fault harness.

Depends on: WP-M13-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_submission_recovery.sh --fault
```

Expected outcome: interrupt worker, recover same submission, and commit exactly one result.

Obvious follow-ons: M15 reads immutable Gradebook evidence.

### Milestone M14: WeBWorK render and deterministic grade

Depends on: M3, M11.

Parallel-plan ready: yes. The renderer adapter uses shared lease boundary but
remains independent from M12 native response-control work.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_webwork.sh
```

### Work package: WP-M14-1 Renderer issue and render boundary

Owner: WeBWorK adapter engineer.

Touch points: current WebWork adapter, renderer client, cache, and typed Job target.

Depends on: M3, M11.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_webwork.sh --render
```

Expected outcome: answer-free render output and provider inputs redacted from browser data and logs.

Accepted evidence: `bash tests/e2e/e2e_live_demo_webwork.sh --render` exited 0
on 2026-09-07 against an isolated stack after fresh runtime repairs. WP-M14-2
records the completing deterministic-grade and bounded-failure evidence.

Obvious follow-ons: preserve private renderer network isolation.

### Work package: WP-M14-2 Renderer grade and outage path

Owner: WeBWorK adapter engineer.

Touch points: renderer grade call, lease committer, failure mapping, and Student Feedback.

Depends on: WP-M14-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_webwork.sh --grade
```

Expected outcome: deterministic grade commit and bounded renderer outage recovery.

Accepted evidence: `bash tests/e2e/e2e_live_demo_webwork.sh --grade` passed on
2026-09-07 against a fresh controller-managed stack: `WeBWorK grade authority:
deterministic renderer grade commit and bounded renderer failure complete`;
`Live Demo WeBWorK grade: PASS`. The renderer fault is a bounded terminal local
outcome, not a fallback or a claim of M19 serial production-browser acceptance.

Obvious follow-ons: M19 adds route to serial browser scenario.

### Milestone M15: Gradebook and Student Work

Depends on: M13.

Parallel-plan ready: yes. It reads established grading evidence and does not
alter worker or Student submission lifecycle.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_gradebook.sh
```

### Work package: WP-M15-1 Gradebook Store and route

Owner: Gradebook service engineer.

Touch points: Gradebook calculation, Student Work projection, Course Instructor predicate, and audit.

Depends on: M13.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_gradebook.sh --api
```

Expected outcome: current Instructor access and concealed foreign-course result.

Obvious follow-ons: omit raw response, Answer Key, and private grader fields.

### Work package: WP-M15-2 Gradebook browser task

Owner: Gradebook browser engineer.

Touch points: Gradebook page, Student Work page, application shell, and Ribbon capability.

Depends on: WP-M15-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_gradebook.sh --browser
```

Expected outcome: Instructor reaches Gradebook after graded Student workflow.

Accepted evidence: `bash tests/e2e/e2e_live_demo_gradebook.sh --api` and
`--browser` passed on 2026-09-07 against fresh controller-managed fixed HTTPS
stacks. The evidence proves answer-free immutable Gradebook evidence for the
current Course Instructor, foreign-Course 404 concealment, and the visible
Gradebook task. Individual Student Work remains unavailable; this does not
claim M19 serial-browser acceptance.

Obvious follow-ons: M20 captures declared safe projection only.

### Milestone M16: Sysadmin Instructor Account management

Depends on: M4.

Parallel-plan ready: yes. It extends Account management without requiring Course
or Student Record access.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_instructor_accounts.sh
```

### Work package: WP-M16-1 Instructor Account Service

Owner: account service engineer.

Touch points: Sysadmin predicate, Create Instructor Account, Account State Event, and audit writer.

Depends on: M4.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_instructor_accounts.sh --service
```

Expected outcome: role qualification, atomic rollback, and no course-record projection.

Obvious follow-ons: preserve current passkey deferral and seeded entry behavior.

### Work package: WP-M16-2 Instructor Accounts browser task

Owner: account browser engineer.

Touch points: Instructor Accounts page, strict request decoder, and Sysadmin Ribbon task.

Depends on: WP-M16-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_instructor_accounts.sh --browser
```

Expected outcome: seeded Sysadmin completes Create Instructor Account through visible controls.

Obvious follow-ons: M17 uses distinct scoped support authority.

### Milestone M17: Support capability and registered operations

Depends on: M8, M16.

Parallel-plan ready: yes. It composes Course Instance and Sysadmin Account
through narrow capability without changing either base role.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_support_capability.sh
```

### Work package: WP-M17-1 Capability issue and revocation

Owner: authorization service engineer.

Touch points: SysadminSupportCapability, issuer predicate, expiry, revocation, and audit events.

Depends on: M8, M16.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_support_capability.sh --issue
```

Expected outcome: exact Course scope, operation kind, purpose, expiry, and foreign-state denial.

Obvious follow-ons: use registered course roster support for demo journey.

### Work package: WP-M17-2 Scoped support browser task

Owner: support browser engineer.

Touch points: support request page, minimal projection, operation receipt, and Sysadmin Ribbon task.

Depends on: WP-M17-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_support_capability.sh --browser
```

Expected outcome: Sysadmin completes issued roster operation without unrelated Student fields.

Obvious follow-ons: retain capability expiry and audit behavior on reload.

### Milestone M18: Invitation mailing export

Depends on: M9.

Parallel-plan ready: yes. It is an Instructor export boundary that consumes
Course Invitation state and remains outside browser delivery mechanism.

Exit command:

```bash
bash tests/e2e/e2e_live_demo_invitation_export.sh
```

### Work package: WP-M18-1 Protected invitation export route

Owner: invitation export engineer.

Touch points: Course Invitation Store, Instructor Course Membership, JSON export DTO, and cache headers.

Depends on: M9.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_invitation_export.sh --route
```

Expected outcome: only current course Instructor receives declared opaque HTTPS export shape.

Obvious follow-ons: preserve Cache-Control no-store and body-only sensitive data.

### Work package: WP-M18-2 Dry-run mailer contract

Owner: invitation tool engineer.

Touch points: current invitation launcher, export parser, and owner-private status file.

Depends on: WP-M18-1.

Acceptance criteria:

```bash
bash tests/e2e/e2e_live_demo_invitation_export.sh --dry-run
```

Expected outcome: existing mailer accepts export and records no delivery claim.

Obvious follow-ons: retain current limited resend guard.

### Milestone M19: Connected production-browser owner

Depends on: M5-M18.

Parallel-plan ready: no. It owns serial fixed stack, scenario ordering, and
browser evidence for the completion contract.

Exit command:

```bash
./devel/run_playwright_tests.sh --build
```

### Work package: WP-M19-1 Serial scenario owner

Owner: production-browser engineer.

Touch points: Playwright configuration, fixed owner lifecycle, scenario namespaces, and production bundle.

Depends on: M5-M18.

Acceptance criteria:

```bash
./devel/run_playwright_tests.sh --build
```

Expected outcome: rebuild stack, drive visible controls, and report every completion journey.

Obvious follow-ons: use user-intent selectors and real service boundaries.

### Work package: WP-M19-2 Persistence and fault scenarios

Owner: production-browser engineer.

Touch points: reload evidence, second authorized session, worker fault harness, and recovery assertions.

Depends on: WP-M19-1.

Acceptance criteria:

```bash
./devel/run_playwright_tests.sh --build --grep recovery
```

Expected outcome: accepted submission survives worker interruption without a second Student response.

Obvious follow-ons: M20 derives capture routes from successful scenario inventory.

### Milestone M20: Screenshot corpus generation

Depends on: M19.

Parallel-plan ready: no. It owns capture staging and manifest publication after
the serial browser owner proves underlying tasks.

Current authenticated captures belong in the Instructor, Student, and Sysadmin
screen folders that own their visible Product Role surfaces. Pre-authentication
surfaces use `public/`. The Live Demo uses the normal application look and has
no separate `docs/screenshots/live_demo/` gallery. The manifest lists the exact
current paths without enumerating or modifying historical captures.
[SCREENSHOT_CONTRACT.md](../SCREENSHOT_CONTRACT.md) defines the durable
ownership rule.

Exit command:

```bash
./devel/capture_screenshots.sh
```

### Work package: WP-M20-1 Capture manifest and route staging

Owner: visual-evidence engineer.

Touch points: screenshot manifest, viewport profiles, safe route staging, and capture runner.

Depends on: M19.

Acceptance criteria:

```bash
./devel/capture_screenshots.sh
```

Expected outcome: write each declared desktop, tablet, phone, and square artifact once.

Obvious follow-ons: retain desktop Ribbon evidence for every Product Role plus Student responsive profiles.

### Work package: WP-M20-2 Capture privacy and reachability checks

Owner: visual-evidence engineer.

Touch points: browser payload scan, screenshot manifest, and publication directory.

Depends on: WP-M20-1.

Acceptance criteria:

```bash
./devel/capture_screenshots.sh --verify
```

Expected outcome: each capture follows visible navigation and excludes answer,
source, token, and protected Student fields.

Obvious follow-ons: store captures as evidence artifacts, not permanent behavior tests.

### Milestone M21: Authority and documentation close-out

Depends on: M19, M20.

Parallel-plan ready: no. It owns the final cross-cutting contract ledger and
requires feature and capture evidence surfaces.

Exit command:

```bash
source source_me.sh && ./launchers/all_test.sh
```

### Work package: WP-M21-1 Contract and terminology reconciliation

Owner: restoration integrator.

Touch points: durable authorities, subsystem contracts, route registry, browser copy, and tests.

Depends on: M19, M20.

Acceptance criteria:

```bash
source source_me.sh && python3 devel/check_live_demo_authority_ledger.py
```

Expected outcome: no primary-authority contradiction or unsupported capability claim.

The anticipated `devel/check_live_demo_authority_ledger.py` does not exist in
the current tree. M21 therefore records a manual, source-and-evidence-backed
reconciliation in its handoff rather than adding a brittle permanent inventory
check that would not satisfy the permanent-test admission rules.

Obvious follow-ons: record evidence-backed documentation corrections only.

### Work package: WP-M21-2 Final restoration gate

Owner: restoration integrator.

Touch points: aggregate launcher, connected acceptance, browser owner, capture receipt, and changelog.

Depends on: WP-M21-1.

Acceptance criteria:

```bash
source source_me.sh && ./launchers/all_test.sh
./devel/run_playwright_tests.sh --build
```

Expected outcome: both commands exit zero on the same material tree.

Obvious follow-ons: archive this blueprint after evidence ledger completion.

## Rollout checklist

- [ ] M0 records RLS, browser-spec, and client-decoder findings.
- [ ] M1-M4 establish worker topology, readiness, protected lease operations, and replayable seed data.
- [x] M5-M10 establish Instructor Question, Blueprint Course, Course Instance, roster, and Assignment tasks.
- [x] M11-M14 establish Student issue, native controls, receipt recovery, and WeBWorK completion.
- [x] M15-M18 establish Gradebook, Instructor Account, support, and protected export tasks.
- [x] M19 completes serial production-browser owner.
- [x] M20 captures only manifest-listed safe artifacts.
- [x] M21 reconciles authority ledger and required gates.

## Documentation close-out

M21 changes durable documents only when executable behavior and evidence change
their current boundary. It updates [docs/LIVE_DEMO_SPEC.md](../LIVE_DEMO_SPEC.md),
[docs/CONTRACTS.md](../CONTRACTS.md), and
[docs/TEST_EVIDENCE_MODEL.md](../TEST_EVIDENCE_MODEL.md) together with
relevant operation contracts. It preserves historical records as historical
evidence and does not rewrite them into current claims.

Each package records command, result, environment assumptions, one-time probe
status, and remaining boundary in its handoff. A service receipt does not
substitute for browser evidence, and a browser journey does not substitute for a
separate PostgreSQL, object-store, renderer, or worker claim.

## Plan maintenance checks

```bash
source source_me.sh && python3 -m pytest tests/test_markdown_links.py tests/test_ascii_compliance.py
source source_me.sh && python3 -m pytest tests/test_source_file_line_limit.py
git diff --check
git diff --cached --check
```

The plan retains exactly 22 Milestone headings. Every work package retains an
owner, touch points, dependency list, acceptance command, and obvious follow-on.
Every milestone retains a parallel-plan statement and exit command. No completion
criterion relies on image interpretation, external mail delivery, or an
unbounded operator step.
