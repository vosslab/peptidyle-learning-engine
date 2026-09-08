# Live Demo foundation findings

Status: M0 complete on 2026-09-06.

This is a current-state handoff for the Live Demo restoration. It does not
claim browser acceptance or treat the former developer structural preview as a
product route.
The governing terms remain [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) and
[TERMINOLOGY_CONTRACT.md](../../TERMINOLOGY_CONTRACT.md).

## RLS baseline

The fresh disposable acceptance command below passed on 2026-09-06:

~~~bash
source source_me.sh && python3 local_stack.py acceptance
~~~

It ran the PostgreSQL schema, authority, and persistence oracle and the
course-appearance PostgreSQL and MinIO coherence oracle. The aggregate has no
browser lane yet; this result is database/service evidence only.

The accepted baseline requires every ordinary and partitioned relation in
`ple_data`, `ple_private`, and `ple_audit` to enable and force RLS. The check
is defined by
[schemas/migrations/2026082932_baseline_acceptance_witness.sql](../../../schemas/migrations/2026082932_baseline_acceptance_witness.sql).
Its protected relation classes include Course and Course Membership data,
Account and Authenticated Session data, Assignment Attempt and Question
Submission data, Grading Result data, Jobs and leases, and audit receipts.

The policy catalog gives exact, narrow access rather than a general database
role. For example,
[schemas/migrations/2026082930_forced_rls_policies.sql](../../../schemas/migrations/2026082930_forced_rls_policies.sql)
authorizes an active Session for the exact Authoring Workspace or Course scope,
permits the API owner only the named lookup relations, and keeps owner-only
Session lifecycle mutations private.

The disposable probe also establishes default deny:

- `PUBLIC` has no protected schema, relation, sequence, or target-database
  privilege.
- `ple_app` can read only the safe migration-state projection; it cannot read
  the SQLx ledger or use the data schema directly.
- `ple_auth` and `ple_student` cannot read that projection, use the data
  schema, assume an owner role, or create an API relation.
- The API and grading-worker logins cannot assume each other's authority or
  read iMathAS Question Backend Session data directly.

The runnable denial evidence is
[tests/e2e/e2e_postgres_migration_acceptance.sh](../../../tests/e2e/e2e_postgres_migration_acceptance.sh).
M3 must preserve these results while adding the Live Demo worker boundary; it
must not grant a worker direct protected-table access.

## Browser specifications

The nine Playwright specifications present during this M0 audit had the following M19 inventory.
"Retain" preserves the scenario name and scope; "rename" preserves its
behavior but removes terminology drift; "retire" removes it from the browser
suite while keeping any appropriate unit-level protocol coverage.

| Current specification | M0 disposition | M19 handoff |
| --- | --- | --- |
| `auth_authorization.spec.ts` | retain | Verify Account, Authenticated Session, and exact Course scope. |
| `fault_handshake_worker.spec.ts` | retire | It has no browser and belongs with the worker/fault protocol tests. |
| `instructor_authoring.spec.ts` | retain | Exercise the Instructor's visible authoring task after M5-M7. |
| `instructor_grade_settings_conflict.spec.ts` | rename | Use Course Grade Scheme terminology and retain the concurrent Instructor task. |
| `item_pool_delivery.spec.ts` | rename | Use Question Pool terminology and retain the full Assignment delivery scenario. |
| `learner_delivery.spec.ts` | rename | Use Student and Assignment terminology for the completed and revisited work. |
| `learner_gateway_recovery.spec.ts` | rename | Use Student terminology and retain the accepted-submission recovery boundary. |
| `question_library_discovery_evidence.spec.ts` | retain | Exercise Question Library discovery without copying old selector comments. |
| `webwork_delivery.spec.ts` | rename | Use iMathAS Question Backend terminology for the external rendering boundary. |

Several retained files name source components that no longer exist. M19 owns
their selector-contract refresh against the current routed components; it must
not revive an obsolete route or vocabulary merely to make a selector pass.

M19 retired the candidates whose current routed components remained unavailable,
including their unregistered providers and specifications. The executable
scenario registry, rather than this historical classification table, declares
the current serial browser owner inventory.

## Client and decoder reuse

The current browser has a usable strict transport and real teaching pages.
They are not a substitute for a route-specific Server Route or authorization
check.

| Current seam | Reuse decision | Owner of remaining mismatch |
| --- | --- | --- |
| [src/api/http_client.ts](../../../src/api/http_client.ts) and `src/api/decoders/` | Reuse the same-origin request boundary and a decoder only when the Server Route returns its exact DTO. | Each feature milestone that introduces or restores that Server Route. |
| [src/api/application_api.tsx](../../../src/api/application_api.tsx) | Reuse its injected client and query identities for restored routes. | M5-M16, by route capability. |
| [src/auth/browser_session_boundary.ts](../../../src/auth/browser_session_boundary.ts) and [src/auth/session_context.tsx](../../../src/auth/session_context.tsx) | Reuse unchanged. They abort stale requests and retain only browser-safe session state. | M4 verifies the seeded Session lifecycle; M19 proves it in a browser. |
| [src/routes.ts](../../../src/routes.ts) and `src/pages/` | Reconnect existing routed page implementations only after their data capability is live. | The matching M5-M18 feature milestone. |
| [src/api/live_demo.ts](../../../src/api/live_demo.ts), [src/api/http_client/live_demo.ts](../../../src/api/http_client/live_demo.ts), and [src/pages/sign_in_page.tsx](../../../src/pages/sign_in_page.tsx) | Keep the seeded Account selector as a deployment convenience. | M19 replaces its former post-selection structural-preview navigation with the real persona entry route. |
| Retired Live Demo structural preview source | Do not reuse as a Live Demo destination. | M19 retires the route after the three real persona tasks are accepted. |

There is no supported generic decoder facade to add. A decoder mismatch is
owned by the milestone that owns the server payload and route: M5-M10 for
Instructor/Course/Assignment DTOs, M11-M14 for Student delivery and grading
DTOs, and M15-M18 for Gradebook, Account, support, and export DTOs. M19 may
only compose those accepted contracts.

## M1 handoff

The current disposable stack proves the database baseline but has no Live Demo
worker service. M1 starts from the current fixed browser topology, then M2 and
M3 add worker readiness and least-privilege procedure/lease authority without
weakening this RLS/default-deny baseline.
