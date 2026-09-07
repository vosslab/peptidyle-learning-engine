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
current canonical captures. The rebuild writes only manifest-listed paths and
leaves older role-folder screenshots alone unless a separate task explicitly
updates them.

## Rebuild and verification

`./devel/capture_screenshots.sh` is the supported rebuild entry point. It
starts the seeded stack it owns, navigates normal visible PLE workflows, checks
capture privacy, writes every declared artifact, and stops that stack.

`./devel/capture_screenshots.sh --verify` validates the declared PNG artifacts
and their publication manifest without starting a stack. The rebuild produces
one-time rendered evidence. `--verify` validates that evidence and its manifest.
Neither command is a permanent behavior test or a replacement for connected
browser acceptance.

## Ribbon visual evidence

The Ribbon is shared application chrome and receives explicit visual coverage.
Each Product Role keeps at least one current canonical desktop capture showing
the normal Ribbon in a representative functional workflow. Add tablet or phone
Ribbon captures only when responsive behavior materially changes the Ribbon.
Ribbon captures come from real role-owned pages, not a presentation surface.

## Privacy

Current captures exclude Answer Keys, private Question Source data, protected
Student fields, credentials, and tokens. A Student Question capture shows an
unanswered Question Presentation unless a separately approved evidence task
names a different safe state.
