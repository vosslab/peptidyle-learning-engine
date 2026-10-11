# Human Guidance deep code review

Status: complete for the captured 378-file change set against
`e67eacab4ceeafe655d5b2b7ca051b7acda7c8df` (September 28). Review date: September 30, 2026.
This report supersedes the earlier selective review and its coverage claims.

The changes contain sound architectural work, but I would correct the competing metadata
owners and SVG content loss before building on them. The clearest recurring design risk is
turning individual Human Guidance sentences into permanent implementation-specific gates.
There is also avoidable persistence complexity and inherited legacy-duration machinery.

## Scope and interpretation

**A statement in Human Guidance should provide direction, not automatically justify a large
implementation.** This review treats the document as accumulated human guidance that can contain
stale, ambiguous, or overly broad statements. A matching sentence is not sufficient justification
for a workflow, abstraction, table, or gate. Product need, ownership, failure behavior, and the cost
of maintaining the implementation determine the assessment.

- Reviewed all 378 changed-file entries through six independent area reviews and coordinator
  synthesis. Each entry has an individual assessment in the linked coverage log. Surrounding
  unchanged code was traced where needed, including native Question resolution and duration editing.
- Read source and diffs; ran no tests, builds, compliance gates, browser checks, or services in this
  deep pass. Existing test results mentioned in source documents are historical claims, not this
  review's verification evidence.
- Explicitly deferred work is outside acceptance scope. Missing email/login completion, deferred
  daemons/backends, and missing producers in ongoing work are not defects in this report.
- Distinguished active-semester Course access from between-semester retention. This is a code and
  design review, not a legal determination about FERPA compliance.
- Allowed UUIDs in hidden API routes and JSON. The navigation restriction concerns visible browser
  URLs and copyable links.
- Treated the project as pre-production. Recommended direct contract and base-schema corrections,
  with no migration or compatibility layer merely to preserve development data.
- Left implementation files unchanged. Audit artifacts and a short changelog entry are the only
  review-owned repository edits.

[Per-file review log](human_guidance_deep_code_review_2026_09_30_files.md) records good, bad,
refinement, and neutral assessments. [Coverage CSV](human_guidance_deep_code_review_2026_09_30_files.csv)
also records original and final source hashes plus baseline-diff hashes. These are review records,
not a proposed permanent compliance gate.

| Area | Changed files reviewed |
| --- | ---: |
| SQL and data access | 63 |
| Tests and maintainer gates | 88 |
| Components and pages | 65 |
| Documentation and manifests | 41 |
| Server, domain models, and object ingest | 53 |
| Frontend contracts, features, and navigation | 64 |
| Dependencies and container configuration | 4 |
| Total | 378 |

## Defects and design problems

### D1 High: successful native Question metadata edits can break subsequent reads

The bulk command in
[published_question_metadata_operations.sql](../../../schemas/base_schema/50_functions/published_question_metadata_operations.sql)
updates `question_title` and `question_description` independently of the immutable Question source.
However, `resolved_ple_question` in
[summaries.rs](../../../crates/server/src/question_library/summaries.rs) compares the source's embedded
title and description with the current database metadata and returns an error when they differ
(lines 121-126 at review). The enclosing list conversion propagates that error.

Trigger: successfully replace a native PLE Question's title or description, then resolve that Question
or a Library page containing it. The write succeeds while the subsequent read rejects the valid
state. This is a competing-authorities defect, not unfinished UI wiring.

Fix: make current Library metadata authoritative for mutable presentation fields and overlay those
fields when constructing the summary. Retain source checksum, backend, and Question-type validation
for immutable content. Do not create a new Question Revision or a compatibility reader to repair a
metadata edit. Preserve the bulk command's useful transaction and concurrency design described in G1.

### D2 High: accepted SVGs can lose instructional labels during conversion

[svg_question_image.rs](../../../crates/objects/src/image_validation/svg_question_image.rs)
creates an empty font database, parses the SVG, rasterizes it, and returns WebP bytes as successful
ingest. The original SVG is discarded by the shared ingest path. The installed renderer's text
resolution requires fonts from that database; ordinary SVG `text` elements have no usable font and
can disappear without making the whole conversion fail.

Trigger: upload an SVG containing ordinary text labels, such as a labeled pathway or molecular
diagram. Shapes can survive while labels vanish from the stored asset. This conclusion follows the
source and renderer implementation; no image conversion was executed during the review.

Fix: define a deliberately supported SVG subset. Supply controlled bundled fonts for supported text,
or reject unsupported text with a clear message. External resource loading should remain disabled.
Align the `raster-images` dependency feature with the actual resolver policy, which currently rejects
both embedded and external images. Avoid silently accepting content the converter cannot preserve.

### D3 Medium: Starred has a permanently incomplete collection contract

[question_stewardship.sql](../../../schemas/base_schema/50_functions/question_stewardship.sql)
limits `list_current_starred_questions()` to 250 records and returns only a truncation flag. The
[store contract](../../../crates/learning-data-access/src/question_star.rs) has no continuation token;
[starred_questions_page.tsx](../../../src/pages/starred_questions_page.tsx) only tells the user more
records exist. With 251 Stars, an older Star cannot be reached through this collection.

Fix the store/API contract with keyset pagination on the existing `starred_at` and Question-ID order,
then reuse the shared paging controls. Raising the cap would retain the same defect. Separately,
[my_questions_model.ts](../../../src/pages/my_questions_model.ts) always requests the first 50 rows,
although its existing backend already returns a cursor. Completing that page should reuse the
existing paging model; it does not require another backend abstraction.

### D4 Medium: Pool provenance saves overwrite concurrent edits silently

`save_question_pool_provenance` in
[question_pools.sql](../../../schemas/base_schema/50_functions/question_pools.sql) replaces authors,
license, and source without an expected Edit Number or a returned new Edit Number. Locking the Pool
row serializes writes but does not detect stale editor state.

Trigger: two Instructors load the same provenance; each saves a different change. The second save
silently replaces the first. Include this state in the owning metadata concurrency contract. Use a
separate provenance Edit Number only if an independent editing lifecycle actually warrants it.
Correct the initial schema directly; a history subsystem or compatibility migration is unnecessary.

### D5 Medium: Discipline-request fulfillment splits one transition across browser writes

[discipline_request.ts](../../../src/components/discipline_request.ts) creates the Discipline and then
resolves its request through separate calls. The
[page](../../../src/pages/content_disciplines_page.tsx) updates local state only after both complete.

Trigger: creation commits, but request resolution fails. The UI reports failure while the Discipline
exists and the request remains open; retrying starts by trying to create the same Discipline again.
Own fulfillment in one server/database transaction that returns the created Discipline and resolved
request. Reuse existing authorization and creation logic. A browser compensation workflow would add
complexity around the wrong ownership boundary.

### D6 Medium: broad guidance has become brittle permanent gates

This is the strongest example of the newly emphasized failure mode. The problem is the asserted
contract, not merely the number of tests.

- [ribbon_profile_menu_contract.mjs](../../../tests/playwright/ribbon_profile_menu_contract.mjs)
  recursively inventories CSS/TS source and constructs artificial DOM samples. Added assertions freeze
  exact corner radii, border widths, padding, shadows, and geometry under test names copied from HG.
  A profile-menu contract has become a broad style enforcement mechanism.
- `tests/playwright/fast_ui_route_composition.mjs` (removed by the current audit repairs) adds exact
  alignment and spacing assertions, including one-pixel alignment and fixed spacing thresholds.
  These can reject a legitimate redesign without establishing that a teaching workflow is usable.
  A concurrent addition made the pattern especially clear: "Similar pages should place similar
  actions in consistent locations" became a two-pixel comparison of Course and Library action
  insets, then a completed checklist row. Consistent action placement is useful guidance; that
  particular geometric equality does not follow from it.
- [test_ribbon_route_contract.mjs](../../../tests/test_ribbon_route_contract.mjs) requires an exact
  inventory of three upload source files and specific textarea markup. Source placement is not the
  authorization or usability invariant.
- `tests/test_blueprint_instructor_browse.py` (removed by the current audit repairs) and
  `tests/test_course_retention_ferpa.py` (removed by the current audit repairs) inspect SQL text with
  regex, substring, and statement-order assertions. Those checks do not prove returned visibility,
  retention, or access behavior and penalize equivalent SQL refactors.

Remove the source inventories and arbitrary visual snapshots from permanent gates. Keep focused
keyboard, focus, interaction, and meaningful authorization/contract coverage. Broad visual guidance
belongs in bounded visual review, with evidence describing the actual pages inspected. This
recommendation is to reduce unjustified gates, not create another mandatory validation framework.

## Sound design to preserve

### G1 Atomic bulk metadata editing

The bulk metadata command locks selected Questions in canonical ID order, validates authorization,
availability, and expected metadata Edit Numbers before writes, and applies changes atomically.
Per-Question title/description replacements and a shared classification/tag patch can coexist in
one deliberate atomic command. They do not inherently require separate endpoints. Fix the read-side
metadata ownership in D1 while preserving this transaction shape.

### G2 Presentation titles remain separate from immutable identities

[recognition_titles.sql](../../../schemas/base_schema/50_functions/recognition_titles.sql) and its
[Rust projection](../../../crates/learning-data-access/src/postgres/blueprint_course/recognition.rs)
resolve bounded sets of current titles without copying them into pinned Blueprint content. Client
validation rejects malformed, duplicate, or unexpected identities. Current names help recognition;
Revision tuples still identify the teaching content. This is an appropriate small read projection.

### G3 Shared image ingest has one owner

[image_validation.rs](../../../crates/objects/src/image_validation.rs) uses `PreparedQuestionImage`
to distinguish original from rewritten bytes. Draft upload callers store the validated output and
media type through this boundary. Preserve that design and correct its SVG acceptance policy in D2;
do not add caller-specific conversion patches.

### G4 Backend and privacy boundaries mostly remain focused

Finalization reuses the existing ObjectStore abstraction rather than introducing a parallel object
platform. Backend-owned interaction remains behind the adapter. Shared Question statistics use one
domain projection for small-cell suppression rather than scattered route-specific checks. The new
closed diagnostic event vocabulary reduces opportunities to log Student answers or arbitrary
private payloads. These are useful boundaries, although static review does not certify complete
privacy protection or operational correctness.

### G5 Shared UI and direct terminology changes are coherent

Record reordering retains canonical IDs and keyboard alternatives. Question inspection reuses
answer-free presentation instead of constructing another renderer. Weekly/Unit Review terminology
is cut over directly through contracts and copy, without a new synonym compatibility layer. Fixed
My Questions and Starred navigation paths respect the visible-URL distinction.

### G6 Local network access uses the existing topology

[Caddyfile](../../../containers/Caddyfile) retains HTTPS for the requested LAN/Tailscale access.
[compose.yaml](../../../containers/compose.yaml) passes request-host acceptance explicitly with an
empty default. This reuses the existing gateway/server boundary. No runtime certificate or network
acceptance is claimed by this review.

## Refinements and decisions

### R1 Simplify Pool provenance storage and settle editing authority

[question_pool.sql](../../../schemas/base_schema/20_tables/question_pool.sql) splits current license
and source into separate one-to-one tables even though the command saves, clears, reads, and
authorizes them together. A single provenance row plus ordered author rows expresses the observed
lifecycle with fewer joins, grants, policies, and replacement operations. Independent lifecycle
requirements could justify separation, but none is demonstrated by this implementation.

The save command also allows any active Instructor to replace any Pool's attribution. Global Pool
availability does not settle who may edit credit. Record whether collaborative editing or
steward/owner authority is intended, then encode that choice at the command boundary. This is an
unresolved product decision, not a claim of a demonstrated browser privilege exploit. Avoid retaining
unused author-account fields/indexes solely for a speculative future feature.

### R2 Remove inherited legacy-duration repair state through a coherent contract

[assessment_duration.ts](../../../src/assessment_duration.ts),
[assessment_template_settings_model.ts](../../../src/pages/assessment_template_settings_model.ts), and
the [policy editor](../../../src/pages/assessment_workspace/assessment_workspace_policies_page.tsx)
retain `legacyTimeLimitSeconds` and block unrelated saves until a non-minute duration is replaced.
This machinery predates the baseline; the changed presentation test explicitly reinforces the
"legacy durations" framing. It is inherited design debt, not a newly introduced compatibility layer.

The database/domain currently permit positive seconds while the editor accepts whole minutes.
Choose one base-override contract: if authoring is whole minutes, enforce that through the domain and
initial schema and rebuild development data; if sub-minute authoring is intended, provide an editor
that represents it. Keep effective accommodated timing separate where fractional-minute results are
valid. Do not carry special legacy repair state because development data once used a different shape.

### R3 Reuse existing small contracts rather than adding frameworks

- The Blueprint point-value editor uses a permissive regex while the existing point-value decoder
  enforces a maximum of 1,000,000,000. Reuse that decoder so the editor and save boundary agree.
- Question inspection dependencies are individually optional. Supplying a loader without an image
  resolver can turn an empty URL into the app root. Current callers were not shown to omit it;
  make the enabled inspection capability a coherent typed bundle rather than claiming a current
  user-facing outage.
- The selected-zone formatter emits an empty string when settings are absent. Initial loading is
  already gated, but a settings-load failure can allow the page to paint with blank dates. Show an
  explicit unavailable state; silently using the browser zone would break the chosen-zone policy.
- Discipline requester IDs should retain the existing `AccountId` domain type across the Rust
  store boundary instead of becoming unvalidated strings.
- Question-ID paste resolution awaits each lookup serially, up to 250 calls. Reuse or add a bounded
  bulk resolution operation if this workflow remains; preserve input order and per-ID results. No
  elapsed-time claim is made, and unbounded browser request fan-out is not the recommended repair.
- Recognition-title effects can repeat the same lookup when only scoring/content state changes.
  Key the existing resource by the normalized identity set; do not build a new global cache platform.
- Library grid placement uses a long negated-child selector. Give the existing page explicit regions
  instead of depending on every present child class. Star mutation failures also need a visible local
  error state rather than an unhandled promise from a void click handler.

### R4 Keep review evidence smaller and more truthful

The active HG checklist and changelog repeatedly mirror individual HG sentences. A checkmark attached
to a broad claim can hide that its cited evidence covers one fixture or SQL spelling. Keep evidence
scoped to what it establishes; organize changes by actual product outcome rather than another full
HG transcription.

`devel/human_guidance_checklist.py` reads live inputs from `docs/archive/audits/hg_checklist_parts`.
Those nine files are functioning generator inputs, so their edits do not by themselves prove
historical records were improperly rewritten. Their location is misleading: move active inputs to
an active owned location, or explicitly document their live role. Avoid duplicating another authority.

Several audit edits replace screenshot/source links with bare filenames. Repair the reference or
state that the capture was retired and retain its original locator. Removing a broken hyperlink
without retaining a usable locator weakens the evidence trail.

The 1,315-line saved-response SQL oracle now mixes response finalization, Blueprint ownership, Pool
forks, retention/access, and stewardship disclosure; its runner requires seven unrelated notices.
Separate proofs by independently meaningful behavior if they remain permanent. Keep their useful
invariants. Do not add runner machinery solely to achieve a line-count target.

## Proportionality judgments and exclusions

The narrow Discipline request table is defensible: Instructors need a handoff to Sysadmins who own
Discipline creation, and email is deferred. It supplies that handoff without a background workflow;
D5 concerns fulfillment ownership. A small shared-statistics projection is also proportionate and
its missing aggregate producer during ongoing work is not a defect. Neither feature is justified
merely because HG mentions it, but neither needs rejection merely because it is new.

The SMTP/retention-notification changes belong to explicitly deferred work. They are recorded in the
coverage log but excluded from readiness findings. This review does not demand their completion or
use them to judge unfinished login content. It likewise does not treat deferred iMathAS/H5P/AI work,
intermediate compile failures, or missing current screenshots as completed-product defects.

No new terminology compatibility layer was confirmed. The concrete legacy concern found here is
the inherited duration state in R2; valid domain time intervals and rejection tests for old wire
shapes are not themselves legacy preservation.

## Evidence boundary

The inventory is a captured working-tree review, not a frozen branch or commit of Grok's changes.
Five inventory files changed during review: the route-composition browser test, changelog, active
HG checklist, and its Instructor-UI and Question checklist inputs. Their additional action-placement
and backend-interaction edits were read separately and included in the final hashes. The changelog
hash also includes this review's short receipt. Other files in the inventory matched their captured
hashes at reconciliation. Review artifacts are outside that original count.

Coverage means each changed file received a substantive static assessment; it does not mean every
path was executed or every possible defect was excluded. Runtime correctness, performance, complete
security coverage, and any manager edits after the recorded hash check remain unverified. The
recommended next work is to fix the ownership/content-loss defects and remove brittle gates, then
make the bounded contract decisions above before expanding these foundations.
