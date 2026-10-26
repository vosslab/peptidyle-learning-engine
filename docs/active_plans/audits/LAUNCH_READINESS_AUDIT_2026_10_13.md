# Launch readiness audit

Date: 2026-10-13. Reviewed revision: `2b975cc7d16af98855f4006334ddf4e5df099bcd`.
The initial tracked worktree was clean; `.agents/` and `skills-lock.json` were already untracked.

## Readiness decision

**Hold classroom production launch. Student response preservation is the first repair priority.**
The [STUDENT_LAUNCH_READINESS_AUDIT_2026_10_13.md](STUDENT_LAUNCH_READINESS_AUDIT_2026_10_13.md)
follow-up reproduced answer loss during WeBWorK Question navigation, loss after leaving a failed
save, a disabled WeBWorK save-retry path, and false submission status after a lost acknowledgement.
It also found native review missing the original Question prompts. Those findings supersede this
first pass's operational emphasis. Initial Sysadmin setup remains a one-time operational concern.

Offline gates, fresh PostgreSQL acceptance, all 257 live screenshot captures, and actual browser
CSV/TSV grade downloads passed. They did not exercise the interruption and recovery failures above.
The Student follow-up independently confirmed 2X time accommodations and background submission
with the browser closed. The setup findings below remain separate operational work.

Scope is the Fall 2026 pilot defined by [FALL_2026_PILOT.md](../../FALL_2026_PILOT.md) and
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md). No deployed URL or chosen host was supplied, so this
audit reviews the current repository and disposable local stack. It does not authorize deployment,
send messages, freeze the schema, or certify an external installation.

## Operational findings

### 1. Production composition is incomplete

**Blocker for a hosted classroom service; confirmed in source.**

The runtime accepts only `PLE_STORAGE_TOPOLOGY=disposable-local` and constructs storage using a
MinIO endpoint and static per-process credentials. The proposed AWS task definitions supply S3
bucket and KMS values without that topology, endpoint, or credential contract. They also omit the
server's required `PLE_BROWSER_ORIGIN`. Applying the checked-in infrastructure does not produce a
working current API. Local database and browser success cannot close this gap.

Evidence:

- [object_storage.rs](../../../crates/server/src/composition/object_storage.rs), lines 9-22 and 50-89.
- [environment.rs](../../../crates/server/src/composition/environment.rs), lines 157-165.
- [compute.tf](../../../deploy/opentofu/compute.tf), lines 137-197.
- [OBJECT_STORAGE.md](../../OBJECT_STORAGE.md), cloud deployment boundary.
- [MULTI_SERVER_SETUP.md](../../MULTI_SERVER_SETUP.md), production acceptance boundary.

Smallest useful next step: select the actual pilot host and make its service, storage, database,
browser-origin, and private-secret configuration one executable contract. If AWS remains the
target, implement its runtime identity/storage path and align OpenTofu. Rehearse initialization,
content provisioning, backup/restore, rollback, and the normal browser journey on that target
before the human release decision. AWS itself is not a prerequisite imposed by this audit.

### 2. Email wiring still uses obsolete settings

**Blocker for ordinary onboarding/sign-in; distinct from obtaining the mailbox.**

The current server adapter consumes these three settings:

```text
PLE_INSTRUCTOR_SETUP_SMTP_URL
PLE_INSTRUCTOR_SETUP_FROM
PLE_BROWSER_ORIGIN
```

The URL must use `smtps` or `smtp` with `tls=required`. The origin is the canonical HTTPS browser
origin. The API Compose service does not pass the two SMTP adapter settings, while `env.example`
describes an absent optional SMTP overlay and obsolete `PLE_SMTP_*` settings. OpenTofu uses those
obsolete settings too. Filling in the supplied example would therefore leave the adapter
unconfigured; email-code routes are conditional on the configured adapter.

Evidence:

- [environment.rs](../../../crates/server/src/composition/environment.rs), lines 117-130.
- [api_assembly.rs](../../../crates/server/src/composition/api_assembly.rs), email-code route wiring.
- [env.example](../../../containers/env.example), lines 61-73.
- [compose.yaml](../../../containers/compose.yaml), API environment beginning at line 370.
- [compute.tf](../../../deploy/opentofu/compute.tf), lines 150-155 and 183-192.

Smallest useful correction: wire the actual adapter settings through the selected runtime and
replace the stale examples. The human can still obtain provider credentials today. Acceptance
then requires a real Instructor setup notice, delivered sign-in code, and a fresh rostered
Student sign-in/invitation claim at the intended origin. Student invitation mailing remains a
separate attended Mail.app workflow in [COOKBOOK.md](../../COOKBOOK.md); the server SMTP settings
do not replace that invitation step. No email was sent during this audit.

### 3. First administrator bootstrap is unestablished

**High; missing production procedure and acceptance, not proof that database bootstrap is impossible.**

Instructor creation requires a Sysadmin. The exposed credential-provisioning operation is
`--provision-local-sysadmin-totp`; its supported flow is wired to the seeded Morgan Account and
local-demo TOTP material. The project-tools command dispatcher exposes no real-account administration command,
and the current installation guide does not specify first production Sysadmin creation,
authentication enrollment, or recovery.

Evidence:

- [application.rs](../../../crates/server/src/application.rs), lines 10-35 and 99-100.
- [composition_live_demo.rs](../../../crates/server/src/composition_live_demo.rs), lines 135-170.
- [application.rs](../../../crates/project-tools/src/application.rs), lines 42-64.
- [INSTALL.md](../../INSTALL.md), database initialization and installation-content procedures.

Smallest useful correction: define and exercise one audited bootstrap for the first real Sysadmin,
then create and sign in as a real Instructor. Keep demonstration identities scoped to the demo.
Include this in the chosen deployment procedure rather than inventing another general admin system.

### 4. Course delivery scope exceeds installed content

**High for the full stated first delivery; conditional on the selected initial teaching activity.**

The pilot authority names Genetics, Biochemistry, and a selected Biotechnology collection.
Current installation includes a substantial Genetics example: 41 canonical PGML sources across
nine nonempty topics. The separate Chapter 1 pilot contains four Genetics and four Biochemistry
Questions. No Biotechnology installation manifest was found. The Chapter 1 pilot is not the
entire installed Genetics collection, and upstream course completeness is not installed-course
completeness.

Evidence:

- [FALL_2026_PILOT.md](../../FALL_2026_PILOT.md), lines 55-66.
- `content/genetics/README.md` and
  `content/genetics/manifest.yaml`, active canonical Genetics input.
- `content/pilot/chapter_1_assessments.yaml`, eight-question pilot.
- [installation_data.rs](../../../crates/project-tools/src/installation_data.rs), lines 68-98 and
  265-279, publication sequence and fixed Genetics input.

Smallest useful next step: name the actual Courses and Assessments for first classroom use and
verify their published content, release, Student submission, and grade export. A deliberately
small Genetics pilot need not wait for every planned course. Full three-course delivery needs
the remaining reviewed content and ordinary publication.

### 5. Readiness documents contain false blockers

**Medium; confirmed documentation drift.**

- [INSTALL.md](../../INSTALL.md), lines 117-176, and [LIVE_DEMO_SPEC.md](../../LIVE_DEMO_SPEC.md)
  still say Bloom blocks installation-data publication. Human Guidance explicitly defers AI/Bloom
  and permits publication without it. Current installation code invokes ordinary publication, and
  the fresh stack completed installation-data provisioning during this audit.
- [TODO.md](../../TODO.md), lines 344-347, calls grade export unbuilt. The authorized route,
  CSV/TSV encoder, and browser download controls exist in
  [live_gradebook.rs](../../../crates/server/src/live_gradebook.rs),
  [export.rs](../../../crates/server/src/live_gradebook/export.rs), and
  [gradebook_page.tsx](../../../src/pages/course/gradebook_page.tsx). Both browser download controls produced
  valid, nonempty grade files in the one-time check below.
- The retained [human_guidance_implementation_checklist.md](human_guidance_implementation_checklist.md)
  labels much evidence stale/pending and cites removed tests. Historical checklist status is not
  current acceptance evidence.

Smallest useful correction: reconcile the operator and readiness documents against current
commands and the receipts below. Preserve genuine pending work without recommissioning implemented
features or requiring the deferred AI backend.

### 6. YAML dependency maintenance advisory

**Informational; not a demonstrated launch vulnerability.** Finding `NDC-2026-001`.

OSV Scanner 2.7.0 scanned 413 Rust and 199 Node lockfile packages. Its only finding was
[RUSTSEC-2024-0449](https://rustsec.org/advisories/RUSTSEC-2024-0449.html): `unsafe-libyaml 0.2.11`
is unmaintained, with no patched version and no CVSS/CWE exploit claim. `cargo tree --locked -i
unsafe-libyaml` traces it through `serde_yaml_ng 0.10.0` to project-tools and acceptance-runtime;
the inspected callers parse operator-controlled curriculum, pilot, and acceptance manifests.
The scanner did not identify a vulnerable public API execution path. Confidence is high in the
dependency/maintenance finding; exploitability was not established. This falls under dependency
maintenance (OWASP A06), not a proven security incident.

Keep manifests trusted and plan a separately validated maintained YAML-parser replacement. There
is no safe patch-only update command to offer for this advisory. Recheck the current resolution
with `cargo tree --locked -i unsafe-libyaml` and OSV before choosing that change. No dependency was
updated during the audit. The scanner was downloaded from the
[official 2.7.0 release](https://github.com/google/osv-scanner/releases/tag/v2.7.0) and checked against
its published SHA-256 digest; it was not installed globally.

## Evidence from this audit

The ignored `output_launch_audit/` directory retains command logs. This report records conclusions
so they remain reviewable after local logs are cleaned.

| Check | Result | Claim and limit |
| --- | --- | --- |
| `source source_me.sh && ./launchers/all_test.sh` | Offline gates passed; connected step initially blocked | Schema style, Rust default/all-feature checks and tests, strict Clippy, Wasm target, TypeScript, lint/format, Node tests, and 9,072 pytest cases passed. PostgreSQL lane stopped at an existing browser-suite lease. |
| Owned Live Demo stop | Passed | The lock holder was an orphaned supervisor; stack inspection found no containers, networks, or volumes. The repository stop command released it. |
| `source source_me.sh && python3 local_stack.py acceptance` | Passed | Fresh PostgreSQL initialization/replay, selected authority/persistence boundaries, and the destructive Unrelease lock race. Owned cleanup completed. |
| Screenshot static verification | Passed | Tracked corpus consistency only. |
| `source source_me.sh && ./devel/capture_screenshots.sh --verify` | Passed | Fresh stack provisioning and all 257 captures passed manifest closure, scenario privacy, and published-artifact integrity checks. Includes Student save/reload/resume/submit/history workflows. Tracked images were unchanged. |
| One-time Gradebook browser download check | Passed | Instructor CSV and TSV controls each returned HTTP 200, an attachment, the expected media type, `nosniff`, and `no-store`. Both downloaded files parsed as 15 synthetic rows with seven consistent columns, including earned/possible points. |
| Owned audit Live Demo stop | Passed | The repository stop command cleaned up the audit stack after browser checks. |
| `cargo tools pilot-content` | Passed | Two Chapter 1 Assessments, eight reviewed Questions, four adapted PGML sources. |
| Genetics curriculum validation | Passed | Nine topics and 41 canonical sources; this validates inputs, not renderer behavior or deployed teaching. |
| `npm audit --json` | Passed | 199 resolved Node dependencies; zero reported advisories. |
| OSV scan of `Cargo.lock` and `package-lock.json` | One informational advisory | 413 Rust and 199 Node packages; only the unmaintained YAML parser above. |
| OSV scan of installed direct Python runtime dependencies | Passed | PyYAML 6.0.3, py-applescript 1.0.3, and the separately installed podman-compose 1.6.0 CLI; zero advisories. Python transitive/developer dependencies, container layers, and separately vendored WASM dependencies were not scanned. |

The Gradebook check was a one-time probe, not a new permanent test. Its receipt is retained in
`output_launch_audit/grade_export_receipt.json`; the temporary probe was removed. Visual spot checks
covered the fresh desktop Instructor Gradebook and phone Student Attempt History. The replay
reported 254 byte-different images, which remain available in `test-results/screenshot-corpus/verify/`
for review; byte equality is not the acceptance gate. This was not a manual review of all 257 images.

The current aggregate dispatches the database baseline only. It does not silently include browser,
object-store, installation-data, worker, or hosted acceptance. See
[TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md) and
[acceptance_lanes.py](../../../local_stack_control/acceptance_lanes.py).

## Review coverage and exclusions

All six fresh independent passes completed: Plan, Test, Style, Docs, Legacy, and Comment. Their
overlapping deployment/email findings were merged and their source claims checked. No pass returned
an entirely empty findings list. No production code, schema, dependency, or permanent test was
changed for this audit.

The coordinator rejected or narrowed these preliminary interpretations:

- Grade export is implemented; its stale TODO is not an implementation blocker.
- The screenshot replay includes meaningful Student save/reload/resume/submit/history behavior;
  the small retained browser registry does not describe all available runtime evidence.
- Genetics installation extends beyond the eight-question Chapter 1 pilot.
- Passkeys are not made a separate immediate blocker: current Human Guidance permits passkeys
  **or** email codes. Email-code delivery remains essential.
- Automated daemons and AI remain explicitly deferred until the final server location. Retention
  source exists; it is not accurately described as wholly unimplemented. Its deployment, notice
  delivery, and recovery/deletion acceptance belong to the selected hosting work.
- Cosmetic naming, unused optional contrast styling, and old comment tags do not block launch.

Residual acceptance includes the actual hosting environment, real email/invitation delivery,
first real-account bootstrap, backup/restore/rollback, the selected initial course set, and
representative assistive-technology use. Existing accessibility reports mark those sessions
incomplete; this audit does not claim full accessibility or comprehensive security certification.

## Recommended execution order

1. Correct Student answer preservation and retry behavior identified in S1-S3 of the Student audit,
   then verify truthful submission status and useful review context. The human can obtain the email
   provider account while runtime SMTP wiring is aligned.
2. Choose the pilot host and first teaching scope, then complete the runtime and one-time first-Sysadmin
   setup for that target.
3. Publish the selected course content and prove the normal real-account teaching loop, including
   saved responses across reload, submission, visible scores, and Instructor CSV/TSV export.
4. Rehearse backup/restore and recovery on that installation, resolve material acceptance failures,
   and record the human go/no-go decision that freezes the base schema.
