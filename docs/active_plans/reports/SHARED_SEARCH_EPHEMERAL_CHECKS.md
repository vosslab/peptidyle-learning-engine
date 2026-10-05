# Shared search ephemeral checks

## Final M15 probe: 2026-10-05

`node --import tsx tests/_temp/shared_search_ephemeral_probe.mjs` passed against the deterministic
compiled current-source M15 fixture in a fresh Chromium context.

- The Question Picker began after its initial load at request 1. Typing `protein` made no request;
  Enter made exactly one request, bringing the count to 2; switching to Visual boxes made no
  additional request.
- The Assessment content picker began at request 3 after its initial load. Typing `protein` made
  no request; Enter made exactly one request, bringing the count to 4; switching to Visual boxes
  made no additional request.
- `localStorage`, `sessionStorage`, and IndexedDB were empty before the picker interactions and
  remained empty afterward.
- A deferred `SearchSession` check resolved `fast` before `slow`; visible state retained only
  `fast-result` after the late `slow` completion.

The picker counts are fixture request counts. They prove the shared submit/display boundary without
claiming a production transport measurement. The stale-response check exercises the shared session
request-generation boundary directly.

The prior M1-M5 fresh-context Library check remains valid evidence for the shared page storage
boundary: Library submit plus List/Visual changes left all three browser stores empty. This final
probe adds both picker consumers rather than replacing that earlier Library observation.

## M16 temporary-check disposition

All classified checks below were removed after their final evidence was recorded. The manager
verified that `tests/_temp/` is empty; permanent behavior remains covered by the named test suites.

| Temporary check                                        | Disposition and retained evidence                                                                              |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
| `audit_shared_search_pool_membership.py`               | One-time preproduction data audit; retain the Pool audit report and connected database rule tests              |
| `library_object_detail_direct_routes.mjs`              | Plan-requested M11 acceptance; retain M11 route/retry receipt and permanent frontend contracts                 |
| `shared_search_speed_baseline.py`                      | M12 rerun complete; retain measured speed report and raw-log references, then remove at M16                    |
| `shared_search_ephemeral_probe.mjs`                    | Final M15 evidence above; remove at M16 after retaining this report and permanent SearchSession ordering tests |
| `shared_search_submit.mjs`                             | One-time submit-boundary check; permanent shared browser tests cover the stable behavior                       |
| `shared_blueprint_capture.mjs`                         | One-time rendered inspection; retained screenshot corpus owns final captures                                   |
| `m12_filter_diagnostic.mjs`                            | One-time selector/filter diagnostic; permanent shared-search tests own the stable control contract             |
| `m14_assessment_content_picker_live.mjs`               | One-time Live Demo M14 picker journey; retain M14 receipt and permanent picker behavior tests                  |
| `m15_blueprint_stars_probe.mjs`                        | One-time Live Demo Blueprint-star verification; retain M15 review evidence                                     |
| `m15_browse_probe.mjs`                                 | One-time Live Demo Browse verification; retain M15 walkthrough receipt                                         |
| `m15_library_pool_probe.mjs`                           | One-time Live Demo Library/Pool verification; retain M15 walkthrough receipt                                   |
| `m15_usability_walkthrough.mjs`                        | One-time independent M15 cognitive walkthrough driver; retain its review report                                |
| `ribbon_review_fixes.mjs`, `ribbon_state_contrast.mjs` | Existing one-time Ribbon render captures; permanent theme/contrast checks own the durable contract             |
| `ribbon_surface_review.mjs`                            | Existing one-time geometry evidence; permanent RecordList layout checks own the durable contract               |
| `__pycache__/`                                         | Disposable interpreter output, removed with the temporary checks                                               |
