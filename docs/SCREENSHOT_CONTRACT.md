# Screenshot contract

## Purpose

Current screenshots are evidence of real, functional PLE application surfaces.
The Live Demo is only the reproducible disposable environment with seeded data
used to produce those captures. It is not a presentation layer, screenshot
gallery, alternate UI, or separate product surface.

Each current capture reaches a normal role- and relationship-gated PLE surface
through visible navigation and a functional workflow. A screenshot requirement
never justifies a non-functional or presentation-only surface. Repair the
application boundary when necessary so the capture comes from the working
system.

## Ownership

Screenshot ownership follows the user-facing Product Role surface. Instructor,
Student, and Sysadmin workflows have role-owned screenshots even when their
implementations reuse shared components:

- `docs/screenshots/instructor/` owns Instructor surfaces.
- `docs/screenshots/student/` owns Student surfaces.
- `docs/screenshots/sysadmin/` owns Sysadmin surfaces.

Current authenticated screenshots belong to the Product Role through which the
surface is reached. Shared implementation does not create shared screenshot
ownership. Pre-authentication surfaces belong under `docs/screenshots/public/`.

Screenshot organization must not encourage generic role-switched presentation
pages. Capture the actual role-owned page and workflow that a person reaches in
the application; reuse low-level components without combining distinct pages.

Playwright `getByRole(...)` names accessible UI roles such as buttons, links,
headings, and dialogs. It is unrelated to PLE Product Roles and remains the
preferred selector for navigating these role-owned workflows.

There is no `docs/screenshots/shared/` or `docs/screenshots/live_demo/`
namespace. Deterministic seeded data does not own presentation chrome.

`docs/screenshots/current_capture_manifest.json` is the closed declaration of
current canonical captures. Its records describe identity, ownership, product
area, workflow state, scenario checkpoint, canonical viewport, privacy profile,
and gallery presentation. They never contain selectors or executable steps.
The TypeScript scenario registry is the one executable mapping from those
scenario and checkpoint names to normal product workflows.

The active tree is flat within exactly four role folders. A successful rebuild
removes screenshots not declared by the manifest. The generated
`current_capture_receipt.json` binds the manifest digest, exact path set,
dimensions, and image hashes. [SCREENSHOT_ATLAS.md](SCREENSHOT_ATLAS.md) is the
generated, grouped visual review surface. The manifest's route and Ribbon
coverage ledger accounts for current captures and concrete deferred product
capabilities.

## Rebuild and verification

`./devel/capture_screenshots.sh` is the supported rebuild entry point. It
starts the seeded stack it owns, navigates normal visible PLE workflows, checks
route, semantic-state, privacy, page-error, origin, and dimension invariants,
then publishes every declared artifact and stops that stack. Capture support
must use ordinary routes, persisted product state, normal HTTP contracts, and
visible application actions. It must not add screenshot-only routes, mocked
responses, or fabricated backend state.

`./devel/capture_screenshots.sh --verify` first validates the manifest,
registry, PNG set, receipt, dimensions, and atlas, then starts a clean Live Demo
and replays the complete corpus with the same assertions. It leaves replay
artifacts under `test-results/screenshot-corpus/verify/` and proves that the
published images were not modified. Byte differences are reported for human
review but are not a pass/fail pixel-equivalence gate.

Publication validates staging before changing the active corpus. Portable
filesystems cannot atomically replace four directories plus two files, so
publication keeps a complete recovery backup while replacement is in progress,
rolls back ordinary failures, and refuses to overwrite evidence from an
interrupted publication. This is recoverable promotion, not a filesystem
transaction.

Neither command replaces the permanent behavioral suite. Machine-clean replay
proves reproducible semantic states; human visual review determines whether the
current design is launch-ready.

## Ribbon visual evidence

The Ribbon is shared application chrome and receives explicit visual coverage.
Each Product Role keeps at least one current canonical desktop capture that
shows the normal Ribbon in a representative functional workflow. The capture
uses a real role-owned page and normal navigation state, so it is evidence that
the Ribbon remains visually coherent as the application evolves. Add tablet or
phone Ribbon captures only where responsive behavior materially changes the
Ribbon. They remain evidence of the working application, never a separate
presentation page.

The canonical profiles are laptop 1280 by 800, portrait tablet 800 by 1280,
phone 393 by 852, and square 800 by 800 CSS pixels. Add a non-laptop variant
only when the responsive composition, control layout, or access outcome changes
materially enough to need separate visual inspection.

## Privacy

Current captures exclude Answer Keys, correct answers, private Question Source
data, credentials, tokens, internal bindings, and unrelated Student records.
Each closed privacy profile is tied to a demonstrated capture state: public,
Instructor answer-free, Student unanswered, the fictional Student's selected
response, Student self-only status, authorization denial, Sysadmin account
administration, or a specifically authorized scoped roster projection.
