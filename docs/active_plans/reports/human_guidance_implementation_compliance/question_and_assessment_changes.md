# Question And Assessment Changes

## Scope

This is a fresh topical implementation-audit inventory. It collects currently open
Human Guidance checklist records relevant to questions and assessments. The bullet text is copied
verbatim from the generated checklist, and its source location is recorded beside it. This report
does not establish exhaustive or disjoint topical coverage; the checklist remains the authority
for each record's status.

The authoritative exhaustive record is the
[generated checklist](../../audits/human_guidance_implementation_checklist.md).

## Evidence updates

- C365/C367/C893 now have accepted bounded backend evidence, not a Question Library workflow
  closure. `PublishedQuestionSharedMetadata` is a closed DTO with generated 1,000-item bounds;
  the current-metadata read and typed update path validate canonical-ID HMAC before Store access,
  use no-store responses, and retain only the all-or-none current-state contract. Fresh PostgreSQL
  17 SQL/API proof and an independent rerun covered read/write/read, stale denial with no
  writes, intentional clear, concealed unauthorized/unvetted/archived/missing/duplicate cases,
  unchanged Revision count, and private-helper denial. New native publications initialize only
  canonical `PLE authoring`/`Pilot` tags; WebWork is empty. The explicit SQL/API initial-tag proof
  rejects null tag elements without database/publication/object side effects, and a cleared empty
  list survives a successor Revision. The narrow PostgreSQL LDA library check passed. Temporary
  compiled Chromium component and strict-client evidence accepted sorted selection/Edit Numbers,
  closed replace/clear, virtualization, stale/ambiguous refresh with no automatic second write,
  generic denial, filter clearing, and zero critical/serious axe findings; it used mock/injected
  transport, not a connected server, and the proof was removed. Connected HTTP and discovery
  projection remain unverified because the full server build is blocked by the existing AWS Smithy
  incompatibility. C58 has accepted actual-source parser evidence for ordinary words, quotes, minus,
  PLE fields, literal unknown tokens, empty fields matching nothing, and exact-ID-plus-filter behavior, but its connected
  HTTP/search projection remains unverified. C366/C368 and C337/C340 therefore remain open.

- C59 now provides native Search tips beside the ordinary search input. An accepted corrected desktop
  component proof showed words, quotes, minus, PLE fields, and examples without obscuring normal
  filters or bulk controls; the full `./check_codebase.sh` gate passed. Large-library runtime
  narrowing remains open because C58's connected HTTP/search projection is still blocked.

- C523 has accepted independent PostgreSQL 17 actual-API receipts for the three Course Instance
  Assessment due/late defaults. Direct and reusable-content creation boundaries default late work
  to `reject`; an Attempt under that rule pins its immutable expiration to the effective Due date,
  while an accommodation remains authoritative. Post-Due start, save, and Student commit were
  denied, and worker finalization retained both accepted pre-Due responses. An entirely unsaved
  Attempt finalized with zero credit for unanswered Questions and no Question Backend evaluation.
  Explicit `accept` and `mark_late` overrides remain valid and use Closes rather than Due as their
  expiration boundary. Save/finalization serialization also preserved concurrent accepted work.
  This closes only the three default rows: it does not establish visible unanswered UI,
  cross-session resume, every backend behavior, or broad release validation. The ignored proof was
  removed after independent acceptance.

- C519--C521 have accepted narrow release-date validation receipts. The shared
  `assessment_release_issues` authority returns the five date issues: missing Due, Due less than
  24 hours ahead, Due after the Course Active limit, Available after Due, and Due after Closes.
  Every released-state writer uses the same hard gate; unchanged near/past-Due Assessments may be
  saved, while changed invalid values are rejected. The accepted fresh PostgreSQL 17 actual-API
  receipt covered authorization, rollback, date boundaries, correction, rerun, and successful
  release. The actual Assessment Properties component receipt covered all five actionable messages
  and correction/rerun/release. These receipts close only the narrow release-date rows below.
  The broader missing, invalid, or unreasonable-values row remains open: the five date issues do
  not establish validation beyond dates, including required-setting ranges and Question validity.

- C506 has accepted strict five-Type contract evidence. The Rust model, generated browser enum,
  PostgreSQL constraints, Blueprint create/view decoder, Course Instance creation, and Blueprint
  adoption all require the same five values without a fallback. Focused Blueprint client evidence
  passed 16/16 and create/model UI evidence passed 10/10. C508 source evidence shows that the
  current adopted-Quiz Properties fixture changes instructions and its time limit from 300 to 600
  seconds without accepting a Type field. Its ignored PostgreSQL acceptance test has not yet been
  rerun after that fixture update, so this is not a current runtime receipt. The effective settings
  implementation is named `effective_assessment_properties.rs`; its final review, formatting, 77
  domain tests, and targeted three-crate Cargo check passed. C524 subsequently established
  Type-aware disclosure defaults at the Rust, direct SQL, Blueprint, and curriculum-publication
  creation boundaries. These results still do not establish every appropriate Type default or the
  full pedagogical-purpose behavior, so the broad defaults row remains open.

- C510 has accepted its bounded Gradebook and Student highest-score selection receipts, not global
  closure. `read_assessment_gradebook_evidence` independently selects the highest
  grading-complete submitted Attempt by earned points; only when no score is established does it
  fall back to the latest Attempt for current progress or expiry. The private answer-free helper
  is consumed by the real Gradebook and Student APIs, without changing raw Assessment Attempt scoring. Accepted
  actual PostgreSQL 17 API evidence proved an earlier higher earned Attempt beats a later lower
  submitted Attempt, and later unfinished or pending work does not replace it; current Question
  points recalculated `8` to `16`; Bonus retained a zero possible denominator; and the latest
  unscored expired Attempt remained the fallback. LDA and browser decoding accept finite,
  nonnegative earned points above points possible and zero possible. The Student API keeps latest-
  Attempt progress separate, applies the selected Attempt's copied score-disclosure timing, and
  emits direct `assessmentScore`; its strict decoder rejects the retired `score` alias. The focused
  decoder/presentation lane passed 8/8, compiled M6 component evidence passed, and the focused
  PostgreSQL LDA library check passed in 5.72 seconds without warnings. Raw Attempt and history
  validators remain unchanged; the Student projection preserves its self-only current Account,
  current Course relationship, and ordinary retention fences. The full `server_core` compile
  remains unverified because the existing incompatible AWS Smithy dependency pair blocks that
  separate integration gate. C510 remains open for broader unverified behavior, including Practice
  extra-credit authoring; these accepted receipts do not establish Course totals.

- C511 remains open. Human Guidance permits Quiz and Exam settings that are more restrictive than
  a Regular Assignment; it does not mandate restrictive Type defaults or arbitrary thresholds.
  Available and Closes authoring is independently accepted in the Properties page: actual-component
  proof covers local timestamp round trips, incomplete-input no-write with linked error, clear to
  null, failure and retry, IANA time-zone display, keyboard use, and a 320-pixel viewport. This
  receipt makes no chronology or DST policy decision and adds no backend redesign. Collaboration
  policy remains undefined, so this does not claim C511 closure.

- C524 has accepted the bounded disclosure/defaults slice. The Rust initializer, direct SQL
  constructor, Blueprint default builder, and curriculum publisher default Practice correct answer
  to `after_submit`; Regular and Bonus default it to `never`; submitted response and per-item
  correctness default to `after_submit`; and Question Feedback and answer explanation remain
  independent. Actual-component proof switched the create dialog Type both ways while preserving
  title, entries, and selected-Type defaults. Accepted whole-submission proof navigated to the
  existing server-redacted summary while failures remained on the Attempt. A fresh PostgreSQL 17
  actual-API proof found zero history response-source rows before whole submission and one after;
  the native PLE summary preserved the disclosed correct answer. Its authorization matrix returned
  `owner=1`, `other Student=0`, `Instructor=0`, `archived=0`, and ended-membership ownership false.
  The reader reuses the existing exact Course, Student Record, current Account, active Student
  Membership, and retention checks; it adds no grant or helper authority. Temporary PostgreSQL and
  Chromium proofs were removed after independent acceptance.

- C524 does not close universal Practice answer disclosure. The public
  [ADAPT source snapshot](https://github.com/LibreTexts/adapt/tree/41e9b75b03960c9e6d6fd01d992ecdb15195f56b)
  and its matching
  [renderer snapshot](https://github.com/openwebwork/renderer/tree/c15474b0e380f63ce4d23c143a91b4696559315b)
  show one iframe/JWT integration, but do not identify the production renderer revision or any
  deployment patches; this is source inspection, not a live vulnerability or runtime proof. PLE
  instead constructs disclosure controls server-side in
  [protocol.rs](../../../../crates/adapters/webwork/src/http_renderer/protocol.rs) and projects
  only renderer HTML after checking that private JWT state is not reflected in
  [client.rs](../../../../crates/adapters/webwork/src/http_renderer/client.rs). Backend-owned
  answers still project as absent. The current sibling renderer makes `showCorrectAnswers` enable
  `showSolutions` and forces that request from hardened `ple_embed` to generic `static` HTML, whose
  template embeds session JWT state and omits the PLE bridge. PLE-managed general feedback follows
  a separate release policy, while decoding answer JWTs or renderer HTML would break the opaque
  backend boundary; no PLE-only workaround therefore meets C524. The narrow follow-on remains an
  opaque, transient, no-store answer document after the completed-Attempt policy decision. The user
  directed that the sibling renderer remain unchanged, so this gap stays open.

- C525 is open implementation work. The release cohort uses current Student Course relationships:
  pending invitations are excluded; accepted enrollment joins; ended or withdrawn relationships and
  deactivated Course access exit; Account deactivation preserves membership; never-started Work
  without a submission blocks release; and Course end is not completion. Existing
  `assessment_submission` evidence makes every Assessment Attempt complete on whole Student
  submission or expiry automatic submission, independently of score or correctness. Another
  Attempt follows the configured Attempt limit: unlimited remains unlimited after a perfect score,
  and Quiz and Exam permit exactly one Attempt. This needs implementation at the existing
  submission and cohort boundaries, without a completion snapshot, latch, new DAG, or bookkeeping
  machinery.

- C512 has accepted the canonical label, semantic color, theme-scope, and icon-name presentation.
  Genuine Free-package glyphs are bundled for all five Assessment Types: `pen-to-square`,
  `arrows-spin`, `star`, `circle-question`, and `file-signature`. Each rendered Type keeps its
  visible label alongside the glyph, and a nested theme fixture verified the scoped color cascade.

- C910 has accepted fresh-PG17 persistence evidence, not closure: an explicit `webworkPgml`
  Draft/binding retained its format, path, and checksum while author-managed general feedback
  produced two immutable Published Revisions. The SQL `RETURNING` ambiguity was corrected and
  independently reviewed; full TypeScript checking passed. Student HTTP feedback projection and
  release remain unverified because the server build is blocked by the current AWS Smithy
  dependency incompatibility.

- C839 has accepted temporary canonical-source evidence for 42 PGML sources (41 official
  biologyproblems-website sources plus HLA). Manifest registration, provenance and checksums,
  render/lint/whitelist, repeatable/reseeded variation, and representative grading passed. This
  does not publish them or reconcile the catalog. The redundant static source bulk has been
  removed, but C840--C841 remain open for ordinary publication and catalog reconciliation.

## Topical inventory

### Questions

- Questions are strictly and deterministically automated; grading does not require an **Instructor**.
  - Source: `docs/HUMAN_GUIDANCE.md:514`

### Questions -- Draft Questions

- Instructors may delete Draft Questions they no longer need.
  - Source: `docs/HUMAN_GUIDANCE.md:523`

- PLE may clean up abandoned Draft Questions after an appropriate warning and recovery period.
  - Source: `docs/HUMAN_GUIDANCE.md:524`

### Questions -- Native PLE JSON Questions

- Native JSON Questions are static, not algorithmic nor random, and receive no random seed.
  - Source: `docs/HUMAN_GUIDANCE.md:541`

- External URLs used by native JSON Questions are explicitly recorded and reviewable.
  - Source: `docs/HUMAN_GUIDANCE.md:543`

- Recorded external URLs include links, images, scripts, stylesheets, and other resources.
  - Source: `docs/HUMAN_GUIDANCE.md:544`

- Native JSON Questions may contain author-supplied JavaScript, including chemistry content using RDKit.
  - Source: `docs/HUMAN_GUIDANCE.md:548`

- Author-supplied JavaScript may provide client-side rendering or interaction without access to a random seed.
  - Source: `docs/HUMAN_GUIDANCE.md:549`

- Author-supplied JavaScript runs in an isolated browser environment.
  - Source: `docs/HUMAN_GUIDANCE.md:550`

- Author-supplied JavaScript is treated as untrusted content.
  - Source: `docs/HUMAN_GUIDANCE.md:551`

- Author-supplied JavaScript is isolated from PLE application state, credentials, and privileged browser context.
  - Source: `docs/HUMAN_GUIDANCE.md:552`

- Author-supplied JavaScript is limited to client-side rendering and interaction.
  - Source: `docs/HUMAN_GUIDANCE.md:553`

- Author-supplied JavaScript operates independently of PLE application APIs and privileged state.
  - Source: `docs/HUMAN_GUIDANCE.md:554`

- Native interactive Question Types such as HOTSPOT use PLE-owned interaction code.
  - Source: `docs/HUMAN_GUIDANCE.md:555`

- HOTSPOT content uses supported static assets such as images and SVG.
  - Source: `docs/HUMAN_GUIDANCE.md:556`

- Grading and correctness decisions remain server-owned and independent of author-supplied JavaScript.
  - Source: `docs/HUMAN_GUIDANCE.md:557`

- External JavaScript dependencies and CDN domains are explicitly recorded and reviewable.
  - Source: `docs/HUMAN_GUIDANCE.md:558`

- Approved external dependencies may initially load from recorded CDN sources.
  - Source: `docs/HUMAN_GUIDANCE.md:559`

- Supported external dependencies should eventually become PLE-owned and served locally.
  - Source: `docs/HUMAN_GUIDANCE.md:560`

### Questions -- Question Backends

- WeBWorK, iMathAS, and H5P are PLE-managed Question Backends.
  - Source: `docs/HUMAN_GUIDANCE.md:564`

- iMathAS and H5P are supported secondary Question Backends.
  - Source: `docs/HUMAN_GUIDANCE.md:566`

- Question Backends own rendering, interaction, response, grading, feedback, and backend-specific state.
  - Source: `docs/HUMAN_GUIDANCE.md:571`

- PLE owns authorization, Question ID, revisions, persistence, lifecycle, and stored outcomes.
  - Source: `docs/HUMAN_GUIDANCE.md:572`

- PLE uses the same basic interface for every Question Backend, each backend handles its own internal details.
  - Source: `docs/HUMAN_GUIDANCE.md:573`

- Each Question Backend adapter retains its backend-specific interaction knowledge.
  - Source: `docs/HUMAN_GUIDANCE.md:574`

- WeBWorK owns PG/PGML rendering, controls, answer evaluators, partial credit, and feedback.
  - Source: `docs/HUMAN_GUIDANCE.md:576`

- H5P owns its runtime, interactions, state, and scoring.
  - Source: `docs/HUMAN_GUIDANCE.md:577`

- iMathAS owns its rendering and evaluation.
  - Source: `docs/HUMAN_GUIDANCE.md:578`

- Question Backends may support more complex interactions without requiring PLE to implement those interactions.
  - Source: `docs/HUMAN_GUIDANCE.md:579`

- A Question Backend returns an immutable credit fraction for each complete response it evaluates.
  - Source: `docs/HUMAN_GUIDANCE.md:580`

- PLE stores the immutable credit fraction as the grading outcome.
  - Source: `docs/HUMAN_GUIDANCE.md:581`

- When PLE requests a grading outcome, the Question Backend returns it without a deferred grading
  state.
  - Source: `docs/HUMAN_GUIDANCE.md:582`

- Assessment scores are calculated from stored credit fractions and current Question point values.
  - Source: `docs/HUMAN_GUIDANCE.md:584`

- Changing Question point values recalculates scores without another Question Backend interaction.
  - Source: `docs/HUMAN_GUIDANCE.md:585`

- When parameterized WeBWorK source exists, prefer it to importing static variants.
  - Source: `docs/HUMAN_GUIDANCE.md:586`

### Questions -- Question Pools

- A **Question Pool** is a set of interchangeable **Published Questions** from which PLE selects for a Student.
  - Source: `docs/HUMAN_GUIDANCE.md:590`

- Question Pools are primarily designed for static Question variations.
  - Source: `docs/HUMAN_GUIDANCE.md:591`

- Pool contents should represent reasonably interchangeable assessments of the intended learning.
  - Source: `docs/HUMAN_GUIDANCE.md:592`

- Question Pools are always published and have no draft or unpublished state.
  - Source: `docs/HUMAN_GUIDANCE.md:595`

- A Question Pool is an independently reusable Question Library object.
  - Source: `docs/HUMAN_GUIDANCE.md:596`

- A Question Pool has its own public `AAAA-ZBBB` Crockford Base32 ID and immutable revisions.
  - Source: `docs/HUMAN_GUIDANCE.md:597`

- Importing a Question Pool into a new Assessment automatically forks the Question Pool.
  - Source: `docs/HUMAN_GUIDANCE.md:598`

- The fork belongs to the new Assessment and can be changed without changing the source Question Pool.
  - Source: `docs/HUMAN_GUIDANCE.md:599`

- Forking a Question Pool preserves its Published Questions by their public `AAAA-ZBBB` IDs.
  - Source: `docs/HUMAN_GUIDANCE.md:600`

- Question Pools work the same way regardless of the Question Backend.
  - Source: `docs/HUMAN_GUIDANCE.md:601`

- Starting a new Attempt makes fresh selections from its Question Pools.
  - Source: `docs/HUMAN_GUIDANCE.md:606`

### Questions -- Question Library

- Published Question Pools are available to all vetted **Instructors**.
  - Source: `docs/HUMAN_GUIDANCE.md:615`

- **Students** access Question content through their Coursework rather than through the Question Library.
  - Source: `docs/HUMAN_GUIDANCE.md:616`

- With 13,000 Questions in Neil's first course, manually archiving Questions is unlikely to be a useful primary workflow.
  - Source: `docs/HUMAN_GUIDANCE.md:618`

- Question Library workflows should support bulk operations because an Instructor may manage thousands of Questions.
  - Source: `docs/HUMAN_GUIDANCE.md:619`

- Instructors should be able to select many Questions and update shared metadata such as tags, subject, topic, or other search fields together.
  - Source: `docs/HUMAN_GUIDANCE.md:620`

- Question Library search, filters, sorting, and bulk editing should make large imports practical to clean up.
  - Source: `docs/HUMAN_GUIDANCE.md:621`

- Published Questions receive a public `AAAA-ZBBB` Crockford Base32 ID.
  - Source: `docs/HUMAN_GUIDANCE.md:625`

- Published Questions and published Question Pools have public Crockford Base32 IDs.
  - Source: `docs/HUMAN_GUIDANCE.md:626`

- Public IDs use the form `AAAA-ZBBB`.
  - Source: `docs/HUMAN_GUIDANCE.md:627`

- ID generation enforces database uniqueness and retries when a random collision occurs.
  - Source: `docs/HUMAN_GUIDANCE.md:631`

- Assessments and Student Work remain pinned to exact immutable Published Question Revisions.
  - Source: `docs/HUMAN_GUIDANCE.md:637`

- Any **Instructor** may fork a Published Question to create a separate Question with a new Question ID.
  - Source: `docs/HUMAN_GUIDANCE.md:643`

- Question stewardship should use a GitHub-like model.
  - Source: `docs/HUMAN_GUIDANCE.md:652`

- **Published Questions** can be starred and watched, similar to GitHub.
  - Source: `docs/HUMAN_GUIDANCE.md:653`

- Star means favorite and visible endorsement.
  - Source: `docs/HUMAN_GUIDANCE.md:654`

- Vetted **Instructors** can see the star count and which vetted **Instructors** starred a Question.
  - Source: `docs/HUMAN_GUIDANCE.md:655`

- Watch means subscription.
  - Source: `docs/HUMAN_GUIDANCE.md:656`

- Watching drives in-app notifications for revisions, forks, improvement threads, and impact notices.
  - Source: `docs/HUMAN_GUIDANCE.md:657`

- An **Instructor's** watch list remains private.
  - Source: `docs/HUMAN_GUIDANCE.md:658`

- **Students** and anonymous users do not receive **Instructor** identity lists or watch information.
  - Source: `docs/HUMAN_GUIDANCE.md:659`

- Question statistics are kept separately for each Published Question Revision.
  - Source: `docs/HUMAN_GUIDANCE.md:664`

- Each Published Question Revision may retain aggregate counts of correct, incorrect, partial-credit, and unanswered results.
  - Source: `docs/HUMAN_GUIDANCE.md:665`

- Question-level statistics may combine Revisions when clearly labeled and privacy thresholds are met.
  - Source: `docs/HUMAN_GUIDANCE.md:667`

- Question statistics contain aggregate counts rather than Student Attempts or identifiable Student records.
  - Source: `docs/HUMAN_GUIDANCE.md:668`

- Student data retention removes the underlying Student evidence without removing approved aggregate Question statistics.
  - Source: `docs/HUMAN_GUIDANCE.md:669`

- Removing Student names alone does not make statistics anonymous.
  - Source: `docs/HUMAN_GUIDANCE.md:670`

- Shared Question statistics should be shown only when individual Students cannot reasonably be identified from the aggregate.
  - Source: `docs/HUMAN_GUIDANCE.md:671`

- Course-specific Question analysis remains FERPA-sensitive when individual Students could be inferred.
  - Source: `docs/HUMAN_GUIDANCE.md:672`

- Answer-choice randomization belongs to the Question.
  - Source: `docs/HUMAN_GUIDANCE.md:676`

- PLE-native Questions control their own answer-choice randomization.
  - Source: `docs/HUMAN_GUIDANCE.md:677`

- Student workflows remain complete whether or not Students read Question Feedback.
  - Source: `docs/HUMAN_GUIDANCE.md:681`

### Assessments

- **Assessment** is the PLE object for organizing Questions into a graded or practice activity.
  - Source: `docs/HUMAN_GUIDANCE.md:827`

- Course Instance Assessments deliver Questions to **Students**.
  - Source: `docs/HUMAN_GUIDANCE.md:830`

- All Assessments use the same underlying Assessment model.
  - Source: `docs/HUMAN_GUIDANCE.md:831`

- **Assignment** is not a separate object or category. The word appears only in the names
  **Regular Assignment**, **Practice Question Assignment**, and **Bonus Assignment**.
  - Source: `docs/HUMAN_GUIDANCE.md:832`

### Assessments -- Assessment content

- Assessments contain an ordered sequence of Questions and Question Pools.
  - Source: `docs/HUMAN_GUIDANCE.md:837`

- **Instructors** can add, remove, and reorder Questions and Question Pools.
  - Source: `docs/HUMAN_GUIDANCE.md:838`

- Questions and Question Pools remain distinct even though both can occupy positions in an Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:839`

- Assessment Question-order randomization is called **Randomize question order**.
  - Source: `docs/HUMAN_GUIDANCE.md:840`

### Assessments -- Assessment types

- Assessment Type describes the pedagogical purpose of an Assessment and provides appropriate defaults.
  - Source: `docs/HUMAN_GUIDANCE.md:845`

- **Regular Assignments** give **Students** regular practice applying course ideas outside class.
  - Source: `docs/HUMAN_GUIDANCE.md:851`

- Regular Assignments reinforce current learning and may also introduce new topics.
  - Source: `docs/HUMAN_GUIDANCE.md:852`

- Regular Assignments are designed as practice for learning, not merely as one-time assessments.
  - Source: `docs/HUMAN_GUIDANCE.md:853`

- **Practice Question Assignments** provide focused review or study-guide practice using material already covered.
  - Source: `docs/HUMAN_GUIDANCE.md:854`

- Practice Question Assignments may be worth a small number of points or a small amount of extra credit.
  - Source: `docs/HUMAN_GUIDANCE.md:855`

- Practice Question Assignments use the same whole-Attempt submission boundary as every other
  Assessment and show the correct answer immediately after that Assessment Attempt is submitted.
  - Source: `docs/HUMAN_GUIDANCE.md:856`

- **Bonus Assignments** provide optional extra credit.
  - Source: `docs/HUMAN_GUIDANCE.md:858`

- **Quizzes** assess understanding of recent material.
  - Source: `docs/HUMAN_GUIDANCE.md:860`

- Quizzes may use more restrictive Attempt and collaboration settings than Regular Assignments.
  - Source: `docs/HUMAN_GUIDANCE.md:861`

- **Exams** are individual assessments associated with scheduled exam periods.
  - Source: `docs/HUMAN_GUIDANCE.md:862`

- Exams may use more restrictive Attempt, timing, availability, and feedback settings.
  - Source: `docs/HUMAN_GUIDANCE.md:863`

### Assessments -- Assessment type appearance

- **Bonus Assignment** uses the Font Awesome `star` icon.
  - Source: `docs/HUMAN_GUIDANCE.md:898`

- **Quiz** uses the Font Awesome `circle-question` icon.
  - Source: `docs/HUMAN_GUIDANCE.md:899`

### Assessments -- Blueprint Assessments

- Blueprint Assessments define Question point values and points possible.
  - Source: `docs/HUMAN_GUIDANCE.md:885`

- Blueprint Assessments have no **Students**, Student Work, due dates, release dates, or other Course Instance delivery settings.
  - Source: `docs/HUMAN_GUIDANCE.md:886`

- Blueprint Assessments do not use Assessment Templates.
  - Source: `docs/HUMAN_GUIDANCE.md:887`

- Creating a daughter Course Instance from a Blueprint Course copies its Blueprint Assessments into the Course Instance.
  - Source: `docs/HUMAN_GUIDANCE.md:888`

### Assessments -- Course Instance Assessments

- Course Instance Assessments are the Assessments delivered to **Students**.
  - Source: `docs/HUMAN_GUIDANCE.md:893`

- Course Instance Assessments also have delivery settings such as due dates, release status, and Student availability.
  - Source: `docs/HUMAN_GUIDANCE.md:895`

- Newly added Blueprint Assessments are automatically copied to daughter Course Instances as unreleased Course Instance Assessments.
  - Source: `docs/HUMAN_GUIDANCE.md:897`

### Assessments -- Assessment Templates

- An **Assessment Template** is a reusable set of settings for creating Course Instance Assessments.
  - Source: `docs/HUMAN_GUIDANCE.md:901`

- Assessment Templates are separate from Assessment Types.
  - Source: `docs/HUMAN_GUIDANCE.md:902`

- Every Assessment Template has one of the five Assessment Types.
  - Source: `docs/HUMAN_GUIDANCE.md:903`

- **Instructors** can create and change their own Assessment Templates.
  - Source: `docs/HUMAN_GUIDANCE.md:904`

- Assessment Templates provide defaults for settings such as Attempts, timing, scoring, and disclosure.
  - Source: `docs/HUMAN_GUIDANCE.md:905`

- Creating a Course Instance Assessment from a Template copies its settings into the new Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:906`

- The new Course Instance Assessment can be changed independently after it is created.
  - Source: `docs/HUMAN_GUIDANCE.md:907`

- Changing an Assessment Template does not change Assessments previously created from it.
  - Source: `docs/HUMAN_GUIDANCE.md:908`

- Assessment Templates do not contain Questions or Question Pools.
  - Source: `docs/HUMAN_GUIDANCE.md:909`

### Assessments -- Course Instance Assessment release and defaults

- Course Instance Assessments start unreleased.
  - Source: `docs/HUMAN_GUIDANCE.md:914`

- Assessment Release Validation checks the Assessment settings and data required for release.
  - Source: `docs/HUMAN_GUIDANCE.md:916`

- Validation should catch missing, invalid, or unreasonable values and explain what the **Instructor** needs to fix.
  - Source: `docs/HUMAN_GUIDANCE.md:937`

- Release Validation should check required settings such as point values, Attempt limits, and time limits for valid ranges.
  - Source: `docs/HUMAN_GUIDANCE.md:921`

- Release Validation should check that the Assessment contains Questions and that required Question settings are valid.
  - Source: `docs/HUMAN_GUIDANCE.md:922`

- Releasing an Assessment makes it available to **Students** according to its dates and access settings.
  - Source: `docs/HUMAN_GUIDANCE.md:925`

- Student Work begins when a **Student** starts an Assessment Attempt.
  - Source: `docs/HUMAN_GUIDANCE.md:926`

- **Practice Question Assignments** show correct answers immediately after Assessment Attempt
  submission.
  - Source: `docs/HUMAN_GUIDANCE.md:933`

- **Quizzes** and **Exams** show correct answers after all **Students** in the Course have completed the Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:935`

- Until then, Quizzes and Exams do not disclose correct answers.
  - Source: `docs/HUMAN_GUIDANCE.md:936`

### Assessments -- Assessment Attempts

- Question responses are saved as the **Student** works and remain part of the Attempt across browser sessions.
  - Source: `docs/HUMAN_GUIDANCE.md:945`

- **Instructors** control the number of permitted Assessment Attempts.
  - Source: `docs/HUMAN_GUIDANCE.md:946`

- Regular Assignments default to unlimited Attempts.
  - Source: `docs/HUMAN_GUIDANCE.md:947`

- **Students** may repeat an Assessment as often as its settings allow, including practicing toward a perfect score.
  - Source: `docs/HUMAN_GUIDANCE.md:948`

### Assessments -- Assessment responses and submission

- A Question either has a complete saved response or has no saved response.
  - Source: `docs/HUMAN_GUIDANCE.md:957`

- Questions without a saved response remain visibly unanswered when the Attempt is submitted.
  - Source: `docs/HUMAN_GUIDANCE.md:961`

- PLE treats an incomplete Question response as unsaved, although the Question interface may keep the Student's unfinished input while they work.
  - Source: `docs/HUMAN_GUIDANCE.md:964`

- A Question Backend may evaluate a response before Assessment submission when needed for its interaction.
  - Source: `docs/HUMAN_GUIDANCE.md:965`

- The **Student** does not see the grading outcome until the Assessment Attempt is submitted.
  - Source: `docs/HUMAN_GUIDANCE.md:968`

### Assessments -- Assessment Attempt timing and expiration

- Each Assessment Attempt has a time limit.
  - Source: `docs/HUMAN_GUIDANCE.md:972`

- Attempt time limits help **Students** develop an accurate sense of expected working speed.
  - Source: `docs/HUMAN_GUIDANCE.md:973`

- A **Student** may reconnect, reload, or use another browser session to resume the same active Attempt.
  - Source: `docs/HUMAN_GUIDANCE.md:977`

- Resuming an Attempt does not reset, pause, or extend its time limit.
  - Source: `docs/HUMAN_GUIDANCE.md:978`

### Assessments -- Student Work

- For a Question Pool, Student Work keeps the exact Question Pool Revision and Published Question Revision selected.
  - Source: `docs/HUMAN_GUIDANCE.md:988`

- Changes to Assessment content do not replace Question evidence already delivered in existing Attempts.
  - Source: `docs/HUMAN_GUIDANCE.md:990`

- PLE should retain only the additional historical Student Work data needed to interpret or grade that work correctly.
  - Source: `docs/HUMAN_GUIDANCE.md:991`

### Assessments -- Assessment scoring

- Blueprint Assessments and Course Instance Assessments assign point values to Questions.
  - Source: `docs/HUMAN_GUIDANCE.md:995`

- PLE does not use separate Question weights, Grade Categories, weighted categories, Course Grade
  Schemes, or Course percentage calculations.
  - Source: `docs/HUMAN_GUIDANCE.md:1003`

- For the pilot, grade export uses CSV or TSV only and exports point-based Assessment scores.
  - Source: `docs/HUMAN_GUIDANCE.md:1005`

- The Instructor handles Course-level weighting or percentage calculations in the home LMS.
  - Source: `docs/HUMAN_GUIDANCE.md:1006`
