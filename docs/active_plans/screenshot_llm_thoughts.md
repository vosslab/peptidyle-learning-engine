Investigate how to turn the existing screenshot system into a comprehensive, durable visual corpus of the PLE application.

My goal is broader than maintaining a few representative screenshots. I want a reasonably complete visual survey of the major pages, states, and workflows available to each Product Role: Instructor, Student, and Sysadmin, plus important public/signed-out surfaces. The existing 69 screenshots are closer to the intended scope than the current 8 automated captures.

This corpus has two purposes:

1. Document the major user-facing surfaces of the platform.
2. Let me rapidly review the platform as a whole for inconsistent visual language, excessive chrome, weak layouts, missing states, responsive problems, and other UI gaps.

Investigate the current 69 screenshots, application routes, Ribbon contracts, role/capability model, demo personas and seed data, Playwright/e2e infrastructure, screenshot contract, current_capture_manifest.json, and devel/capture_screenshots.sh. Determine what the current application actually needs for comprehensive visual coverage, rather than assuming either the existing 69 files or the current 8 captures define the correct corpus.

Think in terms of coverage by product surface and workflow, not individual PNG files. Investigate whether the repository can define reusable capture scenarios that establish a meaningful application state once and then capture multiple states, routes, roles, or canonical viewports. Look for opportunities to reuse the real demo stack and existing seeded personas so the screenshots represent the real application rather than artificial visual fixtures.

I want the architecture to scale. Investigate whether the manifest should become the declarative source of truth for the visual corpus, with enough information to answer questions such as:

* Which role owns this surface?
* Which route or workflow produces it?
* Which meaningful state is being shown?
* Which canonical viewport should be captured?
* Which scenario prepares that state?
* Is the capture part of the current visual corpus?

Consider whether one scenario should naturally produce several screenshots. A Student assignment-delivery scenario, for example, may expose assignment list, unanswered question, selected response, feedback, completion, and repeat/fresh-session states. Likewise, an Instructor workflow may expose several related assignment, question-library, grading, or course-management surfaces without independently rebuilding application state for every PNG.

Investigate coverage systematically across Instructor, Student, and Sysadmin. Produce a coverage inventory that maps the major current routes/surfaces and important workflow states to existing screenshots and automated capture support. Identify meaningful current surfaces that have no screenshot at all, existing screenshots that duplicate the same useful evidence, and screenshots whose represented behavior is no longer part of the current product.

Be ambitious about responsive coverage where it provides useful visual information. Major pages should have enough canonical viewport coverage to reveal meaningful desktop/tablet/phone differences, but use product behavior and layout differences to determine where multiple viewports add value rather than mechanically multiplying every page by every viewport.

Investigate how devel/capture_screenshots.sh can remain the canonical operator entry point while the underlying implementation scales to many scenarios and captures. A full regeneration should be able to provision the required application state, execute the declared capture corpus, and produce deterministic paths without a person manually navigating the UI.

Define a strong meaning for --verify. It should give confidence that the declared current visual corpus, its manifest, its capture scenarios, and the generated files agree, and that important role/surface coverage has not silently disappeared.

Also investigate how the resulting screenshots can support whole-platform visual review. Consider whether the repository should generate or maintain an index/gallery organized by role, product area, workflow, and viewport so a reviewer can quickly scan the entire UI rather than opening dozens of files individually.

Use the repository to determine the right abstractions and implementation details. Prefer extending existing demo, route, and Playwright infrastructure where those boundaries are sound. Where comprehensive coverage exposes a missing reusable state/setup mechanism, treat that as a design problem worth solving rather than automatically excluding the screenshots.

Come back with:

* a current visual-coverage inventory across Instructor, Student, Sysadmin, and public surfaces;
* an assessment of the existing 69 screenshots against the current application;
* gaps where important current surfaces have no useful visual evidence;
* a proposed architecture for a scalable declarative capture corpus;
* a proposed scenario/workflow model based on repository evidence;
* the strongest practical target for automated coverage now;
* a migration strategy from the current 8 automated captures toward that target;
* a useful whole-platform gallery/review artifact;
* concrete, dispatchable milestones with ownership, success criteria, and behavior-focused verification.

Build the strongest practical system the current application can support. Research first, then write the implementation plan. Do not begin implementation yet.
