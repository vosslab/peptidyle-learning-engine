# PortSwigger security review reference

This is a reusable, architecture-first companion to the
[SECURITY_MODEL.md](SECURITY_MODEL.md). It turns current
[PortSwigger Web Security Academy](https://portswigger.net/web-security) material into review
oracles for Peptidyle-like systems. It is not a penetration-test recipe and does not replace a
threat model, code review, or deployment validation.

The durable security contracts remain [SECURITY_MODEL.md](SECURITY_MODEL.md),
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#row-level-security), and [OBJECT_STORAGE.md](OBJECT_STORAGE.md). They are
aligned with the active audit; this reference never supersedes a contract.
A statement marked **Current** is a review hypothesis until its linked code test and, where
relevant, deployment evidence are recorded in the active audit.

## How to use it

For every boundary, start with the protected asset and the authority allowed to act on it. Then
choose a different Account, course, Student relationship, browser origin, request parser, credential, or timing window and
make the system prove denial. A green happy-path test is not evidence of an authorization boundary.

Each topic supplies five reusable prompts:

- **Preconditions** identify when the attack class can exist.
- **Review questions** find the root enforcement point, not just a handler check.
- **Evidence** names the sort of code and runtime proof to collect.
- **Negative oracle** is the durable test that must fail safely.
- **False confidence** describes a tempting but insufficient control.

Evidence has three non-interchangeable classes:

- **Code evidence:** deterministic local tests of an implementation invariant.
- **Integration evidence:** tests of real cooperating services, such as PostgreSQL, Caddy, or S3.
- **Deployment evidence:** proof for the actual edge, IAM principal, bucket/KMS policy, TLS, and
  network path. Only this class closes a deployment-dependent claim.

## Authorization and IDOR

PortSwigger distinguishes vertical, horizontal, and state-dependent access control; an opaque or
unguessable identifier does not authorize its holder. See
[access control and IDOR](https://portswigger.net/web-security/access-control).

- **Preconditions:** A request names a course, student, attempt, publication, object, or action;
  two courses or roles can exercise the same route.
- **Review questions:** Does every read and mutation derive the authenticated Account, exact course/Student or workspace relationship, and capability from a
  server credential? Is authorization repeated after an ID is resolved? Can an alternate HTTP
  method, trailing path, step, or signed URL bypass the normal route?
- **Evidence:** **Code and integration evidence required.** Trace `AuthenticatedSession` from the opaque
  session, PostgreSQL RLS and protected database-function enforcement, typed object-key grants, and re-resolution of
  submissions and immutable presentations. Record the exact tests in the active audit before
  calling this a current guarantee.
- **Negative oracle:** A Student from another course, an Instructor without the exact course role, and a stale
  session each receive no protected record, signed URL, state transition, or distinguishable leak.
- **False confidence:** UUIDs, hidden UI controls, client role fields, an Account or course ID in JSON, or a
  first-step-only authorization check are not authorization.
- **Applicability:** Always applicable. Test route, store, protected database-function, object-signing, and
  background-worker paths as one matrix; do not treat RLS as a substitute for route authorization.

## Passwordless authentication

Passwordless removes reusable password verifiers, but email links, passkeys, recovery channels,
sessions, and enrollment remain authentication and account-recovery authorities. The relevant
review lens is
[authentication vulnerabilities](https://portswigger.net/web-security/authentication).

- **Preconditions:** An email link, WebAuthn ceremony, invitation, recovery action, session cookie,
  or identity-provider callback can establish, attach, or replace an account identity.
- **Review questions:** Are tokens random, single-use, short-lived, hashed at rest, browser-bound
  when appropriate, and rate/cost bounded before persistence? Does completion bind the same
  account, origin, challenge, and relying-party identity? Does logout revoke every active authority?
- **Evidence:** **Code evidence:** token construction and hashing tests, WebAuthn origin/RP checks,
  session-revocation tests, and uniform email-start responses. **Deployment evidence:** configured
  secret-store use rather than source configuration.
- **Negative oracle:** Replaying a consumed or expired link, mixing two browser bindings, injecting
  a duplicate cookie, or using a revoked account/session cannot mint a new authenticated session.
- **False confidence:** `HttpOnly` alone, a random-looking token stored raw, passkey presence
  without user verification, or returning a different email-enumeration response is insufficient.
- **Applicability:** **Current code-evidence target** for the passwordless surface. OAuth and JWT
  checks below are deferred unless such providers or bearer-token APIs are enabled.

## Browser, CSRF, and XSS

SameSite is site-scoped, not origin-scoped. A vulnerable sibling subdomain or a state-changing GET
can defeat a design that treats `Lax` as complete CSRF protection. See
[SameSite bypasses](https://portswigger.net/web-security/csrf/bypassing-samesite-restrictions),
[CSRF prevention](https://portswigger.net/web-security/csrf/preventing), and
[XSS](https://portswigger.net/web-security/cross-site-scripting).

- **Preconditions:** Browsers send cookies, render author or provider content, embed an LMS/tool,
  accept a redirect, or execute first- or third-party JavaScript.
- **Review questions:** Does every cookie-authenticated mutation require an exact allowed `Origin`
  and, where embedding is enabled, a session-bound CSRF proof? Are all state changes non-GET? Are
  cookie scope, Host handling, CSP, frame ancestors, sanitization, and DOM sinks owned centrally?
- **Evidence:** Capture response headers for success and error responses, browser tests from a
  hostile origin and sibling site, sanitizer allowlist tests, and an inventory of `innerHTML`, URL,
  `postMessage`, iframe, and Wasm boundaries.
- **Negative oracle:** A cross-origin form, sibling-domain script, duplicate/host-injected cookie,
  malformed Origin, or untrusted message source cannot change state or read a protected response.
- **False confidence:** `SameSite=Lax`, a frontend-only CSRF token, CORS denial alone, escaping one
  template, or a CSP that does not cover the actual response are incomplete defenses.
- **Applicability:** **Code evidence target.** An answer-free browser payload can reduce grading
  disclosure, but it cannot reduce session, author-content, or iMathAS Question Backend browser risk. Treat
  that payload property as current only when the active audit links the response-contract tests.

## Host, cache, and parser boundaries

An attacker-controlled Host or forwarding header can poison generated links, reset flows, cache
keys, and authorization decisions. Multiple HTTP hops can disagree about request boundaries or
which response is cacheable. See
[Host-header attacks](https://portswigger.net/web-security/host-header/exploiting),
[request smuggling](https://portswigger.net/web-security/request-smuggling),
[cache poisoning](https://portswigger.net/web-security/web-cache-poisoning), and
[cache deception](https://portswigger.net/web-security/web-cache-deception).

- **Preconditions:** A CDN, Caddy/proxy, load balancer, application server, static handler, or
  shared cache processes the request; a header affects a URL, client address, route, cache key, or
  response.
- **Review questions:** Is the public authority configured rather than derived from Host? Which
  proxies may supply forwarding data, and is the peer authenticated before it is trusted? Do edge
  and app use an unambiguous HTTP protocol and limits? Can dynamic routes ever reach static cache
  fallback, including suffix, encoded path, or semicolon variations?
- **Evidence:** **Code evidence:** configuration tests for an exact browser authority and trusted
  proxy CIDRs, plus hostile Host/X-Forwarded-For probes and protected-response `no-store` header
  snapshots. **Integration evidence:** gateway configuration validation and malformed-framing
  probes. **Deployment evidence:** the released edge's protocol, cache, and forwarding behavior.
- **Negative oracle:** Wrong Host, untrusted forwarding header, CL/TE ambiguity, and
  `/api/private.css`-style path variants cannot alter a generated URL, rate-limit key, route, cache,
  or another user's response.
- **False confidence:** Overwriting a header at one local proxy, a cache-control header on only 200
  responses, or a separate edge ACL that the application does not understand.
- **Applicability:** **Deployment target** whenever a reverse proxy is deployed. Request-smuggling
  validation is an integration test against the actual edge and protocol version, not a Rust unit
  test.

## SSRF and outbound services

SSRF turns a server-side HTTP client into an attacker-controlled network principal. URL validation
alone is fragile because redirects, DNS rebinding, alternate encodings, and metadata addresses are
also part of the request path. See [SSRF](https://portswigger.net/web-security/ssrf).

- **Preconditions:** User, course author, import, provider response, or configuration can influence
  a destination URL, hostname, redirect, protocol, or proxy selection.
- **Review questions:** Is each outbound destination an immutable deployment allowlist or a typed,
  normalized capability? Are redirects disabled or revalidated hop by hop? Does network policy
  deny metadata, database, object-store, and control-plane addresses by default? Can an external
  tool cause the API to fetch a supplied URL?
- **Evidence:** Inventory every HTTP client, URL parser, redirect policy, DNS resolver, proxy
  variable, service-account credential, and container egress rule. Test provider and renderer URLs
  against loopback, link-local, RFC1918, IPv6, encoded, and redirect targets.
- **Negative oracle:** A question, import, callback, or provider response cannot fetch a metadata
  service, another workload, or an arbitrary internet host, nor disclose its body in an error.
- **False confidence:** Blocking only `127.0.0.1`, accepting a hostname suffix, or trusting an
  initial DNS lookup while following redirects.
- **Applicability:** **Code and deployment evidence target** for deployment-configured,
  process-fixed iMathAS/WebWork endpoints that no request can override. User-controlled URL fetches
  are not a supported feature and should remain absent until a dedicated SSRF design exists.

## Files, paths, and deserializers

File upload is an execution, parsing, storage, and delivery problem, not merely an extension check.
Path traversal and unsafe object deserialization often occur before an application realizes that it
has crossed its intended trust boundary. See
[file uploads](https://portswigger.net/web-security/file-upload),
[path traversal](https://portswigger.net/web-security/file-path-traversal), and
[insecure deserialization](https://portswigger.net/web-security/deserialization).

- **Preconditions:** The service accepts ZIP/QTI packages, images, canonical source, JSON, and object
  references.
- **Review questions:** Is input bounded before decode? Are archive names normalized and rejected on
  absolute paths, backslashes, traversal, duplicates, symlinks, and expansion bombs? Are media
  types sniffed and fully decoded? Can a wire payload choose a Rust trait object, filesystem path,
  Object Address, serializer type, or privileged enum variant?
- **Evidence:** **Code evidence target:** QTI and raster corpus tests must reject hostile archives
  and restrict raster decoding. Retain corpus tests for malformed, duplicate-key, oversized, and
  polyglot inputs.
- **Negative oracle:** A crafted archive/image/JSON payload cannot write outside staging, exhaust
  decompression resources, execute content, select another course's object, or create an untyped
  privileged value.
- **False confidence:** Filename/MIME checks, a SHA-256 alone, a random storage name, or Serde
  parsing without an explicit schema and resource bounds.
- **Applicability:** **Code evidence target** for author imports and assets.

## Concurrency and business logic

Race conditions are security issues when two individually valid requests make an invalid combined
state. Multi-step authorization must be checked at the state transition, not assumed from prior
screens. See [race conditions](https://portswigger.net/web-security/race-conditions) and
[access-control workflow flaws](https://portswigger.net/web-security/access-control).

- **Preconditions:** Login, invitation claim, publish, attempt issue/submit, quota, signed delivery,
  or retention operations use read-then-write logic or background workers.
- **Review questions:** What invariant must be atomic? Which row/version/idempotency key is locked?
  What happens if two requests race across a timeout, retry, worker, or second browser? Can an old
  presentation or revoked session complete a later step?
- **Evidence:** Database constraints, transaction isolation, `FOR UPDATE`/conditional updates,
  uniqueness constraints, idempotency records bound to exact requests, and parallel integration
  tests with a barrier rather than timing guesses.
- **Negative oracle:** Two simultaneous claims/submits/publishes yield one authorized state and
  one clear duplicate/conflict result; neither bypasses immutable published-content or grade rules.
- **False confidence:** A disabled button, client debounce, a preflight GET, or a unit test that
  runs requests sequentially.
- **Applicability:** **Integration evidence target.** Publication immutability and grading are
  especially high-value transition invariants; closure requires parallel database-backed tests.

## OAuth, JWT, and WebSockets

These topics are feature-gated, but an audit must name their activation contracts before a generic
library or convenience endpoint silently enables them. See
[OAuth 2.0](https://portswigger.net/web-security/oauth),
[JWT attacks](https://portswigger.net/web-security/jwt), and
[WebSocket security](https://portswigger.net/web-security/websockets).

- **Preconditions:** OAuth/OIDC callback, LMS launch, JWT bearer credential, signed webhook, or
  WebSocket upgrade is enabled.
- **Review questions:** Are redirect URI, issuer, audience, nonce, state, PKCE, key rotation, and
  algorithm verification exact? Is a JWT verified before claims are read and forbidden from choosing
  course/role? Does every WebSocket handshake check Origin and authenticate before subscription?
  Are messages schema-bounded and authorized per resource, not just on connect?
- **Evidence:** Provider fixture tests for wrong issuer/audience/key/algorithm/redirect/state;
  WebSocket tests for cross-origin upgrade and post-connect IDOR; a configuration gate that keeps
  the feature off without every required value.
- **Negative oracle:** A token from another issuer, changed `alg`, reused state, arbitrary redirect,
  or a WebSocket message for another course cannot establish identity or receive data.
- **False confidence:** Decoding a JWT, checking a UI role, accepting wildcard redirect URIs, or
  authenticating a socket once and trusting all later messages.
- **Applicability:** OAuth/JWT/WebSocket are deferred. **Code and deployment evidence target:**
  iMathAS and future LTI integration require equivalent issuer, audience, state, replay, and
  per-message/boundary scrutiny.

## APIs, databases, storage, and IAM

API testing needs the same object-level authorization and endpoint discovery discipline as browser
testing; undocumented endpoints and different content types are part of the surface. See
[API testing](https://portswigger.net/web-security/api-testing).

- **Preconditions:** Browser API, worker API, PostgreSQL role, S3 bucket, KMS key, or cloud workload
  identity can read, write, sign, decrypt, or call another service.
- **Review questions:** Is a request's authenticated Account context transaction-local and impossible for a browser
  to set? Does each PostgreSQL login have only the necessary non-inheriting memberships and verified
  TLS? Does each workload receive an independent IAM role, exact bucket prefix/actions, and KMS key?
  Is object authorization checked before a short-lived URL is issued and after object metadata is
  reconciled? Can a worker become an API or grading-reader deputy?
- **Evidence:** **Code evidence:** RLS-force and protected database-function tests, startup database
  identity/privilege checks, and decision logs without secrets or student answers.
  **Integration evidence:** TLS, S3 encryption/key/HTTPS, and cross-role service tests.
  **Deployment evidence:** IAM policy review and the deployed workload-to-bucket/KMS/network path.
- **Negative oracle:** Changing course settings, borrowing another role's URL, requesting another
  bucket/key prefix, or calling a protected database function without its required authority yields no data or action.
- **False confidence:** An ORM filter, one broad cloud role, client-side Object Addresses, a bucket-wide
  signed URL, encryption at rest without authorization, or a database superuser used by the API.
- **Applicability:** **Deployment target and release blocker.** Object storage and cloud IAM are
  separate authorization planes; neither database RLS nor an encrypted drive substitutes for them.

## Third parties and supply chain

External graders, renderers, IdPs, container images, build tooling, and package ecosystems are
confused-deputy and provenance boundaries. They should receive only the authority and data needed
for a single job.

- **Preconditions:** A worker invokes an external engine, pulls an image, accepts a provider result,
  installs a dependency, or passes user/student material to a service.
- **Review questions:** What authenticated Account and typed job target call the service? What exact course, Student, or workspace scope binds the response? What
  secrets, network routes, database roles, object prefixes, and egress does it receive? Is image
  provenance pinned and verified? Is every reply size-bounded, schema-validated, correlated, timed
  out, and safe to retry?
- **Evidence:** **Code evidence:** signed/correlated provider-result tests, timeouts, and failure
  modes that do not broaden access or disclose protected payloads. **Deployment evidence:** a
  network diagram, immutable image digest/provenance record, and per-workload IAM/database
  credentials.
- **Negative oracle:** **Required release oracle:** a production deployment must demonstrate that a
  compromised renderer/provider cannot query PostgreSQL, list object storage, invoke another
  course's job, forge a result for a different correlation, or use an API credential.
- **False confidence:** A private Docker network, an HTTPS URL, a successful image build, or a
  callback shared secret without sender identity and per-job correlation.
- **Applicability:** **Code and deployment evidence target** for renderer and iMathAS boundaries.
  Cloud deployment must prove the runtime IAM/network policy, not just Compose intent.

## Future skill seed

Suggested trigger phrases are: `security architecture audit`, `PortSwigger review`, `account
isolation review`, `CSRF/session review`, `external-service threat model`, and `pre-release security
gate`.

Suggested workflow:

1. Read repository rules, security contracts, deployment configuration, and the active plan.
2. Draw authority and data-flow boundaries before searching for individual bug classes.
3. Select only applicable topic cards above; state why inactive surfaces are deferred.
4. Trace one high-value action through browser, API, database, storage, worker, and external
   service.
5. Add a negative oracle at the strongest enforcement layer and an integration test at the boundary.
6. Implement root design corrections, then update contracts, audit status, and operational evidence.
7. Finish with an adversarial re-review and clearly separate verified guarantees from assumptions.

Progressive references for a future skill should split this document into: browser and HTTP;
authorization and state transitions; data, storage, and IAM; external services and supply chain;
and a short PortSwigger URL index. Keep the skill itself procedural and link to topic cards only
when the feature is present.

Non-goals: automated exploitation, indiscriminate scanning, dependency-version churn, generic
checklists detached from an asset, or treating a security header/package as proof that an authority
boundary is enforced.
