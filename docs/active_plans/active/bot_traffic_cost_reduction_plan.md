# Plan: Bot traffic cost containment

## Status

Planning state: decision-complete companion to WP-RC10 and WP-RC11. OpenTofu in
`deploy/opentofu/` owns production infrastructure, and standards-based institutional OIDC owns
production login. WP-BOT-1 through WP-BOT-10 retain their dependency order; no setup patch waits for
an infrastructure-language or identity-provider choice.

## Decisions

- Use OpenTofu with one declarative root at `deploy/opentofu/`; console changes are emergency-only
  and must be imported before incident closure.
- Use institutional OIDC Authorization Code with PKCE behind the existing `IdentityProvider`.
- Ship no client analytics. Aggregate CloudFront, WAF, ALB, application, queue, database, and cost
  metrics provide the version 1 evidence boundary.
- Keep `www` as a static private-S3/CloudFront origin and `app` as same-origin SPA plus API.
- Preserve legitimate shared-egress, VPN/datacenter, international, IPv4/IPv6, keyboard, and
  screen-reader use as enforcement gates.

## Objectives

- Make anonymous crawling consume static edge bytes or a cheap refusal, never learning-engine
  dependencies.
- Keep production edge, identity, cost, accessibility, recovery, and rollback behavior declarative
  and testable.
- Prove controls against both adversarial traffic and the synthetic class-start workload.

## Scope

WP-BOT-1 through WP-BOT-10, their OpenTofu owners, permanent policy tests, disposable deployment
rehearsals, cost evidence, runbooks, and independent reviews are in scope for version 1.

## Background

The local reference, [99% of My Website Traffic Is Bots](../../how-to-reduce-impact-of-bot-traffic.md),
describes an extreme but useful asymmetry: anonymous crawlers can consume nearly all origin work
while ordinary browser analytics record almost none of it. Its strongest transferable lessons are
to measure at the edge and origin, block or challenge before expensive work, distinguish verified
crawlers from self-declared user agents, rate-limit page requests rather than static assets, and
measure the human cost of every challenge.

PLE has a favorable content boundary. Course, learner, draft, grading, and authored-source data are
already authenticated and tenant-owned. Published problems are shared and immutable, but PLE does
not need a public, enumerable problem page for every version. A small public landing page can explain
the product while every course, catalog, authoring, run, grade, and administrative surface stays
behind authentication. Immutable public assets may remain CDN-cacheable because they contain no
answers or educational records; the authenticated application controls their discovery.

The owner's proposed split is directionally right:

```text
anonymous internet
        |
        v
www.example.org             app.example.org
tiny static landing         authenticated PLE application
no API calls                CDN -> edge controls -> ALB -> stateless API
no cookies                  PostgreSQL / objects / jobs only after authority
        |                              |
        +------ Sign in link ----------+
```

GitHub Pages is not the durable production host for this split if PLE becomes a paid online service.
GitHub's current [Pages limits](https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits)
say Pages is not intended or allowed as free hosting for an online business, e-commerce site, or
commercial SaaS, and has a 100 GB monthly soft bandwidth limit. GitHub also says Pages should not
handle passwords or credit-card data. Pages can be considered for a genuinely non-commercial,
pre-launch project showcase with no form, account, payment, cookie, or API integration. The durable
PLE production landing page will use a private S3 origin behind CloudFront. That is already the M6
asset-delivery architecture, separates anonymous bandwidth from API tasks, and keeps the generated
landing artifact portable. GitHub Pages is not a production fallback in this plan.

## Package objectives

- Make the cost of an anonymous landing-page crawl independent of API, database, object-store,
  renderer, provider, and worker capacity.
- Require a valid server session and tenant authority before any educational content, records, or
  expensive application behavior becomes available.
- Keep the public experience fast, accessible, useful, and honest without exposing a crawlable
  content catalog.
- Bound abuse costs at the edge and application layers without imposing routine CAPTCHA friction on
  legitimate students.
- Measure cache offload, origin work, authentication pressure, and spend so controls respond to
  evidence rather than guesses.

## Design philosophy

This plan applies **fix the design, not the symptom**: anonymous traffic receives a separate static
surface instead of making the dynamic application identify bots after origin work has already
happened. It also applies **design for adaptability**: the landing artifact, static host, edge
policy, application host, and server authorization remain separate replaceable components.

The goal is not to make automation impossible. A bot with valid stolen credentials is an
authenticated attacker and requires account/session controls. The goal is to ensure ordinary
anonymous crawling can obtain only cheap static bytes or a cheap refusal and cannot trigger
database reads, signed URLs, renderer calls, job creation, grading, or provider egress.

The login wall is therefore an authority and discovery boundary, not proof that a caller is human.
A valid session still receives least-privilege route checks, bounded request envelopes, per-session
and per-tenant mutation/concurrency limits, idempotency for job-producing operations, and revocation.
No edge verified-bot or challenge result grants application authority.

- Evidence strategy for uncertain methods: record a traffic and cost baseline, then replay the same
  generated crawler workload and a versioned one-time class-start/shared-egress scenario defined by
  WP-BOT-1 from the main plan's synthetic burst assumptions. Choose the workload size during the
  experiment so repeated runs produce a stable cost estimate; the normalized reporting unit does
  not dictate the number of requests sent. A signal becomes an enforcement rule only when its
  report-only results separate abusive traffic from every legitimate scenario. If they overlap, keep
  the signal for observability and use provider, session, and tenant authority instead. Later pilot
  evidence may revise the scenario and thresholds; the plan does not claim those observations exist
  today.

## Detailed capability scope

- Build one minimal public landing artifact with no API, authentication, or personalized content.
- Separate the public and authenticated hosts with explicit DNS, cookie, CSP, CORS, and redirect
  contracts.
- Keep every PLE product route and data-bearing API behind the existing server session boundary.
- Add cheap unauthenticated refusal before database, object, queue, renderer, or provider work.
- Put CDN caching, WAF/rate controls, origin shielding, spend limits, and observability in front of
  the authenticated application.
- Define permanent regression tests and separate one-time capacity/cost experiments.
- Add an operator runbook for normal controls, elevated challenge mode, and a time-bounded emergency
  mode.

## Non-goals

- Keep billing and subscription checkout outside this plan; payment belongs on a compliant hosted
  billing or authenticated application surface.
- Keep educational content out of the landing artifact; public screenshots and illustrations are
  marketing assets, not a sample problem catalog.
- Preserve the shared immutable published-content model; authentication controls discovery and use
  without making answer-free CDN assets secret records.
- Preserve PLE's international teaching audience by starting with risk and network evidence rather
  than country-wide blocks.
- Preserve legitimate accessibility and campus/VPN access by using risk-triggered challenges instead
  of CAPTCHA on every visit.
- Keep the current WP-QTI and M5 dependency order; production rollout remains an M6 responsibility.

## Current state summary

- The Solid client is one static bundle and requests the current session during startup.
- The local Caddy gateway proxies all requests to the API replicas; it is deliberately not an
  authentication, storage, or asset boundary.
- The server owns replica-safe sessions in PostgreSQL and uses an `HttpOnly` cookie. Authenticated
  route handlers establish tenant authority before returning records.
- `/health` performs semantic dependency verification. It is valuable to an orchestrator but would
  be needlessly expensive as an unrestricted public bot target.
- The active plan already selects CloudFront for immutable public assets and S3 for binary/static
  artifacts, so a static landing origin fits the M6 architecture without adding a new runtime.
- The maintained local stack has no production WAF, CDN, runtime-role attestation, or cloud cost
  alarms yet; those remain deployment work.

## Architecture boundaries and ownership

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component                           | Review boundary                                                   |
| ---------------------- | ----------------------------------- | ----------------------------------------------------------------- |
| M1 / WS-MEASURE        | Edge and origin telemetry           | Anonymous request and cost classification                         |
| M1 / WS-INVENTORY      | Route and dependency inventory      | Public, authentication, and session-required route classes        |
| M2 / WS-LANDING        | Dedicated landing build             | Static HTML/CSS/assets with zero PLE API traffic                  |
| M2 / WS-AUTH           | Browser and server session boundary | Cookie scope, same-origin app API, and cheap refusal              |
| M2 / WS-HOSTS          | DNS, CDN, cache, and origin routing | Static behaviors, app API behavior, origin shield, private health |
| M3 / WS-EDGE           | CDN, WAF, and rate policy           | Cache, verified crawler, risk challenge, and emergency rules      |
| M3 / WS-BUDGET         | Spend and scaling guardrails        | Metrics, alarms, task ceilings, and operator actions              |
| M4 / WS-TEST           | Permanent cross-boundary gates      | Cache/auth/origin/adversarial regression suite                    |
| M4 / WS-REHEARSE       | Deployment acceptance and closure   | Synthetic crawler, legitimate login, cost report, and runbook     |

Durable ownership rules:

- The landing build owns only public project information and public marketing assets. It receives no
  generated API client, WASM grader-adjacent bundle, session code, or environment secret.
- The authenticated Solid application remains the only browser product surface. It calls the API on
  its own origin so production does not need broad CORS or cross-site session cookies.
- The Rust server remains the authority boundary. Edge rules reduce load but never grant a session,
  tenant, role, asset, or grade.
- The CDN owns anonymous byte delivery. PostgreSQL, object records, job queues, renderers, and
  external providers must see no request caused solely by loading the public landing page.
- Orchestrator health checks use a non-public origin path or trusted network. A public status page is
  static and coarse; it does not proxy `/health` per visitor.
- Each work package has one doer and one independent reviewer. The doer owns every production edit
  for that package until its gate passes; the reviewer is read-only. A failing cross-boundary test
  returns to the package that owns the violated behavior instead of being patched in WP-BOT-9 or
  WP-BOT-10. WP-BOT-10 integrates and records evidence but does not introduce new production policy.

### Deployment artifact prerequisite

WP-RC10 creates one OpenTofu root at `deploy/opentofu/`. Its production files are
`versions.tf`, `providers.tf`, `variables.tf`, `locals.tf`, `network.tf`, `database.tf`,
`storage.tf`, `compute.tf`, `edge.tf`, `waf.tf`, `observability.tf`, and `outputs.tf`, with
`env.example.tfvars` and `tests/policy.tftest.hcl`. The disposable proof must show that generated
credentials stay out of source, plan output, logs, and ordinary state inspection; remote state is
encrypted and restricted to the deployment identity; rotation can complete and roll back; drift is
detectable; offline policy tests need no live credentials; and destroy removes only resources tagged
with its exact deployment ID.

The accepted deployment root is the only source of truth for DNS, certificates, distributions,
origins, WAF, alarms, budgets, and runtime ceilings. Console edits are emergency-only, require an
incident record, and must be imported or reproduced declaratively before the incident closes.

## Resolved decisions

In this plan, "login wall" means authentication and institutional/course authority, not a payment
checkout. Billing remains outside scope and cannot weaken the same server authorization boundary.

### Host split

- `www.<domain>` is the canonical public host. The apex sends one permanent redirect to `www` and
  serves no independent content.
- `www.<domain>` uses CloudFront with a private S3 origin and origin access control. It has no route
  to the ALB.
- `app.<domain>` uses a separate CloudFront distribution. `GET`/`HEAD` requests for the SPA shell and
  hashed browser assets use a private S3 origin; `/api/*` uses the ALB origin with caching disabled.
- API routes stay same-origin under `app.<domain>/api/...`; `www` receives no CORS permission.
- The session cookie is host-only for `app.<domain>`; it does not use `Domain=.<domain>` and is never
  sent to the landing host.
- The landing host does not call the app API, accept credentials, embed authenticated frames, or
  receive authentication callbacks.
- The Solid/WASM application code is public cacheable software, not protected educational content.
  It must remain answer-free and secret-free. All catalog data, course data, source, grading,
  tenant records, signed asset delivery, and mutations remain server-authorized.
- DNS cutover temporarily uses a TTL selected from the provider's observed propagation and the
  rehearsed rollback window. After the deployment remains stable for that window, restore the normal
  deployment TTL. These are recorded rollout values, not permanent unit-test constants. Rollback
  restores the previous distribution aliases and manifests; it never repoints `www` at the
  application origin.

### Cache and route behavior

The initial cache policy is explicit and versioned:

| Surface             | Methods       | Cache key              | Response policy                                                     | Origin             |
| ------------------- | ------------- | ---------------------- | ------------------------------------------------------------------- | ------------------ |
| `www` HTML          | `GET`, `HEAD` | normalized path only   | public; short revalidation window compatible with manifest rollback | private landing S3 |
| `www` hashed assets | `GET`, `HEAD` | exact hashed path only | public, long-lived, immutable                                       | private landing S3 |
| `app` HTML          | `GET`, `HEAD` | normalized path only   | public; short revalidation window compatible with app rollback      | private app S3     |
| `app` hashed assets | `GET`, `HEAD` | exact hashed path only | public, long-lived, immutable                                       | private app S3     |
| `app /api/*`        | route-owned   | no CDN cache           | `private, no-store` for authenticated and refusal responses         | ALB                |

WP-BOT-5 records the HTML and asset TTLs after measuring rollback propagation and cache behavior in
the disposable deployment. Permanent tests assert the semantic distinction -- revalidated HTML,
immutable content-addressed assets, and uncached APIs -- while accepting injected TTL values.

Static cache keys exclude cookies, authorization headers, and unrecognized query parameters. API
requests forward only the headers, host, methods, query values, and host-only session cookie required
by the typed route contract; they never share a cached response. `Set-Cookie` is valid only on the
non-cacheable app API behavior.

Client-route fallback may return app `index.html` only for `GET`/`HEAD` paths in the closed Solid
route contract. It never rewrites `/api/*`, `/health`, `/.well-known/*`, unsupported methods, or an
unknown file extension to the SPA. Those requests receive their real bounded 404/405 response.

### Static landing shape

Use a deliberately boring artifact generated from `landing/` into `dist/landing/`: semantic HTML,
CSS, locally stored optimized illustration assets, a privacy/contact link, and one `Sign in` link to
the fixed `app` origin. It uses no client framework, WASM, third-party script, font service, runtime
JSON, form, service worker, or credential. Record the compressed initial transfer during the
implementation review and compare it with the authenticated app entry path; do not turn that
measurement into a permanent byte-count test. An optional large marketing asset loads only after a
user requests it. The build and deployment are independent from the authenticated app.

File placement follows the existing repository roles: tracked landing source stays together under
`landing/` because it is a separate deployable surface, its maintained build logic belongs in
`pipeline/build_landing.mjs`, and generated output belongs under ignored `dist/landing/`. The existing
root `build.sh` remains the single front door and adds landing generation after its client stage,
because the current client builder recreates `dist/`. Browser coverage uses the existing
`run_playwright_tests.sh` and belongs in `tests/playwright/`; cloud rehearsal scripts do not belong in
the regular test runner.

Every landing and app release writes a content-addressed manifest. Deployment changes only the
selected manifest; it never overwrites a hashed object. Retain the active manifest and the last
known-good rollback manifest. Cleanup may remove an older object only after the deployment tool
proves no retained manifest or distribution behavior refers to it; retention beyond the rollback
pair follows the measured deployment cadence and storage budget rather than a hardcoded test count.

### Authenticated content wall

Every catalog query, problem detail, course, assignment, run, authoring, grading, export, import,
administrative, and protected-asset operation requires a valid PLE session. `robots.txt` and
`X-Robots-Tag: noindex, nofollow` express indexing policy for the app, but authorization remains the
only content boundary because hostile crawlers ignore robots rules.

An absent or syntactically malformed session cookie rejects before Store access. A syntactically
valid but unknown opaque token requires exactly one indexed session-hash lookup, then returns the same
generic `401` as every other invalid session. It receives no shared negative cache because that would
create cross-request identity state; the edge rate limit bounds repeated random-token probes. A valid
session establishes tenant context before route authority.

Route authority is checked before request body parsing beyond the route's small fixed envelope,
object fetch, signed-URL issuance, queue insertion, renderer/provider calls, or other work whose cost
scales with attacker input. `GET /api/auth/session` follows the same absent/malformed/unknown rules.

Production uses institutional OIDC Authorization Code with PKCE through `IdentityProvider`. The
local-development provider is never a production fallback. Provider discovery, issuer allowlisting,
state, nonce, PKCE, callback binding, credential recovery, abuse controls, anti-replay, and login
CSRF are owned by WP-RC8. PLE accepts only its bounded presentation,
performs at most one provider verification, and inserts a session only after successful verification;
failure creates no session row and returns one provider-independent generic refusal. This plan does
not add a database counter keyed by attacker-supplied account text.

The edge login/callback allowance is calibrated from WP-BOT-1's versioned class-start/shared-egress
scenario. Its initial retry and refresh assumptions are explicit rather than described as observed;
later pilot evidence may replace them. A compromised valid session remains subject to per-session
mutation limits, provider revocation, and normal tenant/role authorization.

Each job-producing or externally billed route declares its maximum accepted body bytes, required
role, idempotency key scope, per-session concurrency, per-tenant concurrency, queue ceiling, and
fixed refusal class in the route-cost inventory. Retries with the same idempotency key cannot create
a second job. Reaching one tenant's limit cannot consume another tenant's reserved class-start
capacity. This plan adds no generic "authenticated means unlimited" fallback.

An application-owned rate refusal is one fixed `429` response with `Retry-After`, `Cache-Control:
private, no-store`, and no account, tenant, provider, or rule detail. Edge challenge responses are
also non-cacheable. Rate values are deployment configuration established from report-only traffic
and repeated runs of the versioned class-start scenario; permanent tests exercise the limiter with
injected values and do not freeze a tunable production number.

### Origin and health boundary

- Both S3 origins are private and readable only through their owning CloudFront origin access
  control.
- The ALB accepts internet traffic only from the AWS-managed CloudFront origin-facing network list
  and a second origin-authentication condition proven by WP-RC10. The condition is the
  rotated header below; failure of either condition is a refusal before an API task.
- The second condition is a rotated CloudFront-added header whose value lives in Secrets Manager and
  is materialized only by the deployment identity. WP-RC10 proves that it stays out of source, plan
  output, logs, metrics, and application configuration and is visible only to the restricted
  encrypted-state and deployment identities required to configure it. Failure blocks WP-RC10; the
  implementation may not weaken either condition. The ALB listener
  owns verification; application handlers never receive or interpret it. Rotation may accept old and
  new values concurrently only until the new distribution is verified. The deployment records an
  expiry derived from measured CloudFront propagation and the rehearsed rollback window; reaching it
  triggers rollback instead of extending the overlap indefinitely. Permanent tests verify required
  expiry and rollback states with an injected clock, not a provider timing constant.
- ECS target-group health checks call `/health` inside the deployment network. CloudFront has no
  `/health` behavior and the public internet cannot invoke semantic database/object verification.
- This plan adds no public status endpoint. A later status page must be a separate static coarse
  artifact and can never proxy internal readiness.
- AWS may expose provider-assigned S3 or ALB origin names, but no PLE public DNS record advertises
  them and their access policies refuse direct clients. Renderer, worker controls, database, and
  object API have no public DNS or public network path.

### Edge policy order

Use the local bot article as evidence, not a rule set to copy blindly. PLE's initial order is:

1. Block explicitly unwanted declared crawlers with poor measured value; publish matching
   `robots.txt` rules for polite crawlers.
2. Exempt approved, provider-verified search crawlers from later challenges only after the explicit
   blocks.
3. Serve the public host entirely from cache/static origin.
4. Apply the shared-egress-calibrated limit to credential endpoints, a separate missing/invalid-session
   limit to APIs, and per-session/application limits after authentication. Static assets are not
   counted in page or credential buckets.
5. Challenge high-risk datacenter/automation traffic when it reaches the app host. Maintain an
   accessible recovery/support path.
6. Permit authenticated traffic subject to identity-provider, per-session, and tenant-aware
   application limits; IP address is only one signal.
7. Keep an emergency challenge rule disabled by default. Enable it only when an anonymous surge and
   either API 5xx, API latency, task saturation, or database-connection alarms remain active for the
   configured confirmation window. Choose that window from measured alarm delay and normal
   class-start variation. Recovery likewise uses a configured clear window supported by the
   rehearsal, and every activation has a finite expiry. A named operator may renew it only with a
   recorded reason. These values are deployment tuning recorded with the evidence, not source or
   permanent-test constants.

Every edge rule has a stable ID, owner, exact host/path/method matcher, action, exemption, metric,
activation state, and removal condition. User-agent text alone never grants a verified-bot exemption;
the edge provider's verified-crawler signal must establish it. New block/challenge rules run in
count/report mode against the generated crawler workload and recorded legitimate-use scenarios
before enforcement.

Rate thresholds are derived, not hand-waved. Repeated report-only runs of the versioned class-start,
retry, and crawler scenarios establish the assumed legitimate peak, measured variation, and whether
abusive traffic actually separates from it. The operations owner records the selected threshold and
safety margin with that evidence. Enforce only when every legitimate run remains below the proposed
threshold and abusive traffic crosses it; otherwise the rate signal remains a diagnostic metric and
authority/expensive-operation refusal carries the boundary. Permanent tests exercise rule ordering
and behavior with injected thresholds instead of asserting a production constant.

Country blocking is an evidence-triggered last resort, not a launch default. University students may
travel, use VPNs, study from military networks, or participate internationally.

### Cost evidence contract

Use metrics as the primary evidence; do not create an unbounded request-log warehouse while trying
to save money. CloudFront, WAF, ALB, API, database, object, queue, renderer, and provider counters use
the same five bounded dimensions: environment, host, route class, action/status class, and cache/auth
class. They contain no URL identifier, query value, cookie, account text, educational content, or
response body.

The dashboard computes these named quantities:

- `landing_ple_origin_calls`: PLE API/dependency calls caused by a `www` page load; required value 0.
- `anonymous_app_origin_ratio`: anonymous app-origin requests divided by anonymous app requests at
  CloudFront.
- `static_cache_hit_ratio`: CloudFront hits divided by cacheable `www` and app-static requests,
  separated by cold and warm runs.
- `invalid_session_store_lookups`: session Store lookups for unknown well-formed tokens; required at
  most one per request and 0 for absent/malformed cookies.
- `expensive_anonymous_operations`: object signs/reads, queue inserts, renderer/provider calls, and
  grading calls caused by requests without valid authority; required value 0.
- `bot_cost_per_10k`: edge request/WAF/static-origin/API/task/database/object/logging cost normalized
  to 10,000 requests from the generated crawler workload using the provider bill or current price
  model.
- `legitimate_auth_failure_rate`: unexpected failed login/session outcomes divided by the attempted
  legitimate operations in the one-time class-start or pilot run. Deliberately wrong credentials
  are classified separately. Report the observed rate and the affected path rather than freezing a
  synthetic percentage in a permanent test.

For the generated crawler workload, `bot_cost_per_10k = 10,000 * (measured_window_cost - idle_window_cost) /
measured_request_count`. Both windows use the same disposable deployment, duration, region, task
ceilings, and price-model version. The cost numerator includes CloudFront requests/egress, WAF,
landing-origin requests, ALB/API tasks, database/object/queue/renderer/provider work, logs, and
defense-specific fixed hourly allocation. A missing material category blocks the report. A negative
baseline-adjusted delta is retained with both raw windows and uncertainty and reported as
indistinguishable from zero; repeat the measurement only when the policy decision depends on
resolving that noise. The report records both provider-billed values available after settlement and
the same-day price-model estimate.

Full raw WAF/access logging is disabled by default. WAF aggregate metrics remain on. WP-BOT-1 selects
a finite sample cap and expiry from the smallest provider-supported values that still answer the
documented incident question and fit the logging budget. Configuration validation refuses an
unbounded sample or absent lifecycle, but permanent tests do not assert today's tuning values. The
exported sample may retain the edge provider's request ID, minute-bucket timestamp, action, verified-
bot/risk classification, country, and ASN; it excludes IP address, URI query, body, cookies,
authorization, response content, and student/account identifiers. The WAF may still evaluate source
IP transiently for its rule. Daily aggregate counts may remain for cost trends because they contain
no personal or educational record.

### GitHub Pages ruling

GitHub Pages is not selected for PLE production. It is acceptable only as a separately named,
temporary, non-sensitive open-source project showcase when its actual use fits GitHub's terms. It
uses a `github.io` or dedicated preview host, enforced HTTPS, no forms or credentials, and no
dependency from PLE availability to Pages. It never receives the canonical `www` DNS name. GitHub
recommends domain verification to prevent takeover and warns against wildcard records in its
[custom-domain guidance](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/about-custom-domains-and-github-pages).

PLE production uses the resolved S3/CloudFront design. The landing artifact remains portable so a
later provider change is a DNS/deployment change, not a UI rewrite.

## Milestone plan

| M   | Title                                   | Summary                                                               | Goal                                                           |
| --- | --------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------- |
| M1  | Measure and classify                    | Inventory anonymous routes and establish edge-to-origin cost evidence | Know which requests create cost before choosing controls       |
| M2  | Split public and authenticated surfaces | Create the static landing and explicit `www`/`app` authority boundary | Make ordinary anonymous crawling origin-free                   |
| M3  | Bound abuse cost                        | Add edge policy, origin shielding, scaling ceilings, and cost alarms  | Keep a traffic flood from becoming an unbounded bill or outage |
| M4  | Prove and rehearse                      | Exercise bots and real-user paths, then freeze the runbook            | Demonstrate low bot cost without degrading student access      |

### Milestone: M1 measure and classify

- Depends on: none.
- Deliverables: request taxonomy, route/dependency inventory, baseline dashboard, cost model, and
  privacy-bounded log fields.
- Workstreams: WS-MEASURE and WS-INVENTORY.
- Entry criteria: M6 deployment design begins, or the owner explicitly authorizes an earlier
  measurement-only slice.
- Exit criteria:
  - Edge requests, origin requests, cache status, status class, auth outcome, database calls, object
    delivery, queue insertions, renderer/provider calls, and `bot_cost_per_10k` are produced from the
    closed dimensions without logging response text or educational records.
  - Every public and authenticated path has one declared route class.
- Parallel-plan ready: yes -- max parallel doers: 2. Telemetry and route inventory are independent.

### Milestone: M2 split public and authenticated surfaces

- Depends on: WP-BOT-1 and WP-BOT-2 -- the split uses the measured taxonomy.
- Deliverables: portable landing artifact, exact CloudFront cache behaviors, host/DNS contract,
  authenticated app routing, scoped cookies, private origins/health, static security headers, and
  cheap anonymous refusal.
- Workstreams: WS-LANDING, WS-AUTH, and WS-HOSTS.
- Entry criteria: M1 exit criteria met; WP-RC10 owns OpenTofu at `deploy/opentofu/`; WP-RC8 owns
  production institutional OIDC before provider-specific auth deployment begins.
- Exit criteria:
  - Loading and crawling `www` causes zero PLE API, database, object, queue, renderer, and provider
    operations.
  - Anonymous `app` requests reveal no educational content and missing-cookie API requests perform
    no Store operation.
  - Cache-poison, SPA-fallback, direct-origin, and internal-health matrices pass.
  - Legitimate sign-in, refresh, logout, and replica failover remain correct.
- Parallel-plan ready: yes -- max parallel doers: 2. Landing and server refusal are independent;
  WS-HOSTS starts after both artifacts freeze and remains one serial owner through CDN and origin
  shielding.

### Milestone: M3 bound abuse cost

- Depends on: WP-BOT-6 -- edge rules and spend ceilings require the final host, cache, origin, and
  health boundary.
- Deliverables: CDN policy, WAF/rate rules, origin shielding, scaling ceilings, budget alarms, and
  normal/elevated/emergency operator modes.
- Workstreams: WS-EDGE and WS-BUDGET.
- Entry criteria: M2 exit criteria met.
- Exit criteria:
  - A synthetic anonymous burst is absorbed at the static/edge layers and does not scale workers,
    renderers, database connections, or object signing.
  - The versioned one-time class-start/shared-egress scenario stays inside the main plan's
    no-failed-submission gate.
  - Every actionable risk has an observable signal, owner, evidence-derived trigger, operator
    action, and recovery condition; diagnostic metrics need no invented alert.
- Parallel-plan ready: no -- WP-BOT-8 binds alarms and ceilings to WP-BOT-7's final stable rule IDs
  and deployed action metrics; running them concurrently would require mid-flight coordination.

### Milestone: M4 prove and rehearse

- Depends on: WP-BOT-7 and WP-BOT-8 -- validation exercises the complete deployed boundary.
- Deliverables: permanent contract tests, one-time load/cost report, accessibility review, and
  operator rehearsal record.
- Workstreams: WS-TEST and WS-REHEARSE.
- Entry criteria: M3 exit criteria met.
- Exit criteria:
  - Permanent gates pass in the maintained deployment suite.
  - One-time cost comparison records requests and estimated spend before and after the split.
  - The emergency rule is enabled, observed, and disabled in a disposable environment.
  - `docs/CHANGELOG.md` and the M6 tracker record the result.
- Parallel-plan ready: no -- WP-BOT-9 freezes permanent cross-boundary tests first; WP-BOT-10 then
  runs the live rehearsal and closure review against that exact gate.

## Workstream breakdown

### Workstream: WS-MEASURE edge and origin evidence

- Goal: attribute anonymous traffic to cache, origin, and expensive dependencies without collecting
  educational content.
- Owner: expert coder.
- Work packages: WP-BOT-1.
- Needs: current deployment request paths.
- Provides: baseline and thresholds used by edge and budget work.
- Review boundary, when modifying the repository: telemetry schema and redaction contract.

### Workstream: WS-INVENTORY route classification

- Goal: classify every browser/server path as public static, authentication-only, session-required,
  protected asset, internal health, or internal worker/provider.
- Owner: architect.
- Work packages: WP-BOT-2.
- Needs: generated route contract, Rust composition router, and deployment routes.
- Provides: one closed route inventory.
- Review boundary, when modifying the repository: route authority and dependency ordering.

### Workstream: WS-LANDING public artifact

- Goal: create the smallest useful accessible landing page with zero runtime dependency on PLE.
- Owner: UI/UX engineer.
- Work packages: WP-BOT-3.
- Needs: route inventory and approved public copy/assets.
- Provides: portable static build artifact.
- Review boundary, when modifying the repository: landing source, build, CSP, and visual/accessibility
  behavior.

### Workstream: WS-AUTH cheap refusal

- Goal: make anonymous and invalid-session requests terminate before expensive dependencies.
- Owner: expert coder.
- Work packages: WP-BOT-4.
- Needs: route inventory and session contract.
- Provides: server behavior and cost-proof tests.
- Review boundary, when modifying the repository: authentication middleware and route composition.

### Workstream: WS-HOSTS deployment split

- Goal: route `www` only to static hosting and `app` through the protected application edge.
- Owner: expert coder.
- Work packages: WP-BOT-5 and WP-BOT-6.
- Needs: landing artifact and auth contract.
- Provides: DNS/CDN/origin/cookie boundary.
- Review boundary, when modifying the repository: deployment configuration and origin access.

### Workstream: WS-EDGE traffic controls

- Goal: absorb, block, challenge, or rate-limit abusive traffic before origin work.
- Owner: expert coder.
- Work packages: WP-BOT-7.
- Needs: route classes, host split, and measured baseline.
- Provides: versioned edge policy and operator modes.
- Review boundary, when modifying the repository: WAF/CDN rules and legitimate-user exceptions.

### Workstream: WS-BUDGET cost guardrails

- Goal: prevent autoscaling and egress from turning abuse into an open-ended bill.
- Owner: expert coder.
- Work packages: WP-BOT-8.
- Needs: measured baseline and WP-BOT-7's final stable rule IDs/action metrics.
- Provides: dashboards, alarms, ceilings, and actions.
- Review boundary, when modifying the repository: metrics, redaction, budgets, and scale policy.

### Workstream: WS-TEST permanent adversarial gates

- Goal: make cache, auth, host, and cost regressions executable before deployment rehearsal.
- Owner: tester.
- Work packages: WP-BOT-9.
- Needs: every prior package.
- Provides: maintained permanent gates with small inline exploit-shaped scenarios.
- Review boundary, when modifying the repository: cross-boundary test ownership without production
  behavior changes.

### Workstream: WS-REHEARSE deployment acceptance

- Goal: prove the deployed cost and legitimate-user outcomes and close the plan honestly.
- Owner: integrator.
- Work packages: WP-BOT-10.
- Needs: WP-BOT-9 and every deployed component.
- Provides: one-time cost report, operator rehearsal, independent review, and closure evidence.
- Review boundary, when modifying the repository: deployed end-to-end behavior and documentation.

## Work packages

### Work package: WP-BOT-1 establish a cost baseline

- Owner: expert coder.
- Touch points: edge/access log configuration, CloudWatch metrics/dashboards, cost tags, and M6 tracker.
- Depends on: none.
- Acceptance criteria:
  - Implement every named quantity in the cost evidence contract from bounded counters, including
    the fixed `bot_cost_per_10k` calculation.
  - Report public/app request volume, cache status, origin count, auth outcome, and expensive
    dependency counts by the closed dimensions only.
  - Require an explicit finite event-sample cap and lifecycle selected from incident usefulness,
    provider capability, privacy, and logging cost; refuse unbounded configuration.
  - Retain no request body, response body, answer, grade, student name, or raw session credential.
  - Define and version the one-time class-start/shared-egress scenario from explicit concurrency,
    retry, refresh, and provider assumptions. Do not label those inputs observed until pilot evidence
    exists; later evidence may revise the scenario without changing permanent unit tests.
- Evidence or review: permanent pure redaction and bounded-configuration behavior tests with inline
  inputs; one-time generated crawler and versioned class-start/shared-egress baseline sized until the
  estimate is stable across repeated runs.
- Next dependency: freeze the baseline artifact and pass its metric names to WP-BOT-7 and
  WP-BOT-8.

### Work package: WP-BOT-2 freeze the route-cost inventory

- Owner: architect.
- Touch points: `src/route_contract.ts`, `crates/server/src/composition.rs`, asset/auth/health routes,
  and deployment routing.
- Depends on: none.
- Acceptance criteria:
  - Every route has exactly one of these classes: public static, app static, authentication-only,
    session-required, protected asset, internal health, or internal worker/provider.
  - Every class declares allowed methods, cache behavior, first authority check, first permitted
    dependency, public host, and cost class. Every job-producing or externally billed route also
    declares body, role, idempotency, session/tenant concurrency, queue, and refusal limits.
  - Internal health, worker, renderer, provider, and object origins are not public routes.
- Evidence or review: permanent source/route-contract assertion that fails when a new route is
  unclassified or two classes claim it.
- Next dependency: WP-BOT-3 through WP-BOT-5 consume this inventory.

### Work package: WP-BOT-3 build the static landing

- Owner: UI/UX engineer.
- Touch points: `landing/`, `pipeline/build_landing.mjs`, the final stage of `build.sh`, static
  headers, public assets, and generated `dist/landing/` deployment artifact.
- Depends on: WP-BOT-2.
- Acceptance criteria:
  - A browser network capture contains only `www` HTML and local hashed assets before the user
    activates `Sign in`; no fetch, XHR, WebSocket, cookie, redirect beacon, or PLE API request occurs.
  - Keyboard navigation, meaningful landmarks, focus visibility, reduced-motion behavior, readable
    text contrast, and responsive reflow support the landing's actual content.
  - The landing build graph contains no generated API client, Solid runtime, or WASM dependency; the
    built page starts no service worker, third-party request, form submission, or PLE API request.
- Evidence or review: permanent local Playwright checks for the visible sign-in journey, keyboard
  access, and observed network behavior; one-time artifact review, responsive contact sheet,
  measured contrast, and compressed-transfer comparison with the authenticated app.
- Next dependency: hand the exact `dist/landing/` artifact and response-header manifest to
  WP-BOT-5.

### Work package: WP-BOT-4 reject anonymous work cheaply

- Owner: expert coder.
- Touch points: authentication middleware, route composition, `SessionStore` conformance, and mounted
  server tests.
- Depends on: WP-BOT-2.
- Acceptance criteria:
  - Missing and syntactically malformed cookies return the same generic refusal with zero Store,
    object, queue, renderer, provider, or grading calls.
  - One syntactically valid unknown token performs exactly one indexed session-hash lookup and no
    other dependency call, then returns the same status/body class.
  - One rejected provider presentation performs at most one provider verification, creates no
    session, and returns the same status/body class for every provider rejection.
  - Production configuration refuses to start with the local development identity provider.
  - WP-BOT-1's versioned class-start scenario, including its explicit login retry and refresh
    assumptions, completes behind one shared egress identity with the proposed limits injected.
  - Authority precedes body parsing and every expensive dependency on protected routes.
  - A valid low-privilege session cannot invoke an instructor/admin/job-producing route. An
    authorized retry creates one job, and an authorized burst stops at the declared session/tenant
    concurrency and queue ceilings without affecting another tenant's class-start reserve.
- Evidence or review: permanent offline mounted tests with inline inputs and counting Store/provider
  fakes for refusal, provider rejection parity, and production-provider fail-closed composition;
  include OIDC state/nonce/PKCE/replay/CSRF cases from WP-RC8. Reuse the established replica E2E
  instead of adding a second fixture.
- Next dependency: expose the route classes needed by WP-BOT-5 without exposing secrets.

### Work package: WP-BOT-5 configure hosts and cache behaviors

- Owner: expert coder.
- Touch points: DNS, CloudFront distributions/behaviors, private S3 origins, deployment manifests,
  TLS, cookie configuration, CSP/CORS, and infrastructure documentation.
- Depends on: WP-BOT-3 and WP-BOT-4.
- Acceptance criteria:
  - The M6 tracker names one declarative deployment tool and repository root; a checked plan shows
    all resources in this package and contains no secret. Drift detection is proven once against a
    disposable stack rather than through a networked regular test.
  - Apex, `www`, and `app` resolve to their one declared behavior; wildcard DNS is absent.
  - `www` can reach only the landing S3 origin. App static paths can reach only the app S3 origin;
    `/api/*` can reach only the ALB origin.
  - The cache table's behavior is encoded: methods, path classes, cache keys, cookie/query
    forwarding, revalidated HTML, immutable hashed assets, and `no-store` API responses. TTL values
    remain deployment tuning informed by the rollback rehearsal.
  - Random cookies, authorization headers, query strings, `Host`, and `Accept-Encoding` variants
    cannot poison or fragment static cache keys beyond the declared compression variants.
  - Unknown `/api/*`, `/health`, `/.well-known/*`, file-extension, and non-GET/HEAD requests never
    receive SPA `index.html`.
  - Landing requests never carry the PLE session cookie; app/API stays same-origin with no landing-
    to-API CORS allowance.
  - A new app deployment can roll back by selecting the previous immutable app/landing manifests;
    no mutable asset is overwritten.
  - The selected DNS cutover window, active/known-good manifest retention, reference-safe cleanup,
    and rollback action are recorded and exercised without freezing their tuning values in unit
    tests.
- Evidence or review: permanent offline cache-policy behavior and SPA-fallback tests with injected
  TTLs; one-time disposable DNS/TLS/cache-poisoning/cookie browser probe.
- Next dependency: WP-BOT-6 consumes the distribution IDs, origins, and exact path matchers.

### Work package: WP-BOT-6 shield origins and private health

- Owner: expert coder.
- Touch points: ALB listener/security groups, CloudFront origin verification, S3 origin access
  control, ECS target-group health, renderer/object/database networks, and deployment tests.
- Depends on: WP-BOT-5.
- Acceptance criteria:
  - Direct requests to each S3 origin, ALB address, object API, renderer, worker control, database,
    and semantic health path refuse from the public test network.
  - CloudFront reaches both S3 origins and the ALB only through the declared origin identities.
  - The ALB requires both the CloudFront origin-facing network source and the WP-RC10
    second condition; either condition alone is insufficient.
  - Target-group `/health` verifies application dependencies from inside the deployment network and
    never appears as a CloudFront behavior.
  - The selected second condition has a bounded migration or rotation state, explicit expiry, and
    rollback from partial cutover; it never extends an overlap indefinitely.
- Evidence or review: permanent offline infrastructure-policy behavior with injected representative
  current/next condition values, clock, expiry, and rollback state; one-time disposable public-
  versus-internal origin/health matrix.
- Next dependency: WP-BOT-7 and WP-BOT-8 consume the verified origin boundary.

### Work package: WP-BOT-7 implement edge controls

- Owner: expert coder.
- Touch points: CDN cache policy, WAF/rate rules, `robots.txt`, `X-Robots-Tag`, and operator runbook.
- Depends on: WP-BOT-1 and WP-BOT-6.
- Acceptance criteria:
  - Every rule carries the required ID/owner/matcher/action/exemption/metric/state/removal fields and
    the deployed priority matches the frozen order.
  - A spoofed Google/Bing user agent without the provider's verified-crawler signal receives ordinary
    treatment; verified crawlers receive only the explicitly approved exemption.
  - Login/session endpoints use buckets calibrated from the versioned shared-egress scenario and
    remain independent from static assets and authenticated API traffic.
  - Every enforced threshold records the report-only legitimate/abusive measurements, observed
    variation, safety rationale, date, and approver; overlapping traffic remains unenforced.
  - Normal, elevated, and emergency modes are versioned and reversible; emergency recovery, expiry,
    and explicit named renewal are encoded as configurable state-machine behavior rather than
    prose-only timing constants.
  - Country/ASN blocks are absent from the initial production policy; a later rule requires a dated
    evidence record, owner, exception path, and removal date.
- Evidence or review: permanent offline rule-order/state-transition behavior with injected policy
  values; one-time generated crawler, verified-bot spoof, IPv4/IPv6, versioned shared-egress,
  VPN/datacenter, and challenge-accessibility comparison.
- Next dependency: WP-BOT-8 consumes the rule action metrics and stable IDs.

### Work package: WP-BOT-8 add cost and scaling guardrails

- Owner: expert coder.
- Touch points: budgets, CloudWatch alarms, Fargate max capacity, database/object/CDN metrics, and
  escalation runbook.
- Depends on: WP-BOT-1 and WP-BOT-7.
- Acceptance criteria:
  - Every actionable cost or capacity risk has an evidence-derived signal, owner, trigger,
    evaluation window, immediate action, investigation path, and recovery condition. Diagnostic and
    binary contract metrics may remain report-only when an alert would add no operator decision.
  - API maximum task count follows the measured class-start capacity and the main plan's replica-
    failure expectation; worker and renderer maxima cannot increase from anonymous HTTP traffic.
  - Budget alerts give the owner enough measured lead time to act before the configured monthly
    budget is exhausted and include a forecast alert. Percentages remain deployment tuning, not
    source or permanent-test constants.
  - The finite log sample cap and lifecycle participate in cost monitoring so defensive telemetry
    cannot become the largest bot bill.
- Evidence or review: permanent offline behavior proving that absent or unbounded scale, sampling,
  lifecycle, and budget controls refuse and that anonymous traffic cannot raise worker/renderer
  capacity; one-time checked-plan review confirms live dashboard metadata, alarm injection,
  capacity/replica-loss, scale ceilings, and notification delivery.
- Next dependency: supply the injectable policy schema to WP-BOT-9; deployed thresholds and live
  dashboard evidence go only to WP-BOT-10 and the operator runbook.

### Work package: WP-BOT-9 freeze permanent adversarial gates

- Owner: tester.
- Touch points: existing unit/conformance tests, local browser tests, and offline infrastructure-
  policy tests. Live cloud and traffic evidence stays in WP-BOT-10.
- Depends on: WP-BOT-3, WP-BOT-4, WP-BOT-5, WP-BOT-6, WP-BOT-7, and WP-BOT-8.
- Acceptance criteria:
  - Keep only the permanent cases that meet the repository checklist: durable behavior that can be
    exercised offline, deterministically, quickly, and with inline inputs.
  - The suite fails when static/API cache behaviors swap, a cookie reaches `www`, SPA fallback masks
    an API error, either origin-shield condition disappears, malformed/unknown sessions gain work,
    a user-agent spoofs verified status, or emergency expiry is removed.
  - Add no fixture directory, captured network corpus, production credential, provider secret,
    answer, student record, or live external dependency. Reuse existing product fixtures only when
    their shipped behavior is the subject.
- Evidence or review: the focused permanent suite passes once through its normal front-door command;
  temporary mutation probes demonstrate important test sensitivity during implementation and are
  removed before handoff.
- Next dependency: freeze the exact command for WP-BOT-10 and the main M6 gate.

### Work package: WP-BOT-10 run live acceptance and close the plan

- Owner: integrator.
- Touch points: maintained E2E runner, Playwright network tests, deployment runbook, M6 tracker, and
  changelog.
- Depends on: WP-BOT-9.
- Acceptance criteria:
  - Cold and warm public crawler, anonymous app crawler, absent/malformed/random-valid-token spray,
    spoofed/verified crawler, legitimate versioned shared-egress login, authenticated class-start
    burst, origin
    bypass, and emergency-mode scenarios produce the required outcome classes and request-correlated
    dependency evidence. The report avoids exact incidental counter totals when retries, provider
    settlement, or cache fill legitimately varies.
  - One-time campus, residential, VPN/datacenter, assistive-technology, IPv4/IPv6, and international
    reviews complete without a hard block; challenged paths have a keyboard/screen-reader usable
    recovery.
  - Before/after evidence reports `bot_cost_per_10k`, PLE origin/dependency calls, bytes, cache hits,
    scale, alerts, and legitimate failures using the same corpus.
  - An independent security reviewer and operations reviewer report no P0/P1; every accepted P2 has
    an owner and dated follow-on.
- Evidence or review: one fresh disposable deployment rehearsal, one-time cost report, emergency
  enable/recover/expire rehearsal, and two independent reviews. These cloud checks are not invoked by
  `pytest tests/`, the ordinary Node/Rust suites, or the local Playwright runner.
- Next dependency: update M6 status and changelog, preserve the evidence artifact, then archive
  this plan with `git mv` only when every exit criterion passes.

## Acceptance criteria and gates

- Per-patch gate: the package's own acceptance cases, strict lint/format/type checks, secret-free
  logs, scoped diff checks, and the existing security/tenant gates pass before a dependent package
  starts.
- Landing gate: cold and warm crawls of `www` cause exactly zero PLE API, database, object, queue,
  renderer, provider, grading, or semantic-health calls. Warm requests use CloudFront cache; cold
  misses may reach only the private landing S3 origin.
- Cache gate: cookies, authorization headers, hostile query strings, alternate encodings, unknown
  paths, and unsupported methods neither poison/fragment static cache entries nor cache an API or
  `Set-Cookie` response. Unknown API paths stay API 404s rather than SPA HTML.
- Authentication gate: absent/malformed tokens cause zero Store calls; a well-formed unknown token
  causes exactly one session lookup and zero other dependency calls. Every refusal has the same
  existence-hiding status/body class and `private, no-store`.
- Origin gate: public probes cannot reach S3, ALB, semantic health, renderer, worker controls,
  database, or object API directly. CloudFront/internal probes succeed through only their declared
  identities.
- Legitimate-use gate: WP-BOT-1's versioned class-start scenario completes behind one shared egress
  identity with no rate refusal or failed submission. One-time VPN/datacenter, international,
  IPv4/IPv6, keyboard, and screen-reader reviews remain usable.
- Cost gate: in a quiescent disposable environment, the generated anonymous workload produces zero
  request-correlated expensive operations and zero worker/renderer scale-out. Record normalized
  `bot_cost_per_10k` with idle-window uncertainty; a dependency call causally attributed to an
  anonymous request blocks progression, while unrelated background counters do not make the gate
  flaky.
- Recovery gate: deployment rollback selects the prior immutable static manifests; the selected
  origin-authentication mechanism restores its prior bounded state; every WAF mode returns to normal
  automatically or by the documented single action.
- Independent review gate: security and operations reviewers find no P0/P1 bypass, unbounded cost
  path, inaccessible challenge, cookie leak, direct-origin exposure, or false completion claim.

## Test and verification strategy

This strategy is cross-checked against [docs/REPO_STYLE.md](../../REPO_STYLE.md),
[docs/PYTEST_STYLE.md](../../PYTEST_STYLE.md),
[tests/TESTS_README.md](../../../tests/TESTS_README.md),
[devel/DEVEL_README.md](../../../devel/DEVEL_README.md), and the browser-specific rules in
[docs/PLAYWRIGHT_TEST_STYLE.md](../../PLAYWRIGHT_TEST_STYLE.md). Apply the permanent-test checklist;
a useful implementation probe does not become a permanent test merely because it found a bug once.

### Permanent tests

- Rust or Node unit/conformance tests exercise session refusal, authority-before-dependency,
  idempotency, route-cost classification, rate-rule ordering, emergency-state transitions, cache
  policy, and redaction as pure behavior. Inputs stay inline, clocks and tunable policy values are
  injected, and dependency fakes count only calls relevant to the asserted behavior.
- A local Playwright test under `tests/playwright/` loads the built landing output over HTTP, uses
  role/label selectors, activates the visible keyboard-accessible sign-in link, intercepts that
  navigation to a local target, and separately checks the configured production destination. It
  observes that the landing starts no cookie, storage, service worker, third-party request, or PLE
  API request. It uses web-first readiness rather than sleeps and asserts no pixel equality, elapsed
  milliseconds, byte total, or animation magnitude.
- OpenTofu keeps offline policy tests that evaluate a generated
  plan without AWS credentials. They assert security behavior -- private origins, disjoint cache/API
  behavior, finite limits and expiry, and rollback state -- while accepting injected TTLs, rate
  thresholds, budgets, and durations.
- Reuse the existing cross-tenant, answer-secrecy, session-replica, public/private asset, and
  readiness gates. Do not duplicate them in bot-specific fixture files.
- Add a fast `tests/test_*.py` only for a cross-language architectural rule that cannot live with its
  owning Rust/TypeScript code and meets every pytest checklist item. It remains offline, uses inline
  inputs or `tmp_path`, runs well under one second, and does not invoke a subprocess.
- Do not add permanent tests for exact config defaults, file or metric-key counts, serialized byte
  identity, screenshot pixels, wall-clock performance, provider-assigned values, or current cloud
  pricing. Review the schema or behavior that matters and keep tuning measurements in the one-time
  evidence instead.

### One-time checks

- Reconfirm S3/CloudFront terms, quotas, region, price model, and budget immediately before M6
  provisioning; record the date and sources.
- Inspect the landing build graph and compressed transfer, render a responsive contact sheet around
  its actual CSS breakpoints, measure contrast, and compare the result with the authenticated app.
  These measurements guide the implementation; they do not create byte, pixel, or timing goldens.
- Exercise DNS/TLS, private origins, selected origin-authentication rotation, cache poisoning, drift
  detection, destroy, static-manifest rollback, alarms, budget notifications, scale ceilings, and
  emergency modes against one disposable cloud deployment.
- Generate the crawler workload and repeat it until the normalized cost estimate is stable enough for
  the owner to choose policy. Compare the same workload before and after controls and reconcile every
  material cost category within the provider's billing resolution; do not impose an unsupported
  percentage tolerance.
- Review report-only challenge and rate outcomes for the versioned class-start/shared-egress
  scenario and available campus, residential, VPN/datacenter, international, IPv4/IPv6, keyboard,
  screen-reader, and reduced-motion paths. Real pilot traffic can strengthen this evidence but is
  not required for code completion. Reuse the main M6 synthetic class-start rehearsal when it exists
  instead of adding a duplicate permanent test.
- Use temporary mutation probes during implementation to show that the important permanent tests
  fail when their boundary is removed. Delete the mutations and scratch harness before handoff.

### File and fixture placement

- Keep small permanent test inputs inline. This plan adds no `tests/fixtures/` directory or captured
  request corpus; adding shared fixture infrastructure requires explicit human approval under the
  repository fixture policy.
- Keep browser tests in `tests/playwright/`. Keep maintained local whole-system orchestration in
  `tests/e2e/`, outside pytest. Regular test commands make no external network or cloud connection.
- Put screenshots and browser traces in gitignored `test-results/`. Put reproducible generated output
  under gitignored `generated/`. Store the concise one-time conclusions and command evidence in the
  owning `docs/active_plans/workstreams/` report, not the raw traffic capture.
- Keep an operational helper in `devel/` only when maintainers will reuse it for drift, cost, or
  incident rehearsal. Otherwise use a scratch file under `/tmp` and remove it after the recorded
  one-time check. Product code stays in its owning `src/` or Rust capability module.

## Risk register

| Risk                                              | Impact | Trigger                                                                               | Owner               | Mitigation                                                                                                                                                  |
| ------------------------------------------------- | ------ | ------------------------------------------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| GitHub Pages use conflicts with service terms     | High   | A showcase starts serving commercial PLE operation                                    | Architect           | Production is fixed to S3/CloudFront; remove the separate showcase if its current use no longer clearly fits GitHub's terms                                 |
| Bots follow the sign-in link                      | High   | Anonymous app-origin ratio or `bot_cost_per_10k` exceeds its accepted baseline        | Edge owner          | Static app shell, exact WAF buckets, one-lookup unknown-session ceiling, and emergency mode                                                                 |
| Random cookies force database lookups             | High   | Unknown-session lookups approach the API or database alarm                            | Auth owner          | One indexed lookup maximum, missing/malformed zero lookup, edge invalid-session bucket, no other dependency work                                            |
| Challenge blocks legitimate students              | High   | A recorded legitimate-use scenario hard-blocks or cannot recover accessibly           | UI/UX owner         | Return the rule to report-only immediately; revise matcher or remove it before enforcement                                                                  |
| Country or ASN rules exclude remote learners      | High   | Legitimate pilot/support evidence shares the proposed blocked geography/network       | Operations owner    | Initial policy has no country/ASN block; later evidence record requires challenge-first action, exception path, removal date                                |
| Public health endpoint amplifies DB/object probes | Medium | Any public probe reaches semantic `/health`                                           | Deployment owner    | No CloudFront health behavior; permanent offline policy behavior plus a one-time disposable public/internal matrix                                          |
| Direct origin bypass defeats edge controls        | High   | Either origin-authentication condition alone reaches ALB/S3                           | Deployment owner    | Private S3 OAC plus two reviewed ALB origin conditions; the preferred design is CloudFront network allowlist plus a safely materialized rotated header      |
| Cache poisoning crosses users                     | High   | Attacker-controlled query/header/cookie changes a cached clean response               | Deployment owner    | Permanent offline cache-key/response-policy behavior plus a one-time disposable clean-client replay                                                         |
| Autoscaling converts bot traffic into spend       | High   | Anonymous traffic changes worker/renderer scale or exceeds API ceiling                | Operations owner    | Anonymous work cannot enqueue; API ceiling follows the versioned class-start and replica-failure evidence; alarms and emergency mode                        |
| Defensive logs become the largest bot cost        | Medium | Logging materially changes the measured defense cost or exceeds its configured budget | Observability owner | Aggregate metrics first; finite evidence-based sample and expiry; cost alarm on the logging category                                                        |
| Public immutable assets are scraped               | Medium | CDN egress alarm rises without authenticated catalog use                              | Asset owner         | Landing links no problem assets; authenticated catalog provides no public listing; URLs remain non-secret and answer-free; CDN cache absorbs repeated reads |
| Static host outage blocks discovery               | Low    | `www` fails while `app` remains healthy                                               | Deployment owner    | Independent hosts, documented direct app URL, previous immutable landing manifest rollback                                                                  |
| Console drift bypasses reviewed edge policy       | High   | Deployed DNS/CDN/WAF/alarm state differs from the checked declarative plan            | M6 architect        | One deployment root, drift gate before release, emergency console changes imported before incident closure                                                  |

## Rollout and release checklist

- [ ] Implement and accept the OpenTofu root in `deploy/opentofu/`; prove restricted encrypted state,
      no-secret plan/log output, drift detection, policy-test access, rotation/rollback, and safe
      disposable destroy.
- [ ] Implement and accept institutional OIDC through WP-RC8; record its issuer/client, callback,
      PKCE, anti-replay, login CSRF, recovery, and abuse-control evidence.
- [ ] Freeze route classes and baseline metrics before selecting production rules.
- [ ] Reconfirm and record the current S3/CloudFront terms, quotas, price model, region, and budget.
- [ ] Build immutable landing/app manifests and prove rollback locally before DNS work.
- [ ] Deploy `www` and `app` on distinct verified TLS hosts without wildcard DNS.
- [ ] Confirm landing has no credentials, cookies, forms, API calls, or protected content.
- [ ] Confirm session cookie is host-only and all app API calls are same-origin.
- [ ] Confirm direct origins and semantic health are unavailable from the public internet.
- [ ] Run the cold/warm cache and cache-poison matrix before WAF enforcement.
- [ ] Deploy aggregate metrics, sample caps/lifecycle, alarms, and scale ceilings before
      blocks/challenges.
- [ ] Add each declared crawler block, endpoint bucket, and challenge rule in AWS WAF count mode.
- [ ] Replay the generated crawler workload and recorded legitimate scenarios; remove overlapping
      signals and enable only rules that preserve every legitimate scenario.
- [ ] Cut DNS with the prior distribution configuration and immutable manifests retained for one-
      action rollback.
- [ ] Rehearse elevated and emergency modes in a disposable environment.
- [ ] Run class-start and anonymous-burst scenarios together.
- [ ] Inject every configured budget and forecast alarm and verify delivery to its named owner.
- [ ] Record before/after `bot_cost_per_10k`, origin/dependency counts, false positives, exceptions,
      final thresholds, and rollback evidence.

## Documentation close-out requirements

- Active plan / progress tracker: update this file and the M6 `MOD-DEPLOY` status in
  `implementation_plan.md` or its active tracker.
- `docs/CHANGELOG.md` entry: record host choice, route split, permanent gates, measured effect, and
  any accepted challenge exceptions.
- Archive / closure notes: move this plan to `docs/archive/` with `git mv` only after the integrated
  deployment evidence passes.

## Patch plan and reporting format

- Patch 0A: WP-RC10 creates the OpenTofu root and passes disposable
  secret-materialization/state/plan/drift/rotation/destroy proof. This blocks Patch 3, not
  measurement, inventory, or landing work.
- Patch 0B: WP-RC8 implements institutional OIDC and passes its credential/callback/PKCE security
  contract. This blocks provider-specific parts of Patch 2B, not its provider-neutral refusal work.
- Patch 1A: WP-BOT-1 privacy-bounded cost metrics and generated workload definitions.
- Patch 1B: WP-BOT-2 closed route-cost inventory and source assertion; runs parallel with 1A.
- Patch 2A: WP-BOT-3 independent landing artifact and browser/artifact tests.
- Patch 2B: WP-BOT-4 cheap session/credential refusal and counting tests; runs parallel with 2A.
- Patch 3: WP-BOT-5 exact `www`/`app` cache, route, DNS, cookie, CSP, and CORS behavior.
- Patch 4: WP-BOT-6 origin shield, private health, and rotation matrix.
- Patch 5: WP-BOT-7 versioned WAF/rate/emergency policy and measured network scenarios.
- Patch 6: WP-BOT-8 cost ceilings, sample lifecycle, alarms, and budget notifications.
- Patch 7: WP-BOT-9 permanent adversarial cross-boundary suite.
- Patch 8: WP-BOT-10 live crawler/class-start rehearsal, cost report, reviews, tracker, and changelog.

Each patch report states: owned files, behavior changed, permanent tests, one-time checks, measured
origin/cost effect, legitimate-user impact, rollback, and remaining dependency IDs.

## Decision completeness

No bot-cost scope or implementation decision remains open. OpenTofu, institutional OIDC,
S3/CloudFront, the host split, aggregate edge/server metrics, and the no-client-analytics boundary
are fixed for version 1. A post-v1 analytics proposal requires a separate product question and plan;
it cannot enter this package as implementation telemetry.
