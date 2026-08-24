# Peptidyle security architecture audit

## Audit status

- **Audit date:** 2026-08-12
- **Audit type:** end-to-end security architecture audit and remediation re-audit
- **Repository verdict:** complete for the reviewed repository-owned design; no open P0--P2
  architecture finding remains after independent re-reviews
- **Production-activation verdict:** not yet authorized; live deployment, provider, recovery, and
  operational evidence listed below are release gates
- **Product state:** pre-production; the canonical live-demo path uses fictional, disposable live
  data and has no production users or production data
- **Priority:** authority, isolation, and attack-surface design; dependency versions are secondary

This replaces the dated baseline rather than preserving its conclusions. The active release plan,
current source, migrations, contracts, focused tests, and independent reports are the source of
truth. A successful code or static-policy test does not prove an AWS, PostgreSQL, browser-edge, or
third-party deployment property. The single canonical live-demo path is production-shaped, but its
seeded people and records are fictional live data and its regeneration is disposable. This document
therefore labels evidence as **code**,
**integration/static configuration**, or **live deployment**.

The remediation intentionally makes clean breaks. It retains no legacy cookie aliases, stateful
GET routes, embedded `SameSite=None` session mode, mixed storage bucket, or weak publisher path
merely to support hypothetical users. Published content remains immutable.

## Scope and method

The review traced authority and data through:

- browser, Solid UI, TypeScript DTO decoders, URL fragments, browser storage, and the Rust/Wasm
  boundary;
- email/passwordless and passkey ceremonies, account and tenant sessions, cookies, logout,
  invitations, rate limits, Host/Origin handling, CSRF, CSP, and unsafe methods;
- Rust routes, service composition, store capabilities, authorization failure responses, grading,
  replay/idempotency, concurrent state transitions, workers, and telemetry;
- PostgreSQL TLS/login attestation, role membership, capability roles, RLS, broker functions,
  transaction-local tenant context, learner/instructor authority, and retention;
- QTI/flat content, ZIP/XML/image decoding, source checksums, publication, asset registry,
  object keys, signed delivery, CDN exposure, SSE-KMS, IAM, and retention;
- iMathAS and WebWork boundaries, opaque-origin sandboxing, provider activity/finalization,
  timeouts, persistent uncertainty, and external-service egress;
- local Podman/Caddy, AWS/OpenTofu CloudFront/ALB/WAF/VPC/ECS/RDS/S3/KMS/Secrets Manager design,
  container confinement, network reachability, logs, shutdown, backup, and restore.

The review used current repository evidence, adversarial focused tests, migrations, configuration
policy tests, and independent re-reviews. It did not apply AWS, provision a disposable production
PostgreSQL cluster, connect a real SMTP/LMS/provider, run the final current full Compose topology,
or exercise a released HTTP edge. Those omissions are explicit live gates, not silent passes.

## Threat actors and protected assets

| Actor or failure mode | Relevant goal | Principal design response |
| --- | --- | --- |
| Anonymous browser, bot, or malicious page | Consume compute, enumerate identities, fixate a session, or cause CSRF | Uniform passwordless start, scoped quotas, pre-allocation passkey limit, exact Host/Origin, and non-GET mutation policy |
| Authenticated Student or Instructor | Cross tenant, course, role, or retention boundary | Session-derived tenant/actor, Store-level active-membership/direct-Instructor predicates, RLS, opaque concealment, and atomic state transitions |
| Malicious browser, extension, or authored markup | Read answers, execute script, steal a token, or abuse a privileged API | Answer-free DTO/Wasm boundary, strict runtime decoders, inert markup projection, restrictive CSP, no raw secrets in browser storage |
| Concurrent request or replica | Double submit, use revoked authority, publish an orphan, or duplicate an external effect | Database locks/conditional state, request-bound idempotency, leases, pending publication outbox, and indeterminate provider fence |
| Compromised API, worker, renderer, or external provider | Confused deputy, broad database/object/cloud access, SSRF, or secret disclosure | Fixed service contracts, isolated roles/domains, dedicated publisher, no-NAT production design, narrow egress, and disabled renderer activation until attested |
| Deployment/configuration error | Downgrade TLS, grant extra role/IAM power, expose origin/private assets, or leak backup | Fail-closed production settings and attestation, four storage/KMS domains, edge-only HSTS, explicit AWS policies, and release probes |

Protected assets include account/session authorities; tenant, roster, run, response, and grade data;
answer keys and private feedback; unpublished sources/import archives; private and student-record
objects; immutable published bytes; object/KMS/IAM authority; provider launch state; operational
secrets; and forensic/audit records.

## Trust boundaries and current guarantees

| Boundary | Enforced design | Evidence tier and remaining limit |
| --- | --- | --- |
| Browser to API | Production requires one configured HTTPS Host, one exact Origin for cookie-authenticated mutations, `__Host-` sensitive cookies, duplicate rejection, no-store, and a fail-closed method inventory. | Code tests cover hostile Host/Origin/cookies and route policy. Actual CDN/ALB headers and error paths need live proof. |
| Browser/Wasm to grading | Browser DTOs and Wasm exports are answer-free; server re-resolves immutable grading context and issues receipts. | Code and answer-free corpus/parity tests. A future export still requires the same review. |
| Account session to tenant/role | Opaque hashed account/session rows and persisted membership derive tenant and actor; browser claims do not select either. | Code and store conformance evidence. Live passwordless delivery/WebAuthn relying-party setup remains required. |
| Student/Instructor/Sysadmin to records | Student capabilities require active Student membership; Instructor teaching paths require direct course membership. Sysadmin has only closed audited roster support and payload-free retention operations, never general FERPA-record authority. PostgreSQL locks membership with `FOR KEY SHARE`. | Memory/PostgreSQL parity and route tests. Real PostgreSQL revocation interleavings remain a live gate. |
| API to PostgreSQL | Production URLs require `sslmode=verify-full`; fixed logins and capability roles are attested for flags, direct membership, and delegation metadata. Tenant work uses `SET LOCAL ROLE` and transaction-local context. The database table map marks direct and linkage-bearing FERPA relations radioactive and carries the label into derived or recovery copies. | Unit/migration/conformance evidence. Disposable production-login/RLS execution and deployed backup-access evidence are still required. |
| API to objects/CDN | Typed keys, four storage domains, exact public URL parser, Post-only protected delivery, checksum/KMS validation, and immutable tagged public assets constrain access. | Code and static AWS policy evidence. Real IAM/KMS/S3/CloudFront denial probes remain required. |
| Publication to public delivery | A private source and registry/outbox transaction create `Pending`; only dedicated publisher authority writes the immutable public copy and activates it. Pending objects are concealed before audit or signing. | Code, migration, policy, and independent re-review evidence. Live publisher task/IAM proof remains required. |
| API/worker to provider | Deployment-configured but process-fixed HTTPS endpoints, fixed paths, redirects/cookies disabled, bounded responses, and no request-selected destination. | Code evidence. Live DNS/egress/provider behavior and renderer attestation remain required. |
| Browser to external tool | POST creates launch; GET is an inert shell. Opaque-origin `Origin: null` is allowed only for the exact capability-bound activity POST; route rechecks tenant, actor, attempt, proof, and lease. | Code and conformance evidence. Real LMS/provider browser flow remains required. |
| API/worker/renderer containers | Non-root/read-only/capability-free/no-new-privileges profiles, bounded tmpfs/resources, private networks, digest-pinned renderer default, server-generated safe telemetry, and graceful drain. | Static Compose tests and an implementation-time socket probe. Timing-driven probes were not retained in the permanent fast suite; live Podman/image provenance proof remains required. |
| Internet edge to private workloads | OpenTofu specifies CloudFront-to-ALB origin authentication, private Fargate, no NAT, VPC endpoints, separate task roles, restrictive security groups, WAF observation, and edge-owned HSTS. | Static configuration and policy tests only; direct-origin, cache/parser, TLS, egress, and WAF behavior need live proof. |

## Verified security guarantees

### Authentication, sessions, and browser authority

- Passwordless bearer, browser-binding, and WebAuthn ceremony values are 256-bit random values
  retained as hashes. Their atomic consume predicates include the browser binding, so a wrong
  binding cannot burn a valid challenge. Correct concurrent completion still has one winner.
- Production mounts session resolution and complete logout with the passwordless routes, while the
  local credential provider stays behind an opt-in Rust feature and local image target. The ordinary
  browser artifact also excludes the local form and login transport. Logout revokes tenant and
  account sessions before clearing account/session/binding cookies; path-scoped launch proofs become
  unusable through that revocation. Failure leaves cookies for a truthful retry.
- Scoped HMAC-based email, network, principal, and service budgets prevent a recipient-only quota
  from becoming the sole mail-denial control. Successful mailbox proof clears only its own email
  quota. Anonymous passkey starts spend a network budget before ceremony persistence.
- `SameSite=Lax` is retained for the email-link landing flow, but it is not the CSRF defense. Exact
  Origin and POST-only state transitions are. The old embedded `SameSite=None` mode was removed.
- Global response headers are no-store, nosniff, referrer/permissions/frame/CORP protections, and
  CSP fallback; route-owned nonce CSP is preserved rather than overwritten. HSTS is intentionally
  owned by the HTTPS edge, not API middleware.

### Authorization, tenancy, and grading

- Tenant context comes only from the opaque server session. Course context derives from persisted
  account membership, not a browser tenant/role claim. Object, run, attempt, enrollment, and
  receipt authorization re-resolve the server-owned relationship and conceal foreign access.
- Tenant tables use forced RLS except deliberate public catalog and tenant-free aggregate
  projections. Grader-only functions have fixed search paths and no public execution. RLS is a
  tenant fence, not a substitute for per-actor route/store authorization.
- Every learner-visible Store read or mutation rechecks active Student membership and accessible
  course state. PostgreSQL serializes that check against roster revocation. Separate Instructor
  capabilities preserve legitimate direct-course historical access without a Student fallback or
  a Sysadmin bypass.
- Manual grading returns response-bearing evidence only through one current-instructor,
  retention-checked Store operation; it does not authorize, then perform a raw second read.
- Submission, invitation, run issuance, manual grade revision, and worker/publication transitions
  bind durable identity/idempotency/locks to the invariant being changed. The browser never receives
  answer keys or grading implementations.

### Content integrity, object isolation, and publication

- Flat source is strict/canonical and size-bounded; its SHA-256 binds public/private compilation,
  immutable source/version records, issued presentation, and revalidation at publication. WebWork
  definitions/replay maps and QTI imports have equivalent source/digest chains.
- SHA-256 detects changed bytes, including a subordinate-store accident or corruption. It is not an
  authenticity signature against an authority able to change both bytes and metadata. RLS, IAM/KMS,
  immutable publication, audit/reconciliation, and separated publisher authority supply that
  authenticity boundary.
- QTI archives reject traversal, links, duplicate/absolute/backslash paths, expansion/resource
  abuse, and unsupported XML shapes. QTI and hotspot raster ingestion share one validator: PNG,
  JPEG, or WebP only; byte/pixel/allocation limits; full decode; animation refusal; and strict
  terminal-container checks, including JPEG trailing/polyglot rejection.
- Browser dictionary decoding uses a null prototype, own-property reads, and rejects prototype
  control names. Author markup is inertly parsed and rebuilt from narrow DOM/Math/URL allowlists.
- `ProblemAsset` alone uses `PublicAssets`; immutable scope selects `RestrictedProblemAsset` in
  `PrivateContent` for institution content. QTI and flat publication/reconstruction derive exactly
  one physical target from immutable scope. A known restricted key cannot become CDN-readable.
- Four physical domains (`PublicAssets`, `PrivateContent`, `StudentRecords`, and `TempProcessing`)
  use distinct bucket/KMS settings. Source/temp keys never sign. Protected delivery is an audited,
  authorized POST that returns a bounded URL; public GET redirects only immutable activated assets.
- Public bytes are not written before catalog publication. Private source validation accepts only
  exact workspace asset kinds; transactional outbox work is claimed by a dedicated publisher login,
  service, and narrow IAM role. `Pending` is 404 for both public GET and protected POST, before
  audit/signing. API/ordinary worker cannot write, delete, or retag public immutable bytes.

### External tools, containers, deployment, and operations

- External launch creation is same-origin POST; its later shell GET is inert. Provider markup runs
  inside an opaque-origin sandbox with nonce CSP and `frame-ancestors 'self'`. Browser output never
  contains provider URL, credential, handle, score, raw result, or grading payload.
- Activity/verification leases serialize ordinary provider activity, finalization, revocation, and
  grade commit. An effectful provider POST writes an exact, live-lease pre-dispatch marker before
  network I/O. Timeout, crash, malformed response, or loss of response leaves an attempt-wide
  indeterminate fence: no retry, relaunch, finalization, or revoke may assume the effect failed.
  Current signed-grade retrieval is explicitly safe/idempotent GET-only.
- API request IDs are server-minted 128-bit base64url values; completion logs contain only ID,
  method, status, and elapsed time, never paths, queries, headers, bodies, identities, answers,
  URLs, object keys, or secrets. SIGTERM/SIGINT stops admission and drains bounded in-flight work.
- Local Caddy and Compose explicitly use an HTTP/1.1 path with parser/time limits, route API before
  static fallback, private networks, and constrained runtimes. Production OpenTofu adds private
  compute/data subnets, no NAT, endpoints, four bucket/KMS domains, RDS encryption/PITR, separate
  API/worker/publisher secrets and task roles, CloudFront/ALB origin defense, WAF observation, and
  scoped external egress.

## Findings and closure record

Severity is the risk at discovery. `Resolved` means the repository now enforces the design and has
the listed code/static evidence. It does not turn a live deployment gate into a completed probe.

| ID | Discovery severity | Root design problem | Final status |
| --- | --- | --- | --- |
| AS-01 | Medium | Browser binding was checked after deleting an email/WebAuthn ceremony, enabling availability denial. | **Resolved.** Binding is in the atomic consume predicate in Memory/PostgreSQL; wrong binding cannot consume it. |
| AS-02 | Low | Recipient-only passwordless quotas enabled targeted mail denial and authenticated mail abuse. | **Resolved.** Independent email/network/principal/service scopes and recovery-safe quota clearing are enforced. |
| SD-01 | High | Production did not compose full session/logout lifecycle; account session could survive tenant logout. | **Resolved.** One production identity graph owns session and dual-session revocation; local provider login is excluded. |
| SD-02 | High | Client network identity was a spoofable browser-visible custom header. | **Resolved.** Trusted peer/CIDR XFF policy walks right-to-left, bounds malformed chains, and normalizes IPv4/IPv6. |
| SD-03 | High | PostgreSQL URL, login, effective capability role, and delegable membership drift were not fail-closed. | **Resolved.** Verified TLS/fixed identities and exact NOLOGIN/non-privileged/non-delegable capability attestation now gate production pools. |
| SD-04/OI-01/OI-02 | Critical | One mixed storage/KMS domain and static credentials made CDN/IAM separation conventional. | **Resolved in repository design.** Four typed domains, workload identity, SSE-KMS checks, narrow IAM, and a separate public publisher replace the convention. |
| SD-05 | High | Anonymous passkey start could allocate persistent work without application cost control. | **Resolved.** Trusted coarse-network budget runs before persistence; deployment WAF/load calibration remains live work. |
| SD-06/SD-09 | High | Cookie/Host/Origin/security-header boundary was incomplete and HSTS was assigned to the API. | **Resolved in code/configuration.** Exact browser boundary and headers are central; HSTS moved to the edge contract. |
| SD-07 | High feature blocker | Embedded `SameSite=None` sessions had no origin-bound mutation defense. | **Resolved by removal.** No embedded credential mode remains; narrow sandbox `Origin: null` is capability-bound. |
| SD-08 | High feature blocker | Provider HMAC secrets accepted arbitrary nonempty bytes. | **Resolved.** Strict canonical 32-byte decoding and fixed HTTPS transports are required. |
| GET-CSRF launch | High | GET created external launch state under Lax cookies. | **Resolved.** POST alone creates launch; GET is inert and proof-bound; legacy path is absent. |
| GET capability delivery | High | GET authorized/audited and minted protected object bearer URLs. | **Resolved.** Protected issuance is exact-Origin POST; public GET is immutable redirect only. |
| External lease/finalization | High | Provider activity could race revocation/finalization or be released by stale replicas. | **Resolved.** Durable hashed leases, exact release, verification/finalization fences, and atomic commit/revoke serialize the state machine. |
| EXT-01 | High | A process crash/timeout after a non-idempotent upstream POST could permit a duplicate retry. | **Resolved with fail-closed recovery.** A live-lease pre-dispatch marker permanently fences uncertain attempts; only safe GET result retrieval is retryable. Provider-specific operator reconciliation is deferred. |
| CI-01/DOI-04 | High/P1 | QTI and native image paths differed; JPEG trailer acceptance was not proven. | **Resolved.** One strict still-image ingress validator fully decodes and rejects unsupported/animated/trailing containers. |
| Prototype pollution | Low | Generic dictionary decoding admitted prototype-control keys. | **Resolved.** Null prototype, own-property checks, and dangerous-key refusal are permanent behavior. |
| DOI-01 | P0 | Institution content could physically use the CDN-readable public key/bucket. | **Resolved.** Immutable publication scope selects public versus restricted typed key; inverse records refuse. |
| Public orphan | P1 | Candidate public bytes could exist before database publication and remain CDN-visible after a failure. | **Resolved.** Private source plus post-commit pending/outbox publisher prevents pre-activation public bytes. |
| DOI-02/03/05 | P0/P1 | IAM prefixes drifted from typed paths, public-tag policy initially denied writes, and local/e2e names were stale. | **Resolved.** Exact operation/prefix policies, public-tag SDK behavior, local domain configuration, and publisher-only public writes are tested. |
| Human role ambiguity | P1 | Coarse Administrator/Publisher roles and broad Sysadmin course bypasses could grant ambient access to FERPA records. | **Resolved.** The closed human set is Student, Instructor, and Sysadmin. Direct membership owns teaching access; Sysadmin crosses only audited roster-support and payload-free retention boundaries. Publisher is a service identity/action. |
| Learner authority | P1 | Retained enrollment/attempt records could outlive active Student authority; some reads were route-convention only. | **Resolved.** Actor-scoped Store capabilities enforce active membership/accessibility and serialize revocation; Instructor history is explicit. |
| Manual grade read | P2 | Instructor authorization and response retrieval occurred in separate transactions. | **Resolved.** One locked current-instructor Store read returns evaluation and projected response atomically. |
| Export authority | Medium | Export requester identity/roster authority could be a route-level convention. | **Resolved.** Session-derived actor, roster lock, atomic request/job creation, and requester-only status are enforced. |
| DN-01/DN-02 | High | API/worker/renderer containment was image convention rather than runtime policy. | **Resolved in configuration.** Rootless hardening, private networks, limits, and immutable renderer default are declarative and tested. |
| DN-03/DN-04 | High | Production edge, IAM, egress, and parser controls were absent. | **Resolved in declarative design.** OpenTofu defines private/no-NAT workloads, edge origin defense, scoped IAM/KMS/secrets/egress, and cache/header policy. Live proof remains required. |
| SD-11/SD-12 | Medium | Telemetry could expose attacker data and termination had no deterministic bounded-drain proof. | **Resolved.** Safe server-minted IDs/metadata-only telemetry are tested. Bounded graceful-drain behavior was exercised during implementation; its real socket/timing probe is one-time evidence rather than a permanent fast test. |

The initial independent reviews correctly identified the GET, mixed-storage, external timeout,
membership, publication, JPEG, and IAM defects. Later implementation and independent re-reviews
supersede their interim open statuses. No finding above is silently downgraded because an interface
became inconvenient; the stronger pre-production design was implemented instead.

## Completed design changes

1. Replaced development-shaped authentication with a production passwordless lifecycle, exact
   browser boundary, revocable sessions, atomic ceremonies, and composite abuse controls.
2. Added a fail-closed route/method policy and removed every reviewed state-changing GET.
3. Moved Student, Instructor, export, grading, and external authorization to Store/database
   capabilities that receive session-derived actor/tenant authority, not route conventions.
4. Attested real PostgreSQL effective authority, not only a connection's login name; made transport
   validation, capability roles, and membership delegation explicit.
5. Split object data into four KMS/IAM domains, made physical public/restricted scope typed, and
   made publication post-commit through a dedicated database/IAM publisher.
6. Centralized hostile image validation, archive bounds, and TypeScript dictionary hardening.
7. Turned external provider uncertainty into a durable safety fence rather than assuming local
   request cancellation stopped a remote mutation.
8. Added container/edge limits, safe telemetry, graceful drain, and a repository-owned AWS baseline
   with no-NAT private workloads, edge origin protection, and least-privilege task identities.

## Evidence and tests

### Code evidence

- `./check_rust.sh`: passed Rust-owned contract and fixture generation, formatting, default and
  all-feature compilation, strict Clippy, workspace tests and doctests, and the browser Wasm target.
- `cargo test -p server_core --lib --quiet` passed; environment-gated cases remained ignored.
- `cargo test -p learning-data-access --lib --all-features` and the conformance suite passed;
  live PostgreSQL cases remained environment-gated.
- Object unit and conformance suites passed; the live MinIO case remained environment-gated. The
  full workspace all-features gate and strict all-target Clippy passed.
- Focused browser, route, asset, image, publication, passwordless, PostgreSQL-attestation, external
  launch/submission, Instructor/Student, and export tests exercise the negative oracles named above.
- Object/S3 policy units: 30 passed. Native/QTI strict raster tests cover valid supported types and
  adversarial GIF, animation, truncation, oversize, path, and polyglot cases.
- TypeScript runtime decoder/client/renderer tests cover prototype controls, hostile responses,
  protected delivery POST, and answer-free browser contracts.

### Integration and static-configuration evidence

- Existing local Caddy/Compose topology and renderer boundary tests passed. New exact-value source
  scans used while rebuilding containment were removed from the permanent fast suite. Rendered
  configuration and runtime isolation remain integration evidence when the local service runtime is
  available.
- PostgreSQL migrations, role matrices, store parity, and production connection attestation compile
  and test offline. The live PostgreSQL cases remain ignored without a disposable server.
- OpenTofu source includes policy tests for private/encrypted RDS, domain buckets/KMS, task role
  separation, CloudFront/ALB origin controls, publisher isolation, and execution-secret conditions.
  Implementation ran `tofu fmt`, `init -backend=false`, `validate`, and `test`; a later independent
  re-review statically inspected the final configuration because OpenTofu was unavailable locally.
- TypeScript compilation and the fast Node suite passed. The slower emitted-artifact proof that
  local credential UI/transport is absent from the ordinary browser build and present only in the
  explicit local build now lives in the non-browser E2E tier. Three focused external-tool
  Playwright tests passed. The Python hygiene/documentation suite and scoped `git diff --check`
  gates passed throughout remediation. Exact historical case counts are implementation evidence,
  not permanent acceptance contracts.

### Live-deployment evidence required

None of the following has been claimed complete: CloudFront/ALB/DNS/TLS behavior; ECS task identity
and egress; real S3/KMS/IAM allow/deny paths; RDS TLS/role/RLS/lock behavior; backup restore/KMS
revocation; provider/LMS/SMTP/WebAuthn delivery; renderer provenance/containment; or real edge
request-smuggling/cache behavior.

## Remaining risks, assumptions, and intentional deferrals

These are production-activation gates or intentional feature boundaries, not open repository P0--P2
design defects.

1. **AWS deployment proof:** apply to a disposable account as a controlled deployment exercise and
   test direct-ALB denial, OAC-only
   public reads, no public/private cross-bucket access, no list/overwrite/delete/tag removal,
   SSE-KMS exact-key enforcement, secret/KMS context constraints, no static credentials, no NAT,
   metadata/control-plane denial, configured SMTP/iMathAS/renderer-only egress, HSTS/CSP/cache
   headers on successful and edge-error paths, and WAF behavior.
2. **Database proof:** create the exact production logins and a disposable RDS-like cluster. Prove
   `verify-full` TLS, startup rejection of privilege/membership drift, forced RLS/broker grants,
   live learner/instructor revocation races, export/manual-grade authority, and external/publication
   concurrency. Exercise the audited real-person Instructor/Sysadmin approval procedure and prove
   the API login cannot change `platform_roles`. Ignored live PostgreSQL tests do not establish
   those facts.
3. **Recovery proof:** run an encrypted backup and managed PITR restore exercise into a clean cluster;
   validate migration ledger, roles, grants, RLS, logical data fingerprints, application writes,
   broker calls, retention, KMS-key revocation/recovery, and declared RPO/RTO.
4. **External systems:** live SMTP sender and email-link flow, WebAuthn RP/origin, LMS/provider
   launch/POST/GET behavior, redirect/cookie rejection, timeout/indeterminate support workflow,
   and production provider egress must be tested. An indeterminate effectful provider POST stays
   unavailable until a documented instructor/support reconciliation based on provider evidence; it
   must never receive an automatic learner retry or generic database clear.
5. **Renderer activation:** the external production WebWork renderer remains disabled until a
   separately reviewed digest/provenance/health contract proves no public IP, database/object/task
   credentials, Internet/NAT path, or unauthorized egress. Local renderer restrictions do not prove
   its production image.
6. **Container/edge runtime:** Podman later answered `info`, but Compose configuration could not
   connect because of a stale machine URI. Run live inspect/configuration/provenance checks and a
   safe parser/cache regression suite against the actual protocol chain after each edge change.
7. **Dependency inventory:** package versions/advisories remain a secondary continuous-maintenance
   concern. They should be reviewed before release, but no dependency finding displaced an
   architectural authority/isolation correction in this audit.

## Encryption, checksums, and passwordless decisions

### Content validation and checksums

For a long WebWork question, PLE binds strict source bytes and SHA-256 to immutable source/version
records, the issued/replay/grading identity, renderer identity, and public presentation digest. The
browser receives only the answer-free presentation. For a native HOTSPOT question, canonical source
names a server-owned registered workspace asset plus SHA-256; publication re-reads and rehashes it,
validates the bounded still image, then creates a fresh version-scoped typed asset. QTI source and
import evidence follow the same revalidation pattern.

The checksum is deliberately an integrity detector, not a substitute for authority. Any principal
that can change both bytes and their authoritative digest can forge a matching pair. That is why the
design also requires RLS, broker functions, KMS/IAM separation, immutable/append-only publication,
audit/reconciliation, and a dedicated publisher. This is stronger than treating a checksum as an
access-control mechanism.

### Encryption at rest and in use

The chosen long-term design is managed encryption at rest with least authority: encrypted RDS,
backups, four S3 SSE-KMS domains, distinct keys/contexts, Secrets Manager, encrypted state, and
selective application AEAD only for short-lived capability/state values that require it. It does
not encrypt every public question/image in application code and decrypt it on each delivery.

Blanket application encryption would obscure legitimate CDN/public content without improving the
dominant threat model, enlarge key-distribution/decryption attack surface, and make inspection,
publication, and recovery harder. Private/student/source content obtains meaningful protection from
KMS-backed storage plus authorization/IAM boundaries; public immutable assets are intentionally
public only after post-commit activation.

### Passwordless

Passwordless avoids storing reusable password verifiers. It does not eliminate
security-sensitive state: passkey public keys, one-time token/session hashes, recovery email
authority, mailbox delivery, revocation, rate controls, and browser/session binding remain
protected assets. The current design preserves those controls rather than treating "no passwords"
as sufficient authentication architecture.

## Research basis and future reference

The project-specific
[PortSwigger security review reference](../PORTSWIGGER_SECURITY_REVIEW_REFERENCE.md) is the
durable future-skill seed. It maps review questions and negative oracles to current Academy material
without copying exploit instructions, and distinguishes code, integration, and deployment evidence.
It informed the checks for
[access control and IDOR](https://portswigger.net/web-security/access-control),
[CSRF/SameSite](https://portswigger.net/web-security/csrf/preventing),
[SSRF](https://portswigger.net/web-security/ssrf),
[file uploads](https://portswigger.net/web-security/file-upload), and
[race conditions](https://portswigger.net/web-security/race-conditions).

Three local books informed the architecture-level reasoning:

- *The Tangled Web* (origin, cookie, parser, and active-content sections) informed exact browser
  authority, fragment-token hygiene, markup handling, and proxy/cache skepticism.
- *Threat Modeling* (data-flow diagram, trust-boundary, and STRIDE sections) informed the actor,
  asset, storage, and deployment boundary inventory rather than a route-only review.
- *Security Engineering*, third edition (authentication/revocation, least authority, recovery, and
  assurance sections) informed one-use bound ceremonies, role/KMS separation, idempotent recovery,
  telemetry, and the decision against blanket application encryption.

## Human guidance and HCI comparison

Security does not oppose [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md) in this audit. Its
pre-production clean-break instruction enabled removal of weak compatibility paths; its server-only
grading, immutable publication, tenant-owned records, encrypted recovery, keyboard access, and
behavior-focused evidence requirements reinforce the security model.

The review retained three deliberate, visible tradeoffs:

- A passwordless email link is browser-bound. The UI must explain why another device cannot silently
  complete it and preserve entered information for a safe retry.
- Anonymous authentication remains enumeration-safe while authenticated users receive actionable
  rate-limit/recovery feedback. The design avoids CAPTCHAs and punitive lockouts that would harm
  shared-campus and accessibility use cases.
- Authorization concealment returns generic not-found/unavailable results for protected objects and
  uncertain provider effects. Accessible recovery text and an instructor/support path must explain
  the next safe action without revealing whether another tenant's resource exists.

The last point is a guardrail: concealment that leaves a legitimate learner without a comprehensible
recovery path would conflict with human guidance. Current recovery contracts preserve that guidance
while keeping the attacker-facing response non-oracular.

## Re-audit verdict

The stale audit's repository design weaknesses have been remediated and independently re-audited.
Peptidyle now has stronger explicit enforcement at its browser, authorization, tenancy, storage,
publication, provider, and deployment-design boundaries than the stale baseline documented.

The correct final claim is therefore: **repository security architecture remediation is complete;
production activation remains blocked on named live evidence.** No code test, static Terraform
policy, local Compose file, or document statement substitutes for the remaining real edge, cloud,
database, provider, and recovery probes.
