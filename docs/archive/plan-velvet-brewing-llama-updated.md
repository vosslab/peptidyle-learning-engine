# Plan: Shared frontend shell for implemented surfaces

## Context
Improve the
signed-in shared shell while preserving all current product decisions. A page may exist or appear in navigation only when its
corresponding backend capability already exists.

## Objectives
- Make current, usable destinations obvious and remove frontend paths to unimplemented capabilities.
- Provide a compact, accessible shared shell without changing backend behavior.
- Document a repeatable frontend integration seam for later backend capabilities.

## Design philosophy
Use the frontend as a truthful presentation of implemented capabilities, not a preview catalog. Preserve existing route, role, session, and authorization decisions; frontend capability registration controls visibility, never access.

## Scope
- Audit each
current frontend page against its existing client and backend capability; retain only verified surfaces.
- Add a presentation-only shell destination registry that maps each retained destination to its existing route, page, and frontend client.
- Render desktop navigation inline at 48rem
and above; render the same registered destinations behind one labeled Menu button below 48rem.
- Keep the existing skip link, route-focus transfer, active-location cue, role-filtered inputs, and sign-out behavior unchanged.
- Scope shared header CSS so course and assignment navigation keep their own visual ownership.
- Create docs/ux/ artifacts:
- a current-surface ledger with the frontend/client evidence for each visible page;
- a task model and heuristic/accessibility ledger;
- a frontend integration checklist for future completed backend capabilities.

## Non-goals

- Create pages, navigation items, mock data, placeholders, or routes for future backend work.
- Change passkey, email, sign-in, session, Account, API, schema, server, or authorization behavior.
- Redesign authentication, Sysadmin, Instructor, Student, or course-local workflows.
- Treat frontend visibility as authorization.

## Current state summary

App owns the shared header, navigation, skip link, focus transfer, and route-error boundary. Its global navigation CSS overlaps local navigation
surfaces, and the current shell does not distinguish implemented destinations through an explicit frontend capability record.

## Approach

1. Produce the current-surface ledger from existing frontend clients and implemented backend evidence; remove or suppress every unverified page
from route composition and navigation.

2. Extract the presentation-only shell and its destination registry from src/app.tsx; retain existing state inputs and route predicates.

3. Implement the responsive Menu and shared route-error styling with no new fetches, decoders, or API types.

4. Publish the future-capability checklist: backend capability first, then frontend client/decoder, registered page, state coverage,
accessibility review, and tests.

## Files to modify

- src/app.tsx, a focused shared-shell component, and its destination registry
- src/style.css
- Frontend tests and docs/ux/ evidence/integration documents

## Verification

- Unit tests cover registered-destination rendering, mobile Menu state, Escape/focus restoration, and unchanged route-input handling.
- The surface ledger provides one-time evidence that every visible page has a current backend capability; it is documentation evidence, not a
fragile source-inventory test.

- Keyboard walkthrough covers skip link, desktop navigation, mobile Menu, active destination, and route-error recovery.
- Check accessible names, focus order, 44px mobile targets, contrast, and active state that does not rely on color alone.
- Run ./check_codebase.sh, then the repository aggregate gate; record unavailable browser evidence accurately.

## Assumptions

- Existing backend capabilities and frontend clients remain authoritative for page admission.
- Authentication already reaches the application and remains outside this plan.
- A future backend capability becomes visible only after its frontend integration checklist is complete.
