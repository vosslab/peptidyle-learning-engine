# Frontend capability integration

This checklist is the admission path for one future Ribbon destination. It is deliberately
ordered: an earlier unchecked responsibility blocks every later one. It does not authorize a
destination, create a route, or make a Service available. The Application Shell owns the Ribbon;
the capability registry is its truthful visibility ceiling; route access boundaries and the server
remain the authorization owners.

A destination is **backed** only when its complete usable path exists: a declared and mounted
route, a real page rather than a placeholder, the typed browser client method used by that page,
and the registered server handler that method targets wherever a call is required. A page that
genuinely needs no server call may be backed with a specific no-server-call rationale. A route,
client method, server test, or structural fixture by itself never makes a destination backed.
Ribbon visibility never grants permission to view or perform an operation.

Use the repository's precise capability vocabulary in evidence and review:

- **Server Route exists** only after the production server registers the handler and its request
  boundary is usable.
- **Service is implemented** only after its owned behavior and Store capability are implemented;
  it does not imply that a Server Route exists.
- **Browser Surface is available** only after the complete browser-facing path is usable for its
  permitted Product Roles.
- **Mount** only describes attaching a UI component, route component, or container to a rendered
  composition. Do not use it as a synonym for registering a Server Route or implementing a Service.

## Admission checklist

1. **Name and position the destination before implementation.** Confirm its canonical visible name
   and semantic layer in [interface terminology](../INTERFACE_TERMINOLOGY.md), then locate its
   exact catalog control in [the Ribbon catalog](../../src/ribbon/ribbon_catalog.ts). Preserve the
   designed Ribbon Scope, Tab or Task Area, role, priority, presentation, and position. A new
   destination uses a declared append position where the schema permits growth; it does not reorder
   users' existing controls. Confirm whether the proposed control is navigation or a Page Action.
   Page Actions belong with their content and never become Ribbon Tabs or Ribbon Tasks merely to
   obtain visibility.

2. **Declare the executable route and real page, or record why no browser route is appropriate.**
   Add the route contract only when there is a real browser destination with a real content owner.
   The route's path, Product Role ceiling, Ribbon Scope, selection metadata, and Content Layout
   belong in [the route contract](../../src/route_contract.ts). Attach the real route component to
   the route composition, preserving its [route access boundary](../../src/route_access_boundary.tsx).
   If the capability has no Browser Surface, keep it out of the Ribbon and document that it has no
   browser-route rationale. A declared route or a component mounted in a fixture is not backing.

3. **Put authorization at its actual owners.** Confirm the route's declared Product Role ceiling
   and the server-side authorization and relationship checks that protect each request. The
   [route access boundary](../../src/route_access_boundary.tsx) is a browser fail-closed boundary;
   it cannot authorize a server operation. Do not infer permission from catalog membership,
   visibility, a client-side role check, or a successful test fixture.

4. **Design the typed browser operation.** Add or extend the narrow typed method in
   [the browser API client](../../src/api/client.ts), its request/response contracts and strict
   decoding at the existing client boundary. Give the page one meaningful operation rather than a
   generic transport escape hatch. Connect cached page-facing queries through
   [the application API](../../src/api/application_api.tsx) where the operation is route data.
   The method's exact name is evidence, not proof that its endpoint exists.

5. **Implement the Service and its Store capability.** Give the domain behavior an owned Service
   and Store contract, preserving the boundaries recorded in [the contract register](../CONTRACTS.md).
   Establish its behavior, authorization, and failure semantics with the evidence appropriate to
   that boundary. A Service is implemented only when this owned behavior is present; a service
   unit test alone does not establish a Server Route or Browser Surface.

6. **Register the production Server Route.** Add the handler to the production composition and bind
   the typed browser operation to that handler. The production registration point is
   [server composition](../../crates/server/src/composition.rs). Record the handler's exact
   symbol/path, request contract, service owner, Store owner, authorization rule, and expected
   failure mapping. Only after this registration and usable request boundary may evidence say that
   a Server Route exists.

7. **Prove the complete usable path before admission.** Exercise the declared route, real page,
   typed browser method, registered handler, Service, Store capability, authorization outcome, and
   user-visible result as one claimed path. Include both a permitted path and an appropriate
   denial or recovery path. If a no-server-call destination is proposed, prove the real route and
   page and record why no server operation is needed. A hand-written `RibbonModel` or browser
   fixture may prove shell structure, but never this production-capability claim.

8. **Specify relationship presentation and caching when the destination depends on one.** If
   visibility needs a relationship beyond immutable Product Role, declare its requirement in the
   [Ribbon schema](../../src/ribbon/ribbon_schema.ts), identify its data owner and cache/query key
   in [the application API](../../src/api/application_api.tsx), and preserve the synchronous
   schema. While the relationship is outstanding, availability is `Checking` and the control is
   withheld; when it resolves allowed, a new admitted control may only join the designed suffix.
   Loading may fill labels or availability but may not reshuffle, withdraw, or resize visible
   controls.

9. **Change one registry declaration from unbacked to backed.** In
   [the capability registry](../../src/ribbon/capability_registry.ts), retain the catalog identity
   and record all of the following: `RouteId`; exact typed client method; either the registered
   handler evidence or an explicit no-server-call justification; reviewable source references; and
   the relationship requirement. `createRibbonCapabilityEntry` and `ribbonAvailability` are the
   admission boundary. Do not bypass them in the page, shell, catalog, or CSS. The registry change
   follows the proof in steps 1--8; it does not substitute for it.

10. **Verify model derivation and shell admission.** Confirm that
    [model derivation](../../src/ribbon/ribbon_contract.ts) projects the admitted entry only for
    the intended Ribbon Scope, Product Role, route selection, and resolved relationship facts.
    Confirm that [the production application](../../src/app.tsx) supplies the model and that
    `src/application_shell.tsx` remains the single persistent Ribbon
    owner. Do not create a second navigation surface, fetch topology in the Ribbon, or turn a
    route transition into a remount.

11. **Finish the page's loading, error, and recovery states.** The content owner supplies clear
    loading, empty, failure, retry, and denied states appropriate to the new operation. The shell's
    content-only error boundary in `src/application_shell.tsx` must leave the Ribbon usable.
    After a successful Ribbon navigation, focus moves to `#main-content`; a skipped or failed
    request must not leave stale success copy or a falsely selected inaccessible destination.

12. **Run the visible-interface review.** Check keyboard reachability and DOM-order navigation;
    accessible names with icon-plus-label controls; visible selected and pending state; focus
    treatment; 200 percent text and narrow-phone reflow; responsive presentation; ordinary and
    course-theme contrast; `forced-colors`; and `prefers-reduced-motion`. Apply the geometry and
    density rules in [the UI design guide](../UI_DESIGN_GUIDE.md): application state may alter
    selection and content, never the Ribbon's slot positions or block geometry. A new destination
    must not cause a visible control to move.

13. **Refresh the generated ledger and retain human context.** Run the M12 ledger generator owned
    by `devel/` after the registry edit, commit its generated machine-owned columns, and refresh the
    editorial explanation in `docs/ux/RIBBON_DESTINATION_LEDGER.md`. The
    generated record must name canonical label, route id, client method, handler/no-server evidence,
    and derived Ribbon Availability. Do not hand-edit generated values to make the document agree.

14. **Close with durable tests, review, and changelog evidence.** Add permanent tests only for
    behavior that can plausibly regress and meets the deterministic admission rule in
    [the test evidence model](../TEST_EVIDENCE_MODEL.md). Run the focused contract, decoder,
    authorization, registry, model, and browser checks appropriate to the changed path, then the
    required repository gates. Obtain independent review of both capability truthfulness and
    visible behavior. Record the completed bounded work, exact commands, outcomes, environment
    assumptions, and unrun required evidence in [the changelog](../CHANGELOG.md); do not describe
    an unavailable external lane as passing.

## Evidence record template

Create one evidence record per destination admission. Replace every bracketed value with an exact,
openable repository link and a real symbol, not a line-number snapshot, prose claim, or test-fixture
name. The links below show the required authority locations; they do not evidence any particular
future destination.

| Record field                                | Required evidence                                                                                                                                                                        |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Canonical destination and designed position | [Interface terminology](../INTERFACE_TERMINOLOGY.md) and [Ribbon catalog](../../src/ribbon/ribbon_catalog.ts), with the exact control id and Tab/Task/Page Action classification.        |
| Route and page                              | [Route contract](../../src/route_contract.ts) with `RouteId`, plus the mounted real page component. Or: explicit no-Browser-Surface rationale.                                           |
| Product Role and authorization              | [Route access boundary](../../src/route_access_boundary.tsx), named server authorization owner, and relationship rule if applicable.                                                     |
| Typed browser method                        | [API client](../../src/api/client.ts), request/response contract and decoder location, plus [application API](../../src/api/application_api.tsx) query key if cached.                    |
| Service and Store                           | [Contract register](../CONTRACTS.md), exact Service module/symbol, exact Store module/symbol, and service/Store evidence.                                                                |
| Server Route                                | [Production composition](../../crates/server/src/composition.rs), exact handler symbol, request mapping, and failure mapping; or explicit no-server-call justification.                  |
| Complete usable-path evidence               | Exact focused test or disposable acceptance command/result and browser scenario where available, including permitted and denied/recovery outcomes.                                       |
| Registry admission                          | [Capability registry](../../src/ribbon/capability_registry.ts) entry id, `RouteId`, client method, handler/no-server evidence, relationship requirement, and ledger regeneration result. |
| Shell and accessibility                     | [Ribbon model](../../src/ribbon/ribbon_contract.ts), [Application Shell](../../src/application_shell.tsx), browser evidence, and responsive/forced-colors/reduced-motion review record.  |
| Review and release record                   | Independent review artifact, required-gate results, limitations, and [changelog](../CHANGELOG.md) entry.                                                                                 |

## Evidence classes and gate discipline

Permanent gates protect maintained deterministic behavior: typed decoding, authorization boundaries,
registry validation, model admission, and stable browser behavior where its owner exists. They are
not proof that a dependent service, live browser environment, or one-time migration is available.

One-time migration or implementation proof records a narrow completed transition: a schema/data
migration, a production-registration reconstruction, a rendered inspection, or a disposable
acceptance receipt. Keep its command, environment, claim, and limits with the handoff or evidence
record. Do not promote an inventory, source count, screenshot, or migration snapshot into a
permanent gate merely because it is easy to run. The classification and admission test are defined
in [the test evidence model](../TEST_EVIDENCE_MODEL.md).

## Rollback and de-admission

If later evidence shows that any required link is incomplete, unavailable, unauthorized, or unsafe,
de-admit the destination immediately:

1. Change its single registry declaration back to `unbacked`, state the concrete reason, and retain
   openable evidence of the missing or withdrawn path. Do not leave a disabled or misleading live
   Ribbon control.
2. Regenerate the machine-owned destination ledger and revise the editorial explanation so the
   document remains truthful.
3. Keep the route, client, Service, and Server Route only when they remain independently valid and
   safe; removal or rollback of those owners follows their own migration and authorization review.
   Registry de-admission is not a substitute for repairing or retiring an unsafe server operation.
4. Run registry/model and affected browser checks, recheck keyboard and geometry stability, obtain
   independent review, and record the de-admission, user-visible effect, and unresolved recovery
   work in the changelog.

The de-admitted catalog position remains designed but omitted. Restoring visibility repeats this
entire checklist; restoring an old `backed` declaration without renewed complete-path evidence is
not permitted.
