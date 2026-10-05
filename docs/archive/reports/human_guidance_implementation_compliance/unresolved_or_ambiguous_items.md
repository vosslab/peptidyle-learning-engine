# Unresolved or ambiguous items

The 2026-10-03 audit's open checklist rows are listed below. Positions are the 0-based generator
positions in the
[checklist](../../../active_plans/audits/human_guidance_implementation_checklist.md).
No choice here authorizes new product behavior.

## Decisions from the interview

- The ongoing interview has also settled Profile visibility, Blueprint sorting,
  shared spreadsheet search and display modes, tooltips, list-item navigation,
  confirmation before discarding searches, and removal of Student collaboration.
  System-wide settings are deferred. The old questions below are historical audit
  readings, not a current list of unanswered questions. See
  [HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md](../../../active_plans/decisions/HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md)
  for decisions, reasons, expressed commitment, and remaining investigations.
- On 2026-10-04, Neil resolved former position 475: a Sysadmin vets the Instructor
  outside PLE before creating an Account with an email address. PLE sends
  the setup email. Deactivation and reactivation control later access.
- This is now a confirmed implementation gap, not an open product decision.
  `src/pages/instructor_accounts_page.tsx` calls `completeInstructorIdentityVetting`
  before `createInstructorAccount`, whose input requires `vettingDecisionId`.
  Remove the PLE vetting requirement and verify the creation-to-setup-email flow.
- The checklist and counts below refer to the pre-interview guidance. Reconcile
  their evidence with the settled decisions before claiming current compliance.

The documentation-pass report still names a Student Ribbon unlock. Current Human
Guidance unlocks only the complete Sysadmin Ribbon task layout.

## Unlocked design

- Position 484. The complete Sysadmin Ribbon task layout does not have a locked-in
  design yet. Reason: HG: no locked-in design. Neighboring Sysadmin bullets stay
  separate questions.

## Terminology and identity

- Position 40. All fields, identifiers, domain concepts, and terminology across
  PostgreSQL, Rust, TypeScript, JSON, the Terminology Contract, and Human Guidance
  are in alignment. Question: does alignment mean the Terminology Contract and its
  registered surfaces, or a proof that every column, field, key, and label matches?
  One reading treats `Human-facing identifiers` and
  `test_semantic_contract_registry_points_at_native_coverage` as that alignment.
  The other requires a whole-repository comparison Human Guidance does not bound.
- Position 538. An object without a public ID uses a UUID or a parent composite key.
  Question: are Theme and provided-avatar tokens objects that need UUID primary keys,
  or durable vocabulary keys that keep text identifiers? Revisions use a composite
  key and Attempts use a UUID. `theme_id` and `provided_avatar_id` are text keys.
  One reading migrates those keys to UUIDs. The other keeps the vocabulary text.
- Position 546. Use the simplest term that accurately describes the value. Question:
  do Id, Tuple, and Reference already name identity values, or must every remaining
  value be judged? `PublishedQuestionRevisionTuple` and
  `WebworkQuestionSourceBinding` use Tuple and Binding. One reading treats those
  rules as the simplest terms. The other requires a review Human Guidance does not
  list.

## Interface philosophy and avatars

- Position 154. The role badge is always in the upper left, just left of the logo.
  Question: does the later desktop top bar, which places the logo before the role
  badge, replace this sentence? One reading keeps the badge left of the logo at
  every width. The other follows the later top bar: logo, product name, then badge
  on laptop, tablet, and desktop, and logo then badge on phones with the product
  name omitted.
- Position 103. Design around what users need to find and do. Question: do the
  shipped task ribbons and record pages already do that, or is a separate usability
  study still required? `PRODUCT_TIER_ONE` places Courses, Questions, and
  Assessments in the Instructor top bar. One reading treats those surfaces as the
  design. The other treats the sentence as a whole-product outcome with no pass rule.
- Position 114. Choose one visual philosophy and carry it through the interface.
  Question: is the precision field console that philosophy, or is a separate choice
  still open? `docs/UI_DESIGN_GUIDE.md` names the precision field console, and
  `src/style.css` shares `--ple-radius-surface`. One reading treats that console as
  the choice carried by the shell and record lists. The other treats the entire
  interface as an acceptance Human Guidance has not locked.
- Positions 224 and 234. The same avatar sentence appears twice. Question: when
  another screen represents a user, must it show that user's private Profile image,
  or only the provided gallery avatar? `RibbonAccountAvatar` shows the signed-in
  Account's image. Instructor Accounts uses `providedAvatarId` for another Account
  and does not show a private Profile image. One reading shows the private image
  everywhere. The other keeps it on that user's own Profile.

## Blueprint discovery

- Position 279. Public Blueprint search should sort by relevant fields such as Stars,
  Watches, Adoptions, Students, and most recent edit. Question: must search add
  Stars, Watches, and most recent edit, or are name, adoptions, and students the
  relevant sorts? `PublicBlueprintSearchPage` sorts by those three.
  `BlueprintCourseSummaryView` has no stars, watches, or last-edit field. One reading
  adds the missing sorts. The other reads "such as" as examples and keeps the three
  shipped sorts.

## Question Library notes

These indented notes stay open. Restore of a saved Question Library query already
exists and does not by itself decide the "Should" sentence.

- Position 326. Should opening a result and returning preserve search and position?
  One reading requires that restore. The other keeps the sentence as an open design
  note. `takeQuestionLibraryReturnState` already restores one saved query and scroll
  position.
- Position 327. Hover preview and open in a new tab by default. One reading requires
  both. The other treats the sentence as an unsettled note. Links open in the same
  tab and have no hover preview.
- Position 328. Advanced Search considerations. One reading requires another search
  mode. The other treats the bullet as a heading for the notes below. The Library
  has one search box plus filters.
- Position 329. One shared search box, or separate simple and advanced forms? One
  reading keeps the single box in `LibraryPage`. The other asks for a separate form.
  Human Guidance says to consider both.
- Position 330. A minimal interface that shows options by priority. One reading asks
  for a further advanced form. The other treats the sentence as a design note. The
  initial Search page is one search box.
- Position 331. MovieLens as a tiered filter. One reading requires that layout. The
  other treats the link as a comparison note.
- Position 332. IMDb advanced search, without movie-poster Questions. One reading
  requires that form. The other treats the sentence as a comparison note. Results
  use a semantic list.
- Position 333. Google advanced search. One reading requires that form. The other
  treats the link as a comparison note.
- Position 334. PubMed advanced search. One reading requires that page. The other
  treats the link as a comparison note.
- Position 335. eBay advanced search. One reading requires that form. The other
  treats the link as a comparison note.
- Position 336. Should a Question ID include a preview image? One reading adds a
  poster. The other keeps the text ID. `CopyableQuestionId` shows the text ID.

## Student and Sysadmin

- Position 397. Guidance about the student interface. Question: does this require a
  Student guidance surface, or does it only introduce the Student rules that follow?
  The bullet names no Student-facing behavior. One reading treats it as a heading.
  The other would require a guidance surface Human Guidance does not describe.
- Position 469. The Sysadmin menu should make Accounts, Instructors, Courses, and
  system configuration easy to find. Question: does system configuration need its own
  destination now, or is the menu complete while installation-wide settings remain an
  open inventory? Instructor Accounts and Courses are reachable. No system-configuration
  destination exists. One reading adds that destination. The other waits for the
  inventory.
- Position 479. System-wide settings should have their own area. Question: which
  implemented installation-wide settings must Sysadmins view or change, and which
  boundary owns each? No system-settings destination exists. One reading requires a
  page for actual platform settings. The other requires no page until those settings
  are identified.
- Position 481. Rare installation and configuration tasks should remain available
  through secondary navigation. Question: does this require a secondary Ribbon row
  now, or does the Sysadmin home satisfy it while the complete layout stays unlocked?
  `PRODUCT_TIER_TWO` has no Sysadmin destinations. The Sysadmin home still links
  Disciplines, Library activity, and scoped roster support. One reading adds a
  secondary row. The other keeps reachability on the home page.

## Assessments

- Position 1028. Quizzes may use more restrictive Attempt and collaboration settings
  than Weekly Assignments. Question: does this require a distinct collaboration
  control, or only the more restrictive Quiz Attempt limit? One reading keeps the
  Quiz default of one Attempt. The other adds a collaboration setting Human Guidance
  does not name.
