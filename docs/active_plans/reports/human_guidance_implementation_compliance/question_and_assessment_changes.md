# Question And Assessment Changes

## Scope

This is a fresh implementation-audit inventory. Each record below is an owning `[ ]`
Human Guidance bullet: the behavior is unverified or differs from the current implementation.
The bullet text is copied verbatim from the generated checklist, and its source location is
recorded beside it. Later duplicate bullets that carry an `Owner:` pointer are excluded because
their earlier owning record is the single inventory entry.

The authoritative exhaustive record is the
[generated checklist](../../audits/human_guidance_implementation_checklist.md).

## Owning inventory

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

- PLE has **Blueprint Assessments** and **Course Instance Assessments**.
  - Source: `docs/HUMAN_GUIDANCE.md:828`

- Blueprint Assessments define reusable Assessment content and teaching settings.
  - Source: `docs/HUMAN_GUIDANCE.md:829`

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

- PLE defines the available Assessment Types.
  - Source: `docs/HUMAN_GUIDANCE.md:844`

- Assessment Type describes the pedagogical purpose of an Assessment and provides appropriate defaults.
  - Source: `docs/HUMAN_GUIDANCE.md:845`

- Assessment Types are **Regular Assignment**, **Practice Question Assignment**, **Bonus Assignment**, **Quiz**, and **Exam**.
  - Source: `docs/HUMAN_GUIDANCE.md:846`

- **Instructors** select an Assessment Type but cannot create new Assessment Types.
  - Source: `docs/HUMAN_GUIDANCE.md:847`

- Blueprint Assessments and Course Instance Assessments use the same Assessment Types.
  - Source: `docs/HUMAN_GUIDANCE.md:848`

- **Instructors** can change Assessment settings independently of the defaults for its Type.
  - Source: `docs/HUMAN_GUIDANCE.md:849`

- Changing Assessment settings does not change its Assessment Type.
  - Source: `docs/HUMAN_GUIDANCE.md:850`

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

- Bonus Assignments are worth zero points possible and add earned points directly to the grade.
  - Source: `docs/HUMAN_GUIDANCE.md:859`

- **Quizzes** assess understanding of recent material.
  - Source: `docs/HUMAN_GUIDANCE.md:860`

- Quizzes may use more restrictive Attempt and collaboration settings than Regular Assignments.
  - Source: `docs/HUMAN_GUIDANCE.md:861`

- **Exams** are individual assessments associated with scheduled exam periods.
  - Source: `docs/HUMAN_GUIDANCE.md:862`

- Exams may use more restrictive Attempt, timing, availability, and feedback settings.
  - Source: `docs/HUMAN_GUIDANCE.md:863`

### Assessments -- Assessment type appearance

- Each Assessment Type has its own PLE-defined Font Awesome icon.
  - Source: `docs/HUMAN_GUIDANCE.md:867`

- Assessment Type icons remain consistent across PLE themes.
  - Source: `docs/HUMAN_GUIDANCE.md:868`

- Each Assessment Type also has its own theme-defined color.
  - Source: `docs/HUMAN_GUIDANCE.md:869`

- Themes may change Assessment Type colors but preserve the meaning of each Type.
  - Source: `docs/HUMAN_GUIDANCE.md:870`

- Assessment Type should never be communicated by color alone.
  - Source: `docs/HUMAN_GUIDANCE.md:871`

- Icons and labels should remain sufficient to identify the Assessment Type without color.
  - Source: `docs/HUMAN_GUIDANCE.md:872`

- **Regular Assignment** uses the Font Awesome `pen-to-square` icon.
  - Source: `docs/HUMAN_GUIDANCE.md:873`

- **Practice Question Assignment** uses the Font Awesome `arrows-spin` icon.
  - Source: `docs/HUMAN_GUIDANCE.md:874`

- **Bonus Assignment** uses the Font Awesome `sparkles` icon.
  - Source: `docs/HUMAN_GUIDANCE.md:875`

- **Quiz** uses the Font Awesome `square-q` icon.
  - Source: `docs/HUMAN_GUIDANCE.md:876`

- **Exam** uses the Font Awesome `file-signature` icon.
  - Source: `docs/HUMAN_GUIDANCE.md:877`

### Assessments -- Blueprint Assessments

- A **Blueprint Assessment** is an Assessment in a **Blueprint Course**.
  - Source: `docs/HUMAN_GUIDANCE.md:881`

- Blueprint Assessments have an Assessment Type.
  - Source: `docs/HUMAN_GUIDANCE.md:883`

- Blueprint Assessments define Question point values and points possible.
  - Source: `docs/HUMAN_GUIDANCE.md:885`

- Blueprint Assessments have no **Students**, Student Work, due dates, release dates, or other Course Instance delivery settings.
  - Source: `docs/HUMAN_GUIDANCE.md:886`

- Blueprint Assessments do not use Assessment Templates.
  - Source: `docs/HUMAN_GUIDANCE.md:887`

- Creating a daughter Course Instance from a Blueprint Course copies its Blueprint Assessments into the Course Instance.
  - Source: `docs/HUMAN_GUIDANCE.md:888`

### Assessments -- Course Instance Assessments

- A **Course Instance Assessment** is an Assessment in a **Course Instance**.
  - Source: `docs/HUMAN_GUIDANCE.md:892`

- Course Instance Assessments are the Assessments delivered to **Students**.
  - Source: `docs/HUMAN_GUIDANCE.md:893`

- Course Instance Assessments have an Assessment Type, Questions, Question Pools, point values, and points possible.
  - Source: `docs/HUMAN_GUIDANCE.md:894`

- Course Instance Assessments also have delivery settings such as due dates, release status, and Student availability.
  - Source: `docs/HUMAN_GUIDANCE.md:895`

- Course Instance Assessments copied from a Blueprint Assessment can be changed for the needs of that Course Instance.
  - Source: `docs/HUMAN_GUIDANCE.md:896`

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

- Releasing a Course Instance Assessment requires an automated and interactive **Assessment Release Validation** process.
  - Source: `docs/HUMAN_GUIDANCE.md:915`

- Assessment Release Validation checks the Assessment settings and data required for release.
  - Source: `docs/HUMAN_GUIDANCE.md:916`

- Validation should catch missing, invalid, or unreasonable values and explain what the **Instructor** needs to fix.
  - Source: `docs/HUMAN_GUIDANCE.md:917`

- Release Validation should require a due date at least 24 hours in the future and no later than the
  Course Instance's six-month Active limit.
  - Source: `docs/HUMAN_GUIDANCE.md:918`

- Release Validation should check that release, due, and other dates occur in a valid order.
  - Source: `docs/HUMAN_GUIDANCE.md:920`

- Release Validation should check required settings such as point values, Attempt limits, and time limits for valid ranges.
  - Source: `docs/HUMAN_GUIDANCE.md:921`

- Release Validation should check that the Assessment contains Questions and that required Question settings are valid.
  - Source: `docs/HUMAN_GUIDANCE.md:922`

- The **Instructor** should be able to correct validation problems and run Release Validation again.
  - Source: `docs/HUMAN_GUIDANCE.md:923`

- An Assessment can be released only after Release Validation passes.
  - Source: `docs/HUMAN_GUIDANCE.md:924`

- Releasing an Assessment makes it available to **Students** according to its dates and access settings.
  - Source: `docs/HUMAN_GUIDANCE.md:925`

- Student Work begins when a **Student** starts an Assessment Attempt.
  - Source: `docs/HUMAN_GUIDANCE.md:926`

- New Course Instance Assessments default to accepting submissions only through the due date.
  - Source: `docs/HUMAN_GUIDANCE.md:927`

- New Course Instance Assessments default to starting new Attempts only through the due date.
  - Source: `docs/HUMAN_GUIDANCE.md:928`

- Late work defaults to rejected.
  - Source: `docs/HUMAN_GUIDANCE.md:929`

- Assessment disclosure settings remain separate and independently configurable.
  - Source: `docs/HUMAN_GUIDANCE.md:930`

- **Regular Assignments** and **Bonus Assignments** should rarely show the correct answer.
  - Source: `docs/HUMAN_GUIDANCE.md:931`

- Regular and Bonus Assignments show the **Student's** response and whether it was correct or incorrect.
  - Source: `docs/HUMAN_GUIDANCE.md:932`

- **Practice Question Assignments** show correct answers immediately after Assessment Attempt
  submission.
  - Source: `docs/HUMAN_GUIDANCE.md:933`

- **Quizzes** and **Exams** show correct answers after all **Students** in the Course have completed the Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:935`

- Until then, Quizzes and Exams do not disclose correct answers.
  - Source: `docs/HUMAN_GUIDANCE.md:936`

### Assessments -- Assessment Attempts

- An **Assessment Attempt** is one Student attempt at a Course Instance Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:943`

- Blueprint Assessments do not have Assessment Attempts.
  - Source: `docs/HUMAN_GUIDANCE.md:944`

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

## Count method

This report owns **181** checklist records. The count is the number of `[ ]` bullets
in the listed sections after excluding records with a later-duplicate `Owner:` pointer.
It is mechanically reconciled with the other topical inventories by the temporary report
generation check; it is not a permanent test.
