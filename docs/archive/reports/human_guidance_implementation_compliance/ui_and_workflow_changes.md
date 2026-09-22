# Ui And Workflow Changes

## Current heading reconciliation

Current verbatim Human Guidance coverage is 998 bullets: 449 verified, 499 open
(490 owning), and 50 N/A, at SHA256
`e81d5bb0a63cfb7d3ca5f4f34287e5155dc9d20b0b91cb856fdbefa1ef6fa82b`.
The generated checklist owns occurrence status and current first-owner pointers. All nine part
gates, identity diff, and consistency pass for this snapshot. Unchanged scoring, timing,
Blueprint, terminal-Attempt, and bounded MATCH evidence is retained. Product compliance remains
unfinished; these gates establish inventory fidelity, not acceptance of the open requirements.

Part 01 now owns Product vocabulary and glossary, including its five topical subheadings;
new product definitions remain open absent independently accepted evidence. Part 03 owns Profile
avatar interface, Student avatars, and Instructor and Sysadmin Profile images. Account-creation
avatar persistence has a bounded source/SQL receipt, not deployed gallery/upload/cropping or
all-location acceptance. Part 06 owns the shared Content classification requirements; Part 07
owns Library metadata, Part 08 Course classification, and Part 09 Assessment classification.
The shared system requires exactly one Discipline per content object, optional narrower levels,
and a Subject associated with one or more Sysadmin-managed Disciplines. The prior single-parent
four-table foundation receipt does not satisfy this latest association shape or establish commands,
normalization, content attachments, hierarchical selection, or discovery. Those gaps remain open.
KISS/design constraints are audited N/A where not independently closable, but still bind reviews.

Accepted R-4 desktop/phone terminal receipts hide active navigation and visibly label three
no-response records Unanswered, incorrect `0 / 1`; the four exact MATCH pairs remain correct
`1 / 1`, total `1 / 4`. Native diagnostic `AZA01TD` / `R-5` follow-up saved/reloaded FIB, MA,
MULTI-FIB, NUM, and ORDER, then submitted the whole Attempt: four correct `1 / 1` responses,
deliberately partial-reordered ORDER incorrect `0 / 1`, total `4 / 5`. Practice-default permitted
correct answers are displayed separately from retained responses. The supplied ledgers and
`submitted-review-1280.png` / `submitted-review-390.png` under
`/private/tmp/ple-student-types-proof/` are bounded receipts, not all-eight-type, complete keyboard,
touch, or contrast acceptance. HOTSPOT and WeBWorK coverage remain open. Earlier topical
inventories/correction IDs and superseded contradictions below are historical provenance.

## Scope

This is a fresh topical implementation-audit inventory. It collects currently open
Human Guidance checklist records relevant to UI and workflow. The bullet text is copied verbatim
from the generated checklist, and its source location is recorded beside it. This report does not
establish exhaustive or disjoint topical coverage; the checklist remains the authority for each
record's status.

The authoritative exhaustive record is the
[generated checklist](../../../active_plans/audits/human_guidance_implementation_checklist.md).

## Evidence updates

- Question Library classification has accepted isolated actual-component/router/fake-client evidence
  for Question-only identity filters, progressive selector cascades, explicit cross-Discipline
  Subject selection, preserved text/Tags/detail return, stale-selector recovery, keyboard controls,
  and 390px overflow. A separate accepted recovery keeps malformed classification URLs local, sends
  no request until explicit clear, and preserves unrelated text, Tags, return token, path, and hash.
  Pool discovery has accepted isolated actual `LibraryPage` fixture-browser proof at desktop and
  mobile widths. No result is connected HTTP, deployed, authenticated-shell, or whole-Library
  acceptance; Pool text/Tags filtering remains
  unavailable, and no mixed Question/Pool ranking or shared cursor is required. Receipt:
  `/private/tmp/ple-library-ui-batch-evidence-receipt-20260916.md`.

- Public Blueprint Search return state has accepted isolated actual-component/router/fake-client
  evidence. Its single-use, session-bound document snapshot replays applied text, Promoted, and
  classification filters through fresh cursors before restoring activated-link focus and clamped
  scroll, while retaining separate unsent controls. The reachable pending-continuation navigation
  defect was corrected and independently accepted. This does not add sorting, Tags, richer result
  metadata, live `8147`, connected HTTP/authorization, or a checklist closure. Receipt:
  `/private/tmp/ple-library-ui-batch-evidence-receipt-20260916.md`.

- Public Blueprint classification search is implemented locally without a checklist closure.
  `PublicBlueprintSearchPage` now submits ordinary text, Promoted, and optional shared
  Discipline -> Subject -> Topic -> Subtopic identities as one applied snapshot; its optional
  cross-Discipline Subject checkbox is explicit and hierarchy parent changes clear descendants.
  The server and cursor bind/validate the same filters. Root Cargo session 31251, focused Rust
  session 56442, strict TypeScript plus 21 Node tests session 19414, full pytest session 39220
  (7,462 passed), and fresh PostgreSQL 17 actual-role rollback proof passed. The reviewed
  actual-component proof uses a fake read-only client, so live `8147` still provides no connected
  current-source HTTP/browser or real vocabulary-parent/authorization acceptance. This entry is
  only the Blueprint classification slice; later separate Question Library classification and
  Blueprint return-state evidence does not broaden it. Tags, sorting, and richer result metadata
  remain outside this slice. Receipt:
  `/private/tmp/ple-classification-search-pool-receipt-20260916.md`.

- Question Pool review metadata controls now fill their review fieldset while retaining visible
  required labels. An isolated actual `QuestionPoolCreateDialog` plus parent-host fixture passed
  at 1280px with ordered selection, Title/Description preservation after picker return, focused
  review heading, and no horizontal overflow. This is not a full authenticated `LibraryPage` mount
  and does not close I09. Receipt: `/private/tmp/ple-classification-search-pool-receipt-20260916.md`.

- Blueprint Promoted implementation is local source evidence, not a closed workflow. The Public
  Blueprint Search can submit `promotedOnly=true`, retain that applied filter for retry and
  pagination, and clear it with the search. The server adds cursor-bound discovery and a
  Sysadmin-only metadata-ETag/CAS promotion boundary. Canonical PostgreSQL 17 bootstrap/install
  as `ple_migrator` and the isolated actual-role SQL proof passed; root Cargo session 60804,
  stricter TypeScript plus 11 Blueprint-client Node tests session 65918, and pytest session 36484
  passed. The proof container was removed. Deployed HTTP/browser integration remains unverified
  because live `8147` predates the source, so neither Promoted checklist row is closed.

- Current bounded UI evidence: `/private/tmp/ple-compact-student-navigation.md` and
  `/private/tmp/ple-compact-navigation-independent-review.md` accept five compact Attempt-navigation
  rows, with current/saved cues, first/last/range/ellipsis pagination, width adaptation, compact
  orientation, and theme-aware PLE styling. Actual four-Question desktop/phone Save/reload identity
  and the temporary styled 250-Question keyboard/200%-enlargement harness do not close global
  Student layout, all keyboard actions, before-start/review density, or light/dark contrast.
  `/private/tmp/ple-ui-bounded-acceptance.md` retains MATCH partial/reset, click Save/reload, and
  actual mouse-drag proof. The shared-bank desktop row is verified; the keyboard-parity row also has accepted Tab/Space/Enter swap/Clear/restore/Save proof; the remaining two MATCH rows remain open for their full current scope.

- Blueprint authoring and lifecycle reconciliation: the selected editor is now
  `BlueprintAssessmentContentEditor`, with separate Questions and Properties tasks. Accepted
  compiled-main, actual-loopback-HTTP evidence at
  `/private/tmp/ple-blueprint-owned-pool-artifacts.ay40bT/blueprint-properties-browser.json`
  opens one selected Assessment, keeps Questions and Properties drafts shared, exercises all six
  Student-feedback controls, and proves one ordinary PUT Save plus exact reload with an unchanged
  sibling. The whole Properties requirement remains open because scoring, attempt, and late-work
  activity controls were not exercised. Source confirms no Blueprint delivery dates, only
  `private`/`public`/`archived` availability, owner-only Private visibility, and Public-only
  adoption. Private development does not override the explicit rule that Private Blueprints cannot
  be adopted. Search Public Blueprint
  Courses is verified only for C47's submitted-name, Public-only workflow. Accepted bounded history proof at
  `/private/tmp/ple-blueprint-owned-pool-artifacts.sEJZUB` covers Public/Archived Instructor
  Revision and metadata facts, exact older read-only Revision inspection, Private-foreign and
  Student denials, and no `ple_data` mutation; it does not claim browser pagination/retry,
  local-draft preservation, login/TLS, or populated Student Work.

- C47 verifies the bounded Public Blueprint search workflow. Actual HTTP and compiled-main browser
  evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.C6RwpH/public-search-result.json`
  and `public-search-browser.json` cover literal name input, Public-only filtering before paging,
  a query-bound cursor, 51 matches through the real 50-row continuation, empty-result reset,
  existing detail opening, and Create Course Instance preselection beyond the first page. The
  read-only proof leaves `ple_data` unchanged. It does not create a Course Instance or establish
  ordinary login, TLS, full accessibility, manual chooser coverage, or the still-open Properties
  editor scope.

- Current Blueprint fork/comparison closure: accepted actual HTTP evidence at
  `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-lineage-pair-http-proof.json`
  verifies arbitrary visible sibling and transitive same-lineage current pairs, Private concealment,
  unrelated-pair denial, and Question-ID relationships through renamed/reordered/split content.
  Forks use fresh local Assessment and Pool identities while retaining exact Question Revision
  membership. The accepted 84-request target-local Apply proof at
  `/private/tmp/ple-blueprint-owned-pool-artifacts.vs0NCo/blueprint-local-id-apply-http-proof.json`
  verifies selected existing/new destinations, unchanged source and unselected target content, four
  CAS/authorization/Archived denials, and injected-fault rollback. Compiled-main browser evidence
  at `/private/tmp/ple-blueprint-owned-pool-artifacts.wVCQ4m/comparison-browser.json` covers
  current-pair review and selected Apply at 1280px and 390px. A separately accepted compiled-main,
  actual-loopback-HTTP receipt at
  `/private/tmp/ple-blueprint-owned-pool-artifacts.8PN6aH/comparison-browser.json` expands changed
  content at both widths and verifies DTO-backed titles, instructions, fixed Question IDs,
  Revisions, and points. No populated Student Work, login, TLS, full accessibility, or whole-C413
  claim is made.

- The Archived Blueprint read-only row is verified. Owner Save and rename lock the Blueprint and
  reject Archived state before replay, CAS, or no-op handling. Accepted actual HTTP proof recorded
  five `409` denials with unchanged Blueprint state, `200` owner/nonowner historical reads, `404`
  nonowner writes, and `200` restored Private/Public writes:
  `/private/tmp/ple-daughter-revision-notice-artifacts.JhV6aj/archived-blueprint-http-proof.json`.
  Explicit-include Archived browsing, forking, adoption, populated daughters, and concurrency are
  outside this receipt.

- Two Archived Blueprint discovery rows are verified. The default list excludes Archived Blueprints;
  explicit `includeArchived=true` returns the Archived record to owner and nonowner active vetted
  Instructors while retaining owner-only Private visibility. Accepted HTTP proof also records
  nonowner Archived `200`, Private `404`, Student list/detail `404`, and invalid query `400`.
  Accepted compiled-main browser proof covers default off, Include Archived, a read-only Archived
  detail, return to off, eight GETs, and zero writes. Artifacts:
  `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-http-proof.json` and
  `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-browser-proof.json`.
  Fixture sessions and privileged Published-Question seed data do not establish login, TLS,
  pagination, concurrency, publisher-caller preservation, forking, adoption, or a broader Course
  workflow.

- Six Course-update rows are verified by the accepted Course-summary workflow. The lazy authorized
  summary provides changed, matching, removed-source, Type-mismatch, and automatically-added
  adopted-Assessment rows, excluding direct local Assessments. Accepted actual-server and
  compiled-main proof covered 1280 by 900 and 390 by 844 lazy open/reopen behavior, per-read
  summary coherence, Course-to-detail review, zero POST on Cancel, exact source Revision 2 plus
  daughter Edit CAS on Apply, and a refreshed matching row. Student/unrelated reads were `404
  no-store`; the privileged-availability fixture concealed a private parent from another
  Instructor; Archived review remained available and new adoption was denied. Artifact:
  `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`. The earlier per-Assessment receipt
  separately establishes exact pins, stale/no-op/invalid-Released cases, dates, status, origin,
  and one populated Assessment Attempt hash: `/private/tmp/ple-daughter-revision-notice-artifacts.kE8MnT`.
  Neither receipt covers direct Assessments or all Student Work. No persisted offer, receipt,
  comparison baseline, new update table, or whole-Course lifecycle completion is claimed.

- Two bounded older-Blueprint-Revision indication rows are verified. The existing authorized Course
  load returns the origin's adopted and current Revision numbers; the strict browser decoder and
  Course page render both values and a visible stale notice only when the current Revision is newer.
  Accepted independent actual-server/exact-main proof covered empty, current, newer, and explicit
  synthetic Private-origin states. The newer capture was visually inspected; unauthorized Student
  and unrelated-Instructor reads were nonenumerating `404 no-store`, with no extra Blueprint fetch,
  write, or browser error. Original adoption pin, Assessment, and entries remained unchanged; Work
  tables were empty, so no populated-Student-Work claim follows. Artifact:
  `/private/tmp/ple-daughter-revision-notice-artifacts.u1qUyY`. This read-only indication does not
  close Blueprint update offer, review, approval, or apply work.

- C7/C424 close the two exact Instructor Course creation alternatives and their creation-only
  composite. `CourseInstanceCreationSource` and the
  browser decoder use one strict Empty-or-exact-Adopted wire; the Instructor Course form defaults
  to Empty and activates Blueprint discovery only for adoption. Accepted private actual HTTP and
  exact-main browser proof created, persisted, and displayed an Empty Course with zero Blueprint-
  list requests, `no-store` responses, and Student creation denial. A separate Public Blueprint
  exact-Revision adoption then produced a visible daughter Course and Unreleased Practice
  Assessment with one fixed Question Revision, finite Attempt limit, and dates unset. A follow-up
  actual-main/HTTP proof used the Empty Course's **Create Assessment** action to create an
  Unreleased direct-origin Practice Assessment with all delivery dates unset, then saw it on the
  Course list. A further accepted actual-main/HTTP run added and saved one available Published
  Question's exact Revision and point pin, saved a valid Due/time limit through Properties, passed
  release readiness, and released the direct Assessment. Accepted real roster import/Student claim
  and isolated actual-server/exact-main browser proof then showed the Released direct Assessment
  and its pre-start title, Practice Type, one Question/point, one-hour limit, and explicit zero-
  previous-Attempts state to its member; an Unreleased sibling was omitted and an outsider denied.
  The composite Course definition, Pool copying, other backend delivery/grading, and complete
  teaching lifecycle remain open.

- A separate isolated actual-server/native fixed-Question Attempt proof published the checked-in
  Genetics PKU JSON through Draft authoring, added its exact Available Revision to a direct
  Practice Assessment through the actual-main Instructor page, and enrolled the Student by real
  roster claim. Student HTTP Start issued the native four-choice Question; the selected opaque PKU
  response saved, restored after a second read, and whole-submitted. Submitted history disclosed
  a graded 1/1 result; a distinct second unlimited Practice Attempt started despite that perfect
  score. The history reader first omitted that saved response because it supplied the presentation
  checksum as `\\x`-prefixed bytea text instead of bare hex; a one-line
  `assessment_attempt_history.sql` encoding correction passed the same fresh-schema run and
  restored the readable PKU response. Accepted actual-main Student browser proof then reopened
  the Assessment before a second Start and showed all six pre-start facts with real Attempt 1
  Submitted history, the Coursework/"Before you start"/Attempt history Ribbon language, and a
  clicked summary with separately visible recorded response and 1/1 score. This is one native
  fixed-Question path, not a Pool case, every Student viewport, another backend, or a Regular
  Assignment default proof.

- C57 closes the three Question Library Search-landing and return-state rows. `LibraryPage` begins
  with only the Search entry, then starts the result workflow after input. Its session-bound,
  single-use in-document return snapshot restored query, filter, 80 loaded rows, and virtual-list
  position through visible return and browser Back. Accepted one-time compiled-browser evidence also
  confirmed idle no-fetch and changed-session isolation. The temporary harness and screenshots were
  removed after acceptance; this is not connected HTTP evidence.

- C58 closes seven advanced-search rows with accepted private PostgreSQL 17 and actual-server HTTP
  evidence. One production Store/route fixture exercised ordinary words, quoted phrases, minus
  exclusion, all five PLE field tags, active-vetted-Instructor access, anonymous and Student
  concealment, and `no-store`. The bounded 69-Question fixture does not establish expert usability
  or performance for a very large production library, so that separate row remains open.

- C60 closes all eight Browse rows with accepted routed-component, actual-server grouping, and
  private full-app browser evidence.
  Browse now has distinct grouped subject/topic/tag/Question Type navigation, full-authorized-
  snapshot counts, exact Browse-to-Search filter transfer, and the shared Question result path.
  The exact-main app authenticated against the actual server and traversed overview, Biology,
  Enzymes, and focused Search. Root manager visual review accepted the production-styled 1280 by 800 overview,
  narrowed rows, and Search screenshots. Test asset transport is not deployment-gateway or WASM-
  runtime evidence.

- C61 closes its sixth and final G-A4-17 row. Accepted private PostgreSQL 17, actual-server,
  and exact-main browser evidence returned two upcoming Assessments across the Instructor's two
  Courses, excluded an outsider Course, preserved anonymous/Student concealment, and matched each
  visible Due value to the actual HTTP instant in the returned Account zone. Follow-up exact-main
  browser evidence retained readable Chicago Account-zone local Due values in a three-row Course
  list at 1280px and 720px while the browser used Los Angeles time. A second owned Course then
  showed three mixed Released/Unreleased rows with distinct Due values matched to its actual
  200/`no-store` Assessment-list response; the Course heading qualified each row. Root manager
  visual review and independent artifact review accepted this bounded scanning behavior. The
  broader spreadsheet-like Course/Assessment collection-density judgment remains open under C44.
  Empty/error states, release workflow, WASM runtime, deployment gateway, and connected Template
  delivery were not exercised.

- C62 closes all five Assessment-editor-shell rows. Accepted private actual-HTTP and exact-main
  browser evidence navigated the distinct Question and Properties tasks, added, moved, removed,
  re-added, saved, and reloaded two exact Questions, persisted one Properties instructions edit,
  rendered the grouped Properties layout in two desktop columns and one column at 720px, and saved
  and reloaded fixed-Question point values `2.5` and `1` without changing other Assessment fields.
  Cancel, Stay/Discard, and real concurrent-write reload/discard recovery passed. This does not
  claim score recalculation or Released-Assessment editing. Broad cross-list scanability remains
  open pending representative proof across multiple Course lists.

- C64 and C65 close both Assessment-randomization rows. Accepted authenticated Student HTTP
  evidence persisted authored and shuffled rules, observed a real Instructor save blocked behind
  the Student-start lock, issued a complete exact fixed-and-Pool vector, and retained immutable
  shuffled order on resume. Accepted exact-main browser evidence saved and reloaded **Randomize
  question order** through actual HTTP. PLE Question authoring and its strict codec own
  `randomizeChoices`; the source adapter compiles that declaration to `NativeChoiceOrder`, and the
  presentation builder applies the nonce-derived choice order. Assessment activity rules have no
  answer-choice override. This establishes ownership without claiming a runtime matrix of every
  native choice permutation.

- Accepted private actual-HTTP and exact-main evidence closes C63's visible-order and direct
  Search/Browse rows. The editor showed a numbered order, moved the Questions, saved, and reloaded
  the persisted result; separate direct links reached Search and Browse, browser Back restored the
  editor, and the unsaved-changes guard preserved Stay and required deliberate Discard. Exact-
  Revision inspection now resolves the authorized private source and checksum through the opaque
  WeBWorK adapter and hardened iframe. Private PostgreSQL 17/MinIO plus unchanged-renderer HTTP
  evidence returned 200 with hardened headers and concealed missing Revision, Student, and
  anonymous requests; exact-main browser evidence rendered the prompt and five choices, with all
  five Student Work counts remaining zero before and after in this isolated preview path. C63 stays
  open because the renderer JavaScript dereferences `window.frameElement.id` when the hardened
  sandbox has no same-origin frame element, before focus, popover, and parent telemetry. Do not
  loosen the sandbox or rewrite sibling renderer HTML; successful hardened-embed behavior and a
  state-preserving return remain unverified.

- C77--C78 now have accepted source, strict TypeScript, focused projection-test, and compiled
  SolidJS/mock-API browser evidence. A fresh PostgreSQL 17 run also exercised the actual landing
  Store for Regular Assignment plus scheduled, expired unfinished, active resumable,
  Attempt-limit-reached resumable, and late-work-refused resumable states. This closes the direct
  Course-page state and scannable-list rows without claiming a connected HTTP-server run. The
  actual access Store was exercised too, including Type Regular Assignment and active-Attempt
  resume. The pre-start Assessment page presents title, Type, Question count, points possible,
  time limit, previous Attempts, and the Type-specific action; accepted browser evidence covers a
  Quiz label, start action, and Question count, but not the full facts/history projection or a
  connected HTTP-server run, so that broader row stays open.

## Topical inventory

### Interface design -- General interface design

- Instructor and **Sysadmin** workflows should work well in a 1280 by 800 desktop browser viewport.
  - Source: `docs/HUMAN_GUIDANCE.md:163`

- Design around what users need to find and do.
  - Source: `docs/HUMAN_GUIDANCE.md:164`

- Important information should stand out from supporting information.
  - Source: `docs/HUMAN_GUIDANCE.md:165`

- Related information should be visually grouped and aligned.
  - Source: `docs/HUMAN_GUIDANCE.md:166`

- Similar pages should place similar controls in consistent locations.
  - Source: `docs/HUMAN_GUIDANCE.md:167`

- Primary actions should be easy to find and appear near the content or workflow they affect.
  - Source: `docs/HUMAN_GUIDANCE.md:168`

- Avoid scattering related actions across page headers, menus, navigation, and content areas.
  - Source: `docs/HUMAN_GUIDANCE.md:169`

- Optimize large collections for scanning, searching, filtering, and comparison.
  - Source: `docs/HUMAN_GUIDANCE.md:171`

- Show enough useful information at once to support comparison without excessive scrolling.
  - Source: `docs/HUMAN_GUIDANCE.md:172`

- Search and filters should help users quickly narrow large collections.
  - Source: `docs/HUMAN_GUIDANCE.md:173`

- Dense pages should remain easy to scan.
  - Source: `docs/HUMAN_GUIDANCE.md:174`

- Use spacing to separate meaningful groups rather than simply making pages spacious.
  - Source: `docs/HUMAN_GUIDANCE.md:175`

- Prefer alignment, typography, and dividers over unnecessary cards, boxes, borders, and nested containers.
  - Source: `docs/HUMAN_GUIDANCE.md:176`

- Keep the visual design compact, flat, information dense, and consistent across PLE.
  - Source: `docs/HUMAN_GUIDANCE.md:177`

- Dream big on the UI. Choose one visual philosophy and carry it through the entire interface.
  - Source: `docs/HUMAN_GUIDANCE.md:178`

- Use drag-and-drop where it makes reordering faster and more natural.
  - Source: `docs/HUMAN_GUIDANCE.md:179`

- Reordering must also have a precise keyboard-accessible method.
  - Source: `docs/HUMAN_GUIDANCE.md:180`

- Implement the themes as specified in `docs/BIOME_THEME_PALETTES.md`
  - Source: `docs/HUMAN_GUIDANCE.md:184`

- UUIDs should never appear in visible content, navigation URLs, or copyable links.
  - Source: `docs/HUMAN_GUIDANCE.md:182`

- Use [Atkinson Hyperlegible Mono](https://www.brailleinstitute.org/freefont/) for code and other monospace text.
  - Source: `docs/HUMAN_GUIDANCE.md:184`

- Question Backend-rendered content may use its own fonts when needed for correct display.
  - Source: `docs/HUMAN_GUIDANCE.md:186`

- Students should have no upload capabilities. Instructor-created content should use text boxes.
  - Source: `docs/HUMAN_GUIDANCE.md:187`

### Interface design -- User top bar interface

- Each Product Role has its own home dashboard and navigation.
  - Source: `docs/HUMAN_GUIDANCE.md:204`

- Profile appears at the far right as an icon-only avatar.
  - Source: `docs/HUMAN_GUIDANCE.md:207`

- Clicking the Profile avatar opens the Profile menu.
  - Source: `docs/HUMAN_GUIDANCE.md:208`

- The Profile menu contains Profile settings, account settings, and Sign Out.
  - Source: `docs/HUMAN_GUIDANCE.md:209`

- Sign Out belongs in the Profile menu rather than the main top bar.
  - Source: `docs/HUMAN_GUIDANCE.md:210`

- The Profile avatar uses a generic user avatar until the user selects another avatar.
  - Source: `docs/HUMAN_GUIDANCE.md:211`

- **Students** select avatars from a PLE-provided collection and cannot upload Profile images.
  - Source: `docs/HUMAN_GUIDANCE.md:212`

- Student avatar selection should be visual and playful, similar to choosing a LEGO avatar.
  - Source: `docs/HUMAN_GUIDANCE.md:213`

- **Instructors** and **Sysadmins** may select a provided avatar or add their own Profile image.
  - Source: `docs/HUMAN_GUIDANCE.md:214`

- The current avatar appears consistently anywhere PLE represents that user.
  - Source: `docs/HUMAN_GUIDANCE.md:215`

### Interface design -- Breadcrumbs interface

- All signed-in users have a permanent breadcrumb row below the top Ribbon.
  - Source: `docs/HUMAN_GUIDANCE.md:222`

- The breadcrumb row remains in the same location and keeps the same space as users navigate.
  - Source: `docs/HUMAN_GUIDANCE.md:223`

- Breadcrumbs show the path from the user's home dashboard to the current page.
  - Source: `docs/HUMAN_GUIDANCE.md:224`

- Each breadcrumb level links back to its corresponding page.
  - Source: `docs/HUMAN_GUIDANCE.md:225`

### Interface design -- Instructor interface

- The Instructor interface should make frequent teaching tasks fast and easy to find.
  - Source: `docs/HUMAN_GUIDANCE.md:233`

- The Instructor menu has **Courses**, **Questions**, and **Assessments** in one dense top bar.
  - Source: `docs/HUMAN_GUIDANCE.md:234`

- All required ribbon choices remain visible even when their collection is empty.
  - Source: `docs/HUMAN_GUIDANCE.md:236`

- All required Instructor ribbon choices remain visible even when their target page is not implemented or complete.
  - Source: `docs/HUMAN_GUIDANCE.md:237`

- Empty collection pages should explain what the collection is for and provide an obvious action to create or add the first item when the user can do so.
  - Source: `docs/HUMAN_GUIDANCE.md:239`

- Similar pages should place similar actions in consistent locations.
  - Source: `docs/HUMAN_GUIDANCE.md:240`

- Instructor pages should be composed around the teaching task rather than collections of padded components.
  - Source: `docs/HUMAN_GUIDANCE.md:241`

- Instructor Course and Assessment lists should be dense and easy to scan, more like a spreadsheet than cards.
  - Source: `docs/HUMAN_GUIDANCE.md:242`

- Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.
  - Source: `docs/HUMAN_GUIDANCE.md:243`
  - Implementation: `AssessmentWorkspaceStudentViewPage`, `assessment_student_view_router`, `PostgresInstructorStudentViewStore`, and `load_instructor_student_view_question_source` now form the server-authorized answer-free, no-write Assessment projection.
  - Accepted evidence: fresh PostgreSQL exercised the real Store through API roles with a nonempty Ready Asset rendition, read-only SQLSTATE `25006`, and zero Student-state writes. Independently reviewed Chromium component evidence covered native and WeBWorK rendering, navigation, disabled controls, stale and error recovery, and no mutation requests using mock transport.
  - Remaining gap: the Chromium evidence was not connected or live-stack acceptance; the unchanged full server compile remains blocked in the AWS dependency graph; and production iMathAS Student View integration remains deferred outside the pilot. Independent final server source review passed, but it does not establish runtime behavior.

- The **Courses** ribbon must include: My Blueprint Courses, My Active Courses, My Inactive Courses, Search Public Blueprint Courses.
  - Source: `docs/HUMAN_GUIDANCE.md:247`

- The Course Editor should show the Course structure and its ordered Assessments without showing every Question at once.
  - Source: `docs/HUMAN_GUIDANCE.md:249`

- Selecting an Assessment in the Course Editor opens that Assessment for editing.
  - Source: `docs/HUMAN_GUIDANCE.md:250`

- Assessment content and Assessment properties should remain separate editing tasks.
  - Source: `docs/HUMAN_GUIDANCE.md:251`

- My Active Courses and My Inactive Courses should both be available from the Courses area.
  - Source: `docs/HUMAN_GUIDANCE.md:252`

- **Search Public Blueprint Courses** helps Instructors find a Blueprint Course they already have in mind.
  - Source: `docs/HUMAN_GUIDANCE.md:257`

- Public Blueprint Course search should support quickly narrowing a large collection.
  - Source: `docs/HUMAN_GUIDANCE.md:258`

- Blueprint Course editing should follow Course Editor -> Blueprint Assessment Editor.
  - Source: `docs/HUMAN_GUIDANCE.md:260`

- Selecting a Blueprint Assessment in the Course Editor opens the editor for that Blueprint Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:262`

- Only the selected Blueprint Assessment's Questions should appear in its editor.
  - Source: `docs/HUMAN_GUIDANCE.md:263`

- **Blueprint Assessment Question Editor**: Selects, adds, removes, and orders Questions in a Blueprint Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:264`

- **Blueprint Assessment Properties Editor**: Controls scoring, attempts, late work, and what **Students** can see.
  - Source: `docs/HUMAN_GUIDANCE.md:265`

- Blueprint Courses follow the lifecycle **Private -> Public -> Archived**.
  - Source: `docs/HUMAN_GUIDANCE.md:267`

- New and forked Blueprint Courses start **Private**.
  - Source: `docs/HUMAN_GUIDANCE.md:268`

- Private Blueprint Courses are visible only to their owner.
  - Source: `docs/HUMAN_GUIDANCE.md:269`

- Instructors may develop and use Private Blueprint Courses without publishing them.
  - Source: `docs/HUMAN_GUIDANCE.md:270`

- Making a Blueprint Course **Public** adds it to the shared Blueprint Course collection.
  - Source: `docs/HUMAN_GUIDANCE.md:271`

- A Public Blueprint Course with no adoptions may return to **Private**.
  - Source: `docs/HUMAN_GUIDANCE.md:272`

- A Public Blueprint Course with one or more adoptions remains **Public**.
  - Source: `docs/HUMAN_GUIDANCE.md:273`

- Instructors may fork a Public Blueprint Course to continue development privately.
  - Source: `docs/HUMAN_GUIDANCE.md:275`

- Blueprint Courses do not have a separate Draft state.
  - Source: `docs/HUMAN_GUIDANCE.md:276`

- **My Active Courses** should emphasize Course Instances the Instructor is currently teaching.
  - Source: `docs/HUMAN_GUIDANCE.md:280`

- Active Course Instances should make upcoming Assessments and important course activity easy to find.
  - Source: `docs/HUMAN_GUIDANCE.md:281`

- **My Inactive Courses** should keep past Course Instances available without competing with active Course Instances.
  - Source: `docs/HUMAN_GUIDANCE.md:282`

- Creating a Course Instance from a Blueprint Course preserves its Assessments, Questions, pools, and settings.
  - Source: `docs/HUMAN_GUIDANCE.md:283`

- Assessments created from a Blueprint Course start unreleased with dates unset.
  - Source: `docs/HUMAN_GUIDANCE.md:284`

- A Course Instance represents one teaching period and remains Active for at most six months from
  creation.
  - Source: `docs/HUMAN_GUIDANCE.md:285`

- Course banners use a 5:1 aspect ratio.
  - Source: `docs/HUMAN_GUIDANCE.md:287`

- 1280 by 256 pixels is the recommended Course banner authoring size.
  - Source: `docs/HUMAN_GUIDANCE.md:288`

- Higher-resolution 5:1 Course banner images are supported.
  - Source: `docs/HUMAN_GUIDANCE.md:289`

- PLE responsively scales Course banners while preserving their aspect ratio.
  - Source: `docs/HUMAN_GUIDANCE.md:290`

- Course banners appear as small centered banners rather than full-width page heroes.
  - Source: `docs/HUMAN_GUIDANCE.md:291`

- Course Instance Assessments have two editors:
  - Source: `docs/HUMAN_GUIDANCE.md:293`

- **Assessment Question Editor**: Selects, adds, removes, and orders Questions in an Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:294`

- **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and what **Students** can see.
  - Source: `docs/HUMAN_GUIDANCE.md:295`

- The **Questions** ribbon must include: My Questions, My Draft Questions, Starred, Watched, Search Question Library, Browse Question Library.
  - Source: `docs/HUMAN_GUIDANCE.md:299`

- **My Questions** should make the Instructor's Published Questions easy to find and manage.
  - Source: `docs/HUMAN_GUIDANCE.md:300`

- **Starred** should provide a quick personal collection of Questions the Instructor wants to keep handy.
  - Source: `docs/HUMAN_GUIDANCE.md:302`

- **Watched** should help Instructors follow Questions where changes or activity matter to them.
  - Source: `docs/HUMAN_GUIDANCE.md:303`

- Search syntax should help expert users quickly narrow a very large Question Library.
  - Source: `docs/HUMAN_GUIDANCE.md:331`

- The **Assessments** ribbon must include: Assessments Due Soon, My Assessment Templates.
  - Source: `docs/HUMAN_GUIDANCE.md:344`

- Assessment lists should make Course, release status, due date, and other important state easy to scan.
  - Source: `docs/HUMAN_GUIDANCE.md:346`

- **My Assessment Templates** should emphasize reusable Assessment design rather than Course activity.
  - Source: `docs/HUMAN_GUIDANCE.md:347`

- **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and other Assessment settings.
  - Source: `docs/HUMAN_GUIDANCE.md:350`
  - Current implementation: the named Properties Editor controls dates, instructions, Attempt/time limits, late work, order, and disclosure.
  - Remaining gap: fixed-Question point values are not editable in Assessment Properties.

- Instructors should be able to inspect a Question before adding it to an Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:354`

- Instructors can randomize Question order for an Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:356`

- Answer-choice randomization belongs to the Question, not the Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:357`

- Danger Zone contains **Assessment Unrelease**, **Archive Published Question**, and **Archive Blueprint Course**.
  - Source: `docs/HUMAN_GUIDANCE.md:363`
  - Current implementation: Assessment Unrelease and Archive Blueprint Course have Instructor controls; Archive Published Question has a browser API but no current Danger Zone interface.
  - Remaining gap: expose the Published Question archive workflow rather than treating its transport contract as a usable action.

- Archive actions should explain the effect on shared availability and require a clear confirmation.
  - Source: `docs/HUMAN_GUIDANCE.md:367`
  - Current implementation: Archive Blueprint Course explains removal from new selection and requires its long name.
  - Remaining gap: no current Archive Published Question interface provides the corresponding availability explanation and confirmation.

### Interface design -- Student interface

- **Coursework** is the Student-facing collective term for Regular Assignments, Practice Question
  Assignments, Bonus Assignments, Quizzes, and Exams.
  - Source: `docs/HUMAN_GUIDANCE.md:373`

- Student-facing interfaces should use the specific Assessment Type when referring to an individual item rather than calling it an Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:375`

- Coursework lists may provide filters for **Regular Assignments**, **Practice Question Assignments**,
  **Bonus Assignments**, **Quizzes**, and **Exams**.
  - Source: `docs/HUMAN_GUIDANCE.md:377`

- Each Coursework item should clearly show its Assessment Type using its label and Type icon.
  - Source: `docs/HUMAN_GUIDANCE.md:379`

- The Student menu is simpler than the Instructor menu.
  - Source: `docs/HUMAN_GUIDANCE.md:381`

- Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.
  - Source: `docs/HUMAN_GUIDANCE.md:382`

- Every Student browser action should be usable with the keyboard alone.
  - Source: `docs/HUMAN_GUIDANCE.md:383`

- Student navigation and pages should contain only Student interfaces and capabilities.
  - Source: `docs/HUMAN_GUIDANCE.md:385`

- The complete Student Ribbon task layout does not have a locked-in design yet.
  - Source: `docs/HUMAN_GUIDANCE.md:401`

### Interface design -- Sysadmin interface

- The Sysadmin menu should make Accounts, Instructors, Courses, and system configuration easy to find.
  - Source: `docs/HUMAN_GUIDANCE.md:406`

- Sysadmins should be able to find users quickly by name or email.
  - Source: `docs/HUMAN_GUIDANCE.md:407`

- Account lists should support searching, filtering, and scanning large numbers of users.
  - Source: `docs/HUMAN_GUIDANCE.md:408`

- User pages should clearly show role, account status, and other important administrative information.
  - Source: `docs/HUMAN_GUIDANCE.md:409`

- Sysadmins approve Instructors before they receive Instructor capabilities.
  - Source: `docs/HUMAN_GUIDANCE.md:411`

- Instructor approval status should be easy to find and change.
  - Source: `docs/HUMAN_GUIDANCE.md:412`

- Sysadmins should be able to find and inspect Courses across the installation.
  - Source: `docs/HUMAN_GUIDANCE.md:413`

- Course administration should show the Instructor and important Course status information.
  - Source: `docs/HUMAN_GUIDANCE.md:414`

- Sysadmins should manage Courses through Sysadmin interfaces and capabilities.
  - Source: `docs/HUMAN_GUIDANCE.md:415`

- System-wide settings should have their own area, separate from user and Course administration.
  - Source: `docs/HUMAN_GUIDANCE.md:416`

- High-consequence administrative actions should have a visually distinct area.
  - Source: `docs/HUMAN_GUIDANCE.md:419`

- Confirmation for destructive actions should clearly state what will happen.
  - Source: `docs/HUMAN_GUIDANCE.md:420`

- The complete Sysadmin Ribbon task layout does not have a locked-in design yet.
  - Source: `docs/HUMAN_GUIDANCE.md:421`

### Course specifications

- **Courses** organize reusable teaching content and its delivery to **Students**.
  - Source: `docs/HUMAN_GUIDANCE.md:685`

- PLE has two Course forms: **Blueprint Courses** and **Course Instances**.
  - Source: `docs/HUMAN_GUIDANCE.md:686`

- **Blueprint Courses** provide reusable course designs for creating Course Instances.
  - Source: `docs/HUMAN_GUIDANCE.md:687`

- A Course can have multiple co-**Instructors** with equal teaching authority.
  - Source: `docs/HUMAN_GUIDANCE.md:689`

- **Sysadmins** can create Courses, but **Instructors** teach them.
  - Source: `docs/HUMAN_GUIDANCE.md:690`

- Creating a Course Instance establishes its first Instructor membership but does not give that Instructor greater Course authority than later co-Instructors.
  - Source: `docs/HUMAN_GUIDANCE.md:692`

### Course specifications -- Blueprint Courses

- Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
  - Source: `docs/HUMAN_GUIDANCE.md:701`

- Blueprint Courses contain only **Published Questions** and published **Question Pools**.
  - Source: `docs/HUMAN_GUIDANCE.md:702`

- An **Instructor** may deliberately publish an existing Course Instance structure as a new Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:703`

- Blueprint Courses have three lifecycle states: **Private**, **Public**, and **Archived**.
  - Source: `docs/HUMAN_GUIDANCE.md:707`

- New Blueprint Courses and forks start Private.
  - Source: `docs/HUMAN_GUIDANCE.md:708`

- Private Blueprint Courses are visible only to their owning **Instructor**.
  - Source: `docs/HUMAN_GUIDANCE.md:709`

- Private Blueprint Courses cannot be adopted to create daughter **Course Instances**.
  - Source: `docs/HUMAN_GUIDANCE.md:710`

- Public Blueprint Courses can be adopted to create daughter Course Instances.
  - Source: `docs/HUMAN_GUIDANCE.md:712`

- Archived Blueprint Courses can be forked but not adopted.
  - Source: `docs/HUMAN_GUIDANCE.md:717`

- The owning **Instructor** can return an Archived Blueprint Course to Public before adopting it again.
  - Source: `docs/HUMAN_GUIDANCE.md:718`

- Other **Instructors** can fork an Archived Blueprint Course to create a new Private Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:719`

- **Instructors** can Star or Watch Public and Archived Blueprint Courses.
  - Source: `docs/HUMAN_GUIDANCE.md:735`

- A Star is a visible endorsement and helps **Instructors** save useful Blueprint Courses.
  - Source: `docs/HUMAN_GUIDANCE.md:736`

- Vetted **Instructors** can see who Starred a Blueprint Course and its Star count.
  - Source: `docs/HUMAN_GUIDANCE.md:737`

- Watching a Blueprint Course is private.
  - Source: `docs/HUMAN_GUIDANCE.md:738`

- Watchers are notified about new Blueprint Revisions and other important Blueprint changes.
  - Source: `docs/HUMAN_GUIDANCE.md:739`

- Forking or adopting a Blueprint Course does not automatically Star or Watch it.
  - Source: `docs/HUMAN_GUIDANCE.md:740`

- Stars and Watches belong to the Blueprint Course across all of its Revisions.
  - Source: `docs/HUMAN_GUIDANCE.md:741`

- It should be obvious when a Course Instance is using an older Blueprint Revision.
  - Source: `docs/HUMAN_GUIDANCE.md:749`

- Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Source: `docs/HUMAN_GUIDANCE.md:751`

- Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Source: `docs/HUMAN_GUIDANCE.md:752`

- An **Instructor** can fork a **Blueprint Course** to create a new independent Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:756`

- A fork records the source Blueprint Course and Blueprint Revision from which it was created.
  - Source: `docs/HUMAN_GUIDANCE.md:757`

- Forked Blueprint Courses develop independently and have their own Blueprint Revisions.
  - Source: `docs/HUMAN_GUIDANCE.md:758`

- Changes to a source Blueprint Course are never automatically applied to its forks.
  - Source: `docs/HUMAN_GUIDANCE.md:759`

- A fork should make newer changes from its source Blueprint Course easy to discover and review.
  - Source: `docs/HUMAN_GUIDANCE.md:760`

- An **Instructor** can selectively bring changes from a source Blueprint Course into their fork.
  - Source: `docs/HUMAN_GUIDANCE.md:761`

- An **Instructor** can create a **Blueprint Course Change Proposal** to propose changes to another Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:762`

- A Change Proposal shows added, removed, and changed Assessments and Question content.
  - Source: `docs/HUMAN_GUIDANCE.md:763`

- The receiving **Instructor** decides which proposed changes to accept.
  - Source: `docs/HUMAN_GUIDANCE.md:764`

- Accepted changes create a new Blueprint Revision of the receiving Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:765`

- Change Proposals never directly change daughter Course Instances.
  - Source: `docs/HUMAN_GUIDANCE.md:766`

- Daughter Course Instances receive accepted changes through the normal Blueprint update workflow.
  - Source: `docs/HUMAN_GUIDANCE.md:767`

- Blueprint Courses have a canonical JSON representation for comparison, import, export, and exchange.
  - Source: `docs/HUMAN_GUIDANCE.md:771`

- Canonical Blueprint JSON must contain enough information to fully recreate a Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:772`

- Importing exported Blueprint JSON should reproduce the same Blueprint Course content and structure.
  - Source: `docs/HUMAN_GUIDANCE.md:773`

- Blueprint JSON contains Blueprint metadata and an ordered list of Blueprint Assessments.
  - Source: `docs/HUMAN_GUIDANCE.md:774`

- Blueprint Assessments contain ordered **Published Questions** and published **Question Pools**.
  - Source: `docs/HUMAN_GUIDANCE.md:776`

- Blueprint Revisions can be compared through their canonical JSON representations.
  - Source: `docs/HUMAN_GUIDANCE.md:778`

- Blueprint Course Change Proposals use canonical JSON to identify changes between Blueprint Revisions.
  - Source: `docs/HUMAN_GUIDANCE.md:779`

- Canonical Blueprint JSON is the complete exchange format, not the primary persistence model.
  - Source: `docs/HUMAN_GUIDANCE.md:781`

### Course specifications -- Course Instances

- Course Instances have **Students**, deadlines, releases, and other delivery-specific settings.
  - Source: `docs/HUMAN_GUIDANCE.md:789`

- Course Instances contain only **Published Questions** and published **Question Pools**.
  - Source: `docs/HUMAN_GUIDANCE.md:790`

- Active Courses are current teaching Course Instances.
  - Source: `docs/HUMAN_GUIDANCE.md:792`

- Inactive Courses are past Course Instances and retain Course metadata, including after
  FERPA-sensitive Student data is removed.
  - Source: `docs/HUMAN_GUIDANCE.md:793`

- An **Instructor** may deliberately publish reusable Course Instance structure as a new **Blueprint Course**.
  - Source: `docs/HUMAN_GUIDANCE.md:795`

- Creating a Course Instance from a Blueprint Course copies its Assessments, Questions, Question Pools, and reusable settings.
  - Source: `docs/HUMAN_GUIDANCE.md:806`

- It should be obvious when a daughter Course Instance is using an older Blueprint Revision.
  - Source: `docs/HUMAN_GUIDANCE.md:810`

### Course specifications -- Course names

- Short names are for compact navigation and should stay under about 16 characters when practical.
  - Source: `docs/HUMAN_GUIDANCE.md:819`

Latest Student response distinction rewrite: live HG requires visually distinct current Question,
saved-response status and keyboard focus, plus response-effect labels distinguishing Save/Clear/
change from whole Coursework submission. Both rows are open; earlier navigation styling receipts
are retained as partial proof, not blanket acceptance of native response controls/actions.
