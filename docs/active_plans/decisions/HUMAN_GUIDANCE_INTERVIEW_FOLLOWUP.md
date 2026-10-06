# Human guidance interview follow-up

This is an ongoing interview, not a complete design review. The original 27 open
audit bullets are not an inventory of all open product decisions.
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) remains product authority. This note
records interview reasons, expressed commitment, and follow-up work.

## Decision strength and reasons

Labels describe Neil's expressed commitment, not a numerical confidence score.
"Firm" reflects an explicit or emphatic rule. "Settled" means he chose the behavior
without saying it was immutable. "Flexible" preserves examples and possibilities.
"Open" means there is no accepted decision. "Delegated" is an engineering choice.
These classifications are the recorder's reading of his words; evidence is given
below so a later reviewer can distinguish direct statements from interpretation.

### Instructor onboarding

- Decision: A Sysadmin vets the Instructor outside PLE, creates an Account using
  an email address, first name, last name, and affiliation, and PLE sends setup email.
  Later access uses deactivate/reactivate.
- Strength: Firm. Neil said "No stupid approval pipeline" and corrected "approve"
  to "vetted before getting an account."
- Because: Vetting happens before the Account exists and belongs outside PLE;
  creating an Account should not introduce another approval process.
- Account fields: Settled by explicit statement; no separate rationale or immutable
  commitment was given. Neil called the name-entry question bikeshedding. Spend
  interview time on consequential teaching and assessment decisions.

### Assessment results timing

- Decision: Use the Blackboard Ultra model shown in Neil's four screenshots:
  separate availability and timing controls for submissions and correct answers.
  Question Feedback timing was subsequently deferred. Showing Question scores
  is not a controlled feature.
- Strength: Settled model choice; explicit exclusion of Question score controls.
  Neil did not state that every detail of the reference is immutable.
- Because: Neil supplied the model he wants; he did not give a separate rationale
  for excluding score controls. Do not invent one.
- Decision: Use Neil's preferred release behaviors as defaults that Instructors
  can change. The separate exclusion of Question score controls still applies.
- Strength: Current preference, open to reconsideration. Neil said "I Would love
  to enforce my defaults for all instructors, but I should probably be flexible."
- Because: Neil wants flexibility for other Instructors despite his own preferences.
- Reference: Screenshots show after-submission or individual-grade timing, after
  due date, after all grades are posted, and a specific date. Submission viewing
  also offers one-time viewing. These are reference options; their exact mapping
  to PLE's automatic grading still needs clarification.
- Decision: Keep the Quiz correct-answer default of waiting until all Students
  complete it. An Instructor can change that setting when a Student goes AWOL;
  there is no automatic deadline-based exception to the all-completed condition.
- Strength: Settled, with explicit acceptance of the tradeoff: "a necessary evil."
- Because: Neil said Quizzes require more attention from Instructors. The
  Instructor handles exceptions through the existing answer-visibility setting.
- Decision: Both Question scores and total Assessment scores are visible as soon
  as an Attempt is submitted and automatically graded. There is no Instructor
  control to withhold them and no separate score-posting step.
- Strength: Firm decision after considering flexibility for other Instructors:
  "let's remove the ability to delay scores."
- Because: Neil said withholding automated scores adds complication with little
  benefit, harms Students, and can prevent scores appearing when one Student
  goes absent. Separating grade entry from posting makes sense to him for manual
  grading, not automatic grading.
- Reconciliation: Replaced older HG wording about released scores, "Score not
  released," and Coursework settings controlling scores. Viewing submissions
  and correct-answer timing remain separate controls. The absent-Student case
  for Quiz answers is now settled as an Instructor-handled exception.
- Decision: Defer the feedback-timing decision until Neil better understands
  how feedback is used in PLE. Feedback itself remains optional.
- Strength: Explicit deferral: "Let's defer the feedback decision."
- Because: Neil wants a better sense of its actual use before choosing behavior.
- Follow-up: Review actual PLE-managed and backend feedback examples and where
  they appear. Then revisit timing and whether feedback follows answer visibility.
  Automatic display, a separate timing control, and following correct-answer
  visibility are unselected alternatives, not approved requirements. This defers
  the decision, not the existence of optional feedback content.

### Flawed Questions after submission

- Decision: The usual remedy is to set the Question's point value to zero at the
  Assessment level. Only the owner may correct a Native JSON Question. A bad
  WeBWorK Question requires a revision, so use zero points for affected work.
- Strength: Settled usual practice; Neil rejected the assistant's framing of a
  separate grade-correction feature: "None of this requires 'grade correction'."
- Because: Assessment point changes already recalculate scores. Neil explained
  that a bad WeBWorK Question cannot be fixed without a revision, making zero
  points the sensible remedy to him.
- Direction: Neil generally agrees that correcting a Native JSON answer key
  should recheck submitted responses, but worries this requires a major
  infrastructure change. This is conditional agreement, not a firm delivery
  requirement or authorization to redesign grading during the interview.
- Because: Corrected keys should benefit affected Students, but the cost and
  risk of changing grading infrastructure could outweigh that benefit now.
  The benefit is the assistant's proposed rationale; Neil explicitly raised cost.
- Decision: Regrading replaces the previous grading result; superseded grades
  are not retained. Changing the answer key back runs grading again.
- Ownership: Neil explicitly stated that only the Native JSON Question's owner
  can change it. Other Instructors retain the Assessment-level zero-point remedy.
  This confirms existing owner and revision rules, rather than adding a new
  permission or editing mechanism. No separate rationale or degree of flexibility
  was supplied for this reminder.
- Strength: Explicit direction on replacement. Deferral is the current, flexible
  choice: Neil said "I would probably still defer this."
- Because: Neil described PLE as a "change only system" and rejected keeping the
  old result. His earlier concern about implementation cost remains relevant.
- Engineering inspection: The database permits one grading result per Question
  Attempt and rejects updates through a trigger. That restriction can be changed.
  The existing Native JSON grader accepts source and response, so evaluation
  logic could be reused. The trigger alone does not establish a major rewrite.
- Interview correction: The assistant assumed retention of superseded grading
  results and overstated the implications of immutability. Neil rejected that
  assumption; a future implementation assessment must use replacement instead.
- Evidence: [grading result table](../../../schemas/base_schema/20_tables/assessment_attempt.sql),
  [grading functions](../../../schemas/base_schema/50_functions/grading.sql),
  and [saved-response evaluation](../../../crates/server/src/assessment_delivery/direct_finalization.rs).
  This was a narrow source inspection, not an implementation estimate or runtime test.
- Delivery: Automatic rechecking is deferred for now, with room to reconsider.
  Use the existing zero-point remedy in the meantime. No separate override
  workflow or grading-history feature is required by this decision.
- Scope: Replacement applies to superseded grading results. The interview did
  not repeal the existing Published Question revision rules. HG's correction
  wording now explicitly refers to those rules to avoid implying an exception.

### Instructor Profile visibility

- Decision: Everyone with a PLE Account can see Instructor Profiles and images,
  including Students viewing Question authors or Question Pool owners.
- Strength: Firm. "Instructors are public within PLE" and "Profile images should
  not have a permissions mechanism."
- Because: Instructor identity is public within the authenticated PLE community.

### Student collaboration

- Decision: Remove the Quiz collaboration sentence. PLE has co-instructors,
  not Student collaboration features.
- Strength: Firm. Neil identified the sentence as an unwanted agent addition.
- Because: The sentence invented a feature he did not intend.

### System-wide settings

- Decision: Place System-wide settings in deferred product behavior.
- Strength: Firm deferral; future contents are open.
- Because: Neil does not know what those settings would be. He explicitly asked
  to defer them rather than define an empty settings page.

### Blueprint search sorting

- Decision: Support all five named sorts and additional relevant search metadata.
  Say "number of students having taken the course."
- Strength: Firm on all named fields and aggregate wording. Additional fields are flexible.
- Because: "Such as" is not a cap on useful sorting. The Student metric is a count;
  wording suggesting access to identifiable Student data causes confusion.

### Shared search and display modes

- Decision: Use the modular spreadsheet-style interface for exploration, specific
  searches, and results from a simple search. It is also a valid starting view.
- Strength: Firm. Neil said the shared interface "SHOULD HAVE" multiple display modes.
- Because: Instructors need precise filters and useful results. Neil doubts that
  simple search alone would find what he needs and expects most Instructors to
  handle a Google Sheets-like interface with filters.
- Decision: Offer compact, list, and movie-poster-style boxes in that shared interface.
- Strength: Firm on multiple modes; mode names and visual details are flexible.
- Because: Different presentations help different tasks, especially finding visual Questions.
- Reference: Reddit illustrates multiple modes; OER Commons illustrates simple
  search with visible filters and image-focused results.
- Strength: Flexible design references, not exact layouts to copy.

### Tooltips and list navigation

- Decision: Use tooltips, not hover previews.
- Strength: Firm. Neil explicitly said he never wanted hover previews.
- Because: Hover previews add overhead and slow the interface.
- Decision: Opening list items, including search rows and Assessments in lists,
  opens a new tab/window. Ribbon navigation stays in the current tab; buttons
  perform their stated actions.
- Strength: Firm. Neil said "ALWAYS" for search rows and clarified "list items."
- Because: The original search remains available, avoiding repeated search time
  and server work. Ribbon navigation is navigation within the current interface.

### Leaving a search

- Decision: Confirm before discarding an existing search and its results; leave
  empty search pages directly.
- Strength: Settled by explicit selection. Neil did not state how immutable this is.
- Because: Recorder's interpretation: protect time spent building and reviewing
  a search when restoration is not guaranteed. Neil challenged silent departure
  and supplied an example leave-page warning.
- Decision: Do not store old search results for later restoration.
- Strength: Explicit rejection: "I would not store them at all."
- Because: Neil is not convinced there is an elegant storage approach. Earlier,
  he questioned reliability and pointed out that results could be four months old.
- Decision: Neither search prompts nor search results belong in permanent storage.
- Strength: Firm. Neil explicitly said "never permanent storage."
- Because: Neil wants to control database size and bloat. This extends the
  no-restoration decision to permanent storage of search prompts as well as results.
- The existing search page remains available when list items open in another
  tab. This is distinct from saving results to restore after leaving the page.
- The assistant's proposed automatic restoration is rejected, not an open feature
  to design. Confirmation before discarding an existing search remains required.

### Catalog identifiers

- Decision: Theme and avatar catalog key details are delegated engineering work.
- Strength: Delegated. Neil said "I do not care."
- Because: No product rationale was supplied. Keeping current text keys is the
  assistant's recommendation, not a human-mandated identity exception.

## Assessment editing restrictions: history and clarified direction

- Neil recalls that some Assessment fields were already locked once a Student
  started. The assistant's question about adding such a restriction was premature.
- Historical evidence supports the recollection:
  [August changelog](../../CHANGELOG-2026-08c.md) records `issuedStudentWork` as
  the conflict blocking structural mutation. Commit `1dd37192` replaced that
  response with a requirement for a successor Assignment Revision.
- Commit `4ed6b236` on September 12 removed that successor-revision mechanism
  during the redesign to current-state Assessments. The same commit introduced
  the contract allowing released Assessment edits to affect future Attempts.
- Current [Assessment editing contract](../../CONTRACTS.md#teaching-current-state)
  allows those edits after release validation. The current
  [save function](../../../schemas/base_schema/50_functions/assessments.sql)
  changes entries without a blanket prohibition based on existing Attempts;
  omitted entries become retired rather than being deleted from old work.
- Current [Attempt guards](../../../schemas/base_schema/50_functions/assessment_attempts.sql)
  preserve the started Attempt's policy, expiration, issued Questions, and Pool
  selections. These are distinct from preventing edits to the Assessment itself.
- Historical investigation: No separate human authorization for removing the
  structural-edit restriction was established in this bounded review.
- Current direction: After issue, allow a limited set of content edits: change
  Question/Pool points, reorder Questions, and remove individual Pool members
  only when they have not been issued to any Student in that Assessment.
  Remove a bad Pool as a whole and exclude its score contribution for everyone.
  Standalone Question removal retains the earlier "I would probably allow."
- Strength: Point changes and reordering are explicit allowances. The initially
  tentative Pool-member restriction is now the cautious rule, following Neil's
  direction to remove a bad Pool as a whole and prioritize fairness. Whole-Pool
  removal and its score effect are firm. Standalone Question removal remains
  tentative. Neil's earlier BB Ultra comparison was not independently verified.
- Because: Neil identified fairness to all Students as the ultimate goal. If a
  whole Question or Pool is removed, remove its earned and possible points from
  every Attempt, including existing Attempts. Keeping its score contribution
  only for earlier Attempts is rejected as unfair.
- Strength: The score effect of complete removal is firm ("only the Yes option
  makes sense"). Neil subsequently settled whole-Pool removal: "if a question
  pool is bad remove the whole thing, and its score is nulled" and "fairness
  needs to take precident."
- Neil also reminded us of Assessment Unrelease. Existing HG already defines
  this as deleting all Student Work, returning to pre-release state, and allowing
  normal editing before another validated release. This is an existing mechanism,
  not a new workflow or an exception that preserves earlier Attempts.
- Pool-member removal: Neil proposed this restriction after considering the
  difficulty of fair scoring when only some Students received a flawed Question.
  Under this rule, individual removal cannot change an already-issued Question or its
  score. Issued includes in-progress work, not just submitted responses. The
  Assessment scope follows this discussion of Assessment-level edits.
- Consequence: A flawed Pool member already issued cannot be removed individually.
  Remove the bad Pool as a whole; its earned and possible points are excluded
  from all Attempts. Setting the whole Pool's points to zero also remains an
  existing Assessment-level remedy. No per-Student scoring exception is introduced.
- Open: Standalone Question removal remains tentative; Pool removal and
  whole-entry removal's score effect are settled. Also keep
  these content restrictions distinct from existing date and answer-visibility
  settings; this list has not established a freeze on every Assessment property.
- Evidence is source and history inspection, not a fresh runtime test. The new
  content-edit direction is recorded in HG; implementation remains follow-up.

## Design-decision authority review: findings and document corrections

The findings below describe the wording found during review. Neil then explicitly
requested removal of conflicting contract wording and correction of the Pool
model. DESIGN_DECISIONS and CONTRACTS now reflect public Instructor Profiles,
immediate scores, current-state Pools, required classification, and post-issue
Assessment limits. They no longer authorize general discussion threads or
Sysadmin-only Pool moderation. Production reconciliation remains outstanding.

- [Verified Instructor Display Name](../../DESIGN_DECISIONS.md#instructor-profiles-are-visible-within-ple)
  restricts name visibility to Star lists and treats the name as an uneditable
  vetting attribute. The visibility restriction conflicts with current HG's
  public-within-PLE Instructor Profiles; the extra name-editing restriction has
  no explicit support identified in the reviewed HG.
- [Account avatars](../../DESIGN_DECISIONS.md#account-avatars-are-one-role-neutral-aggregate)
  and CONTRACTS still describe self-only image delivery. That is superseded by
  the interview's Instructor Profile visibility decision.
- [Student Progress](../../DESIGN_DECISIONS.md#student-progress-distinguishes-completion-from-score)
  and Response Stats still depend on withheld scores. Those decisions are
  superseded by immediate automated score visibility.
- [Bulk metadata editing](../../DESIGN_DECISIONS.md#bulk-metadata-editing-is-an-all-or-none-current-state-command)
  describes only Tags, Subject, and Topic as currently defined shared fields;
  HG explicitly includes Discipline and Subtopic. Follow-up source inspection
  found both fields in `bulk_replace_published_question_metadata`; the documented
  field restriction is stale, not evidence that the current SQL omits them.
  The document also describes clearing Subject despite HG requiring one Subject
  per Library Object. Reconcile the document with required classification rules;
  this review has not demonstrated a runtime validation failure.
- [Current state](../../DESIGN_DECISIONS.md#current-state-is-not-a-hidden-revision-family)
  still lists published Question Pools as immutable Revision families, contrary
  to HG and the same document's later current-state Pool decision.
- [Library improvement activity](../../DESIGN_DECISIONS.md#content-stewardship-follows-human-guidance)
  gives only Sysadmins authority to resolve/reopen Pool discussion threads and
  manage Pool impact notices. HG requires GitHub-like stewardship and notifications
  but does not specify that restriction. This affects who can handle reported
  content problems and could make routine Pool stewardship depend on a Sysadmin.
  This finding concerns discussion/notice administration, not Pool content edits.
  No replacement permission rule has been selected.
- Assessment editing contracts still allow broad changes for future Attempts;
  they need to reflect the post-issue limits and whole-Pool score treatment
  settled in this interview. This is reconciliation with new guidance, not a
  reason to reopen the fairness decision.
- Lower-priority conventions such as the seven-day Due Soon window are not
  stated in HG, but that alone does not make them conflicting requirements or
  justify spending this interview on them. HG explicitly delegates usage-statistic
  schema and increment rules to DESIGN_DECISIONS and FERPA_DATA_POLICY; those
  details must not be labeled unsupported merely because HG omits their formulas.
- These are bounded document findings, not a complete repository audit. Some
  became stale during this interview; that does not prove their original authors
  ignored the guidance that existed at the time. Technical details compatible
  with HG are not automatically unsupported product decisions.

## Content stewardship and Stars: human correction

- Neil explicitly rejects Verified Instructor as a PLE concept. He vets people
  before they enter PLE; once admitted, all Instructors are equal. Strength: firm.
  This reaffirms HG's existing equal-capabilities rule; it is not a newly chosen
  product policy. The extra verification tier was conflicting agent-written wording.
  Removed wording implying verified/vetted permission tiers from HG, design,
  terminology, and contracts. Existing Account state and content ownership rules
  do not create another Instructor role. Source reconciliation remains follow-up.
- Blueprint Courses are structurally very similar to GitHub repositories, with
  a few key differences. Strength: emphatic direction ("very, very similar").
  Use this as the design baseline, with HG defining PLE-specific differences.
  The analogy does not authorize GitHub's entire feature set.

- Neil says Sysadmins should be reluctant to get involved in any content. PLE
  is neither social media nor an online forum. Strength: explicit direction.
- The assistant's "endorsement list" meant who Starred an item; Neil did not
  author or recognize the special identity restriction. He directs PLE to follow
  GitHub's Star policy. Use Stars and who Starred in product language.
- GitHub's [Star documentation](https://docs.github.com/en/get-started/exploring-projects-on-github/saving-repositories-with-stars)
  describes starring/unstarring, finding saved content, and seeing who Starred
  content the viewer can access. Apply that model within PLE's existing content
  access rules. It does not require a new forum, identity restriction, feed, or
  public anonymous access to PLE content. Watch lists remain private under HG.
- Neil prefers Change Proposals as the term for Instructors; he naturally uses
  PR because he knows GitHub. Strength: clear terminology preference. Because:
  Change Proposals is more suitable language for Instructors.
- He recalls proposals for Questions, not Pools. Current HG separately defines
  Blueprint Course Change Proposals and describes Question owner revisions and
  forks. Keep the Blueprint rules. The exact Question proposal mechanics still
  need confirmation; his reply selected terminology, not detailed behavior.
- Removed HG's references to improvement threads as notification events, along
  with design/contract claims that general retained discussion threads and routine
  Sysadmin Pool moderation are approved. Existing code remains for later review;
  documentation edits do not demonstrate its removal from the running system.

## Further authority review and language cleanup

- API descriptions still withheld scores in Progress, Attempt History, and Response
  Stats. Updated them to the settled immediate-score rule; code and DTOs remain
  follow-up, not verified compliant by these documentation edits.
- WeBWorK review text treated whole-Course completion as a fixed answer gate.
  Corrected it to the Instructor-selected setting, with all-Students-completed
  as the Quiz/Exam default. Reconciled the same default language in terminology
  and the duplicate Unit Review bullet in HG.
- Terminology selected automatic backend feedback despite HG deferring that
  decision. Removed the selection, and marked Latest Feedback's existing API
  behavior as implementation evidence pending the deferred timing decision.
- Native JSON being an unpublished, unversioned format does not mean Native
  Questions cannot be Published or have Revisions. Clarified that distinction.
- Corrected the terminology claim that Student Ribbon layout is unsettled; HG
  already defines it. Removed an extra Account settings menu entry not listed in
  HG. Corrected Blueprint scan summaries to include the fields HG names rather
  than treating existing API fields as the product limit.
- Removed remaining approved/vetted-Instructor role language, including obsolete
  approval wording in HG's glossary. This applies earlier human decisions.
- The support contract added a mandatory Instructor-issued grant for Sysadmin
  repair access. Neil rejects that requirement: "if Sysadmin has power, they have
  power." Strength: firm. Because: administrative authority belongs to the
  Sysadmin role and does not need to be delegated by an Instructor. Sysadmins can
  initiate repairs directly; existing task limits and audit requirements remain.
  Updated HG, design, authorization, and API/contract descriptions accordingly.
  The existing grant workflow still needs production reconciliation.
- Neil subsequently deferred much of the Sysadmin functionality beyond creating
  Instructor Accounts. Strength: explicit deferral. Because: more administrative
  capabilities will be needed, but their actual needs are still unproven. The
  no-Instructor-approval authority rule remains settled; it does not authorize
  inventing or implementing replacement support workflows now. Added the deferral
  near the top of HG and to the affected contracts.
- This is a bounded review of design, terminology, authorization, API, and backend
  contracts. It does not establish that every design decision or implementation
  matches HG. Lower-impact conventions are not new interview priorities.

## Repeated design language in supporting model documents

- Repeated-practice notes still claimed Quizzes had collaboration settings.
  Removed that rejected rule and speculative future authoring-collaboration
  language in Object Storage. This applies the existing no-collaboration decision,
  not a new restriction on ordinary co-Instructors, forks, or Change Proposals.
- Assessment lifecycle and repeated-practice notes used old Regular Assignment
  and Practice Question Assignment names. Replaced them with Weekly Assignment
  and Unit Review Assignment, including a wrapped occurrence in disclosure prose.
- Supporting lifecycle, payload, activity, and terminology documents still
  selected optional feedback timing and sometimes made scores subject to disclosure
  policy. Reconciled immediate scores and deferred feedback timing; added the
  already-decided post-issue limits and whole-Pool score treatment where needed.
- The H5P backend contract attributed future content types, pinned library versions,
  and xAPI decisions to a recorded HG question. HG only defers that backend.
  Reworded these as future product/engineering work, not decisions made by Neil
  or current delivery requirements.
- Removed remaining verified/vetted-Instructor tier wording in the Question,
  identity, security, and object-storage models reviewed in this pass. Clarified
  that access to Library Star lists does not restrict public-within-PLE Profiles.
- No new human decision was needed for these corrections. This review does not
  mark existing runtime behavior compliant. Historical changelogs and generated
  schema documentation remain evidence of the code/history they describe.

## Shared spreadsheet interface

- Neil expects a modular, reusable spreadsheet-style interface, like Google Sheets
  with filters. He is concerned that earlier shared-interface work was reverted
  or replaced with another custom Question Library page.
- He expects most Instructors to handle this interface. A simple search alone
  would frustrate him because he doubts it would return useful results.
- Neil wants tooltips, not hover previews. Hover previews add overhead and slow
  the interface; the existing HG wording misstates his intent.
- The shared interface must offer multiple display modes: compact, list, and
  movie-poster-style boxes. These are descriptions; final mode names are undecided.
- Neil cites Reddit's display choices and [OER Commons](https://oercommons.org/)
  as references. Image-focused boxes could help Instructors find visual Questions.
- Treat these as modes of the shared interface, not a choice between separate
  page implementations or a reason to omit one of the modes.
- Follow up after the interview; keep implementation work separate from collecting
  the remaining Human Guidance decisions.

### Evidence checked on 2026-10-04

- Shared components remain in
  [record_table.tsx](../../../src/components/record_list/record_table.tsx) and
  [record_list.tsx](../../../src/components/record_list/record_list.tsx).
- `src/pages/library_browse_rows.tsx` (historical path; retired) renders
  Question Library results with `RecordList`.
- [library_page.tsx](../../../src/pages/library_page.tsx) owns the search form,
  filters, and sorting. `RecordTable` provides columns and row headers but has
  no built-in column-filter or sort-state contract.
- The checked migration commit, `8fe0eadd`, also has a basic `RecordTable` and a
  Question Library using `RecordList`. This bounded comparison does not establish
  whether an earlier spreadsheet capability existed or was removed.
- [record_list_image_browser.tsx](../../../src/components/record_list/record_list_image_browser.tsx)
  provides Gallery/List modes. Shared presentation-state helpers also remain in
  [record_list_presentation.ts](../../../src/components/record_list/record_list_presentation.ts).
  The checked Question Library result component does not expose these mode controls.
- OER Commons returned HTTP 403 to the research tool. Neil later supplied a
  screenshot showing a text box, Subject, Education Level, Standard, and Search
  in one row. That screenshot supports the simple-search reference.

### Follow-up work

- Trace the intended shared-interface contract and relevant history.
- Identify what reusable spreadsheet filtering already exists and what is missing.
- Carry the agreed search design through shared components and the query layer.
- Verify the resulting Question Library in the browser against Neil's stated vision.
- Align search navigation with the decision not to store old results for later
  restoration; retain separate tabs for list items and search-discard confirmation.
- Continue the interview beyond the original compliance-audit list. Review open
  requirements in the actual teaching and authoring workflows before declaring
  the product design complete.

## Blueprint Watches clarification during implementation

- Neil's direction: "follow the GH repo model for watches."
- Firmness: direct instruction to use that model. Neil did not separately specify
  each notification option or identity-list permission.
- Why: reuse the repository model already chosen for Blueprint Courses; avoid
  inventing a PLE-specific meaning of Watch.
- Implementation mapping: Watch/Unwatch subscribes to Blueprint notifications;
  the Watches search sort uses the aggregate number of subscribers, distinct
  from Stars and from the current Instructor's own subscription state.
- Checked GitHub's [Watch API documentation](https://docs.github.com/en/rest/activity/watching?apiVersion=2022-11-28)
  and [June 2026 access change](https://github.blog/changelog/2026-06-30-upcoming-access-restrictions-to-public-api-endpoints-and-ui-views/).
  GitHub distinguishes subscriber counts from Stars and restricts watcher-list
  access. PLE search shows the count without a public watcher-identity list.
  This mapping does not add GitHub's collaboration roles to PLE.

## Pool member licenses: undecided

- Neil wants Pool members to have compatible licenses and strongly dislikes the
  idea of mixed-license Pools. He is unsure whether to require an identical
  license and version or permit different compatible licenses.
- Firmness: strong preference against mixed-license Pools; exact membership rule
  undecided. His answer to those two alternatives was "I do not kno, either 1 or 2".
- Why: keep Pool membership consistent with his preference against mixed licenses.
  The agent suggested identical licenses as simpler to enforce, but Neil has not
  chosen that rule.
- Follow-up: compare the practical restrictions of the two choices before making
  either an enforced rule. Member-license rules and the Pool's own license are
  separate questions. Continue with teaching-related membership rules meanwhile.

### Creative Commons reference supplied by Neil

- Neil supplied the [CC License Compatibility Chart](https://wiki.creativecommons.org/wiki/File:CC_License_Compatibility_Chart.png).
- The agent checked the image and the current
  [CC guidance on combining works](https://creativecommons.org/faq/#can-i-combine-material-under-different-creative-commons-licenses-in-my-work).
  The chart concerns remixes. CC separately explains that a collection's license
  does not replace the licenses of its member works.
- Agent inference: a PLE Pool of separate, unchanged Question references is closer
  to a collection. The remix chart should not automatically become a legal
  requirement for Pool membership.
- The earlier recommendation to require identical licenses for implementation
  simplicity was premature. A concrete CC BY / CC BY-SA example is being used
  to clarify whether Neil wants compatible differences or strict uniformity.
  His supplied link is evidence for discussion, not approval of a final rule.

### Pool license minimum settled

- Neil clarified: "at minimum only compatible licenses, but perhaps stronger".
- Decision: Pool members must have compatible licenses.
- Firmness: the compatibility minimum is settled. Requiring an identical license
  and version remains open; do not treat it as approved.
- Why: Neil wants to avoid incompatible license combinations and has expressed
  discomfort with mixed-license Pools even where compatibility might permit them.
- Follow-up: specify and verify the concrete compatibility rule for PLE's supported
  licenses. The remix/collection distinction remains relevant to that definition.
  No new membership enforcement has been implemented in this discussion.

## Pool Question Type: stronger membership direction

- Neil: "I am thinking of higher requirements for pools, all contain the same
  type of question, no mixing match with mc."
- Direction: every member of a Pool has the same Question Type; Matching and
  Multiple Choice do not belong together in one Pool.
- Firmness: current design direction; Neil is still exploring the stronger
  membership requirements. Do not report this as implemented.
- Why: Neil wants stronger similarity among Pool members. Consequence inferred
  by the agent: Question Type filtering can describe the entire Pool, avoiding
  the previously proposed any-member/all-members choice for mixed-type Pools.
- The earlier recommendation to include mixed-type Pools when any member matches
  is superseded by this direction. Question Backend is a separate property;
  whether it must also be uniform is the next interview question.
- Manager handoff: plan for uniform Question Type; verify the concrete type
  identity used across supported Backends before implementing membership checks.
  Do not silently remove existing members to make a Pool satisfy the rule.

### Same Backend confirmed

- Neil selected: "Require one Backend per Pool: all Native JSON or all WeBWorK."
- Decision: every Pool has one Question Type and one Question Backend; its members
  share both. The Backend rule supersedes the earlier HG wording permitting
  members from any Backend within a Pool.
- Firmness: Backend uniformity explicitly selected. Same Question Type remains
  Neil's stated direction, now carried into HG alongside the Backend rule.
- Why: Neil wants stronger similarity among members. The search consequence is
  an agent inference: Type and Backend filters can describe the entire Pool.
- Manager handoff: establish membership enforcement before treating these as
  guaranteed Pool properties in search. Audit existing mixed Pools explicitly;
  do not silently remove members. Source and checklist updated; code work pending.

### Reason for stronger Pool similarity

- Neil: "I have done like a 180, and I think having more common pools allows for
  better search and better assessments".
- Why: uniform Pool membership should improve search and Assessments. This is
  Neil's stated reason for the stronger direction, replacing the need to infer
  his reason from implementation convenience.
- Firmness: clear change toward stronger member similarity, including the stated
  same-Type and selected same-Backend rules. Further restrictions, such as
  identical licenses or shared Topic/Subtopic, remain undecided.

### Pool classification similarity settled

- Neil selected: "Keep only the same Discipline and Subject as required
  classification matches."
- Decision: Pool members share Discipline and Subject. Topic and Subtopic may
  differ. Same Question Type, same Backend, and reasonable interchangeability
  remain separate requirements.
- Firmness: explicit selection; settled for this design.
- Why: Neil did not provide a separate reason for this boundary. His broader
  rationale is better search and Assessments through stronger Pool similarity;
  do not infer that every classification field must therefore match.
- Manager handoff: do not require common Topic/Subtopic when creating or editing
  a Pool, or present member Topic/Subtopic as guaranteed uniform values.

## Pool metadata and member metadata

- Neil explicitly allows different authors among member Questions and requires
  the Pool to have its own Topic and Subtopic. These are settled directions.
- Why: Neil distinguishes metadata belonging to the Pool from metadata belonging
  to its members; Topics/Subtopics primarily support search. Broader motivation
  remains better search and better Assessments.
- Pool: its own title, description, owner, Topic, and Subtopic. Pool ownership is
  distinct from Question authorship. Existing optional Pool attribution/license
  concerns the Pool's own material, not replacement of member licenses.
- Members: preserve each Question's authorship, license, title, description,
  Topic, Subtopic, and Tags. Their authors and Topics/Subtopics may differ.
- Shared membership requirements: Discipline, Subject, Question Type, and Backend.
  A Pool may display their common values without changing its members.
- Licenses must be compatible; the possible stricter identical-license rule
  remains open. Do not invent a single replacement license for all members.
- Agent proposal, not yet approved: use Pool-specific Tags rather than an
  automatic union of member Tags. Which metadata search filters should match
  remains the next question. Search promotion/ranking is also unresolved.

### Question ownership correction

- Neil corrected the metadata split: "Each keeps its owners AND authors; owners
  AND authors may differ".
- Decision: each member Question retains its own owner and authors. Different
  members may have different owners and authors. Pool ownership is separate;
  membership does not transfer ownership of the Questions.
- Firmness: explicit correction, settled.
- Why: ownership and authorship are distinct and both belong to the individual
  Question; recording only authorship loses that distinction.
- Wording clarification: the agent's "any Pool attribution" meant source credit,
  such as identifying the original Pool when a Pool is forked. Use concrete
  source-Pool wording when referring to that relationship. This clarification
  does not add a role or decide any additional Pool-credit fields.

### One required Pool license

- Neil: "since we require only compatible licenses for members, we can assign the
  pool that compatible master license. So the pool must have a license, it cannot
  be mixed."
- Decision: each Pool has one required license compatible with every member
  Question's license. "Mixed" is not a Pool license. Earlier optional Pool-license
  wording is superseded.
- Firmness: explicit direction, settled. Different compatible member licenses
  are accommodated by this design; identical member licenses are not required
  by the selected single-Pool-license rule.
- Why: compatible member licenses allow the Pool to have one clear license for
  discovery and reuse rather than a mixed label.
- Preserve each Question's original license. CC's collection guidance says the
  collection license does not replace licenses on the individual works. The
  Pool label must not imply rights absent from a member's original license.
- Manager handoff: Pool license filters use the Pool's license. Membership writes
  must respect its compatibility with member licenses. The concrete mapping for
  supported licenses and the choice between automatic assignment and owner
  selection remain to be specified. No enforcement was implemented here.

### Pool-specific Tags and Bloom dimensions confirmed

- Neil: "yes, we should have pool specific tags, pool specific Blooms 2D
  attributes, etc."
- Decision: a Pool has its own Tags, Bloom Knowledge Dimension, and Bloom
  Cognitive Process, alongside its own Topic and Subtopic. Member Questions
  retain their own metadata.
- Firmness: explicit direction, settled for these named fields. "Etc." does not
  approve arbitrary additional fields or a new metadata system.
- Why: the Pool and its members are distinct objects with their own descriptions
  for search. This carries forward Neil's stated separation of their metadata.
- Manager handoff: expose Pool metadata independently of member values. Do not
  substitute member values for missing Pool fields or require member values to
  equal Pool values without an explicit membership rule. Which values search
  filters use to match a Pool is the pending question.

### Concrete Pool-license example confirmed

- Neil's example: members under public domain, CC BY, CC BY-NC, and CC BY-NC-SA
  yield a Pool license of CC BY-NC-SA. Adding a CC BY-SA Question is rejected as
  incompatible.
- Firmness: explicit example, settled as PLE membership behavior.
- Why: the Pool needs one compatible license; its members cannot impose
  conflicting reuse conditions. Compatibility is not a single most-restrictive
  ordering of all licenses.
- This is a PLE membership policy. CC distinguishes remix compatibility from
  collecting unchanged works; the original member licenses remain intact.
  Reference: [Creative Commons combining guidance](https://creativecommons.org/faq/#can-i-combine-material-under-different-creative-commons-licenses-in-my-work).
- Implementation gap: `generated/api/QuestionLicense.ts` and
  `schemas/base_schema/10_types.sql` currently allow only CC0-1.0, CC-BY-4.0,
  and CC-BY-SA-4.0. The example requires adding CC BY-NC and CC BY-NC-SA through
  the publication, validation, database, generated API, search, and export code.
  Do not add them only to a Pool dropdown or relabel existing Question licenses.
- Public-domain status and CC0 are distinct descriptions; the concrete supported
  identifiers need mapping during implementation. The example does not approve
  NoDerivatives licenses or every historical license version.

### Cost recovery and NonCommercial content: unresolved

- Neil asked whether charging only the site's operating costs permits CC BY-NC
  content in PLE. This is a question, not a confirmed pricing or license decision.
- CC's [NonCommercial explanation](https://wiki.creativecommons.org/wiki/NonCommercial_interpretation)
  evaluates the primary purpose of the use, including commercial advantage or
  monetary compensation. Cost recovery and nonprofit status do not by themselves
  establish permission for a particular deployment.
- Agent conclusion: no legal clearance established for a paid PLE deployment.
  Review the actual fee arrangement with counsel or obtain rights-holder
  permission before relying on NC licensing for that arrangement.
- Keep software support for NC metadata and compatibility separate from whether
  a particular hosted use meets the NC condition. This does not change Neil's
  Pool compatibility example or add an in-product approval workflow.

### NC supported; ND deferred

- Neil: "I see no reason to not include CC NC; but CC ND 'NoDerivative' would
  have to block forking, let's plan to support CC ND, but defer that for now."
- Decision: support NonCommercial Question licenses, including the CC BY-NC and
  CC BY-NC-SA licenses in his example. Supporting NoDerivatives licenses is
  planned but deferred; ND Question forking must be blocked when implemented.
- Firmness: explicit direction for both current NC scope and ND deferral.
- Why: Neil wants NC content available in PLE; ND introduces a forking restriction
  that needs separate future work.
- NC support does not establish legal permission for every hosted fee arrangement.
  Keep that deployment question separate from the supported-license code.
- Deferred detail: distinguish forking an ND Question from forking a Pool that
  merely references an unchanged ND Question when ND support is resumed. Do not
  use this future distinction to block the current shared search work.
- Manager handoff: expand NC support consistently; leave ND acceptance disabled
  for the current implementation. Preserve each member's original license.

### NC inclusion reopened

- Neil asked what PLE's NC policy should be and said he is less certain. This
  reopens the preceding decision to include NC in current scope.
- Firmness: current NC inclusion is now tentative. ND remains explicitly deferred.
- Agent recommendation: defer NC acceptance for the initial hosted release while
  the fee arrangement is unresolved. Continue using the existing CC0, CC BY,
  and CC BY-SA set; retain future NC support and the Pool compatibility example.
- Why: cost recovery alone does not establish NonCommercial use under CC's
  guidance. Avoid relying on an unsettled deployment assumption for release.
- Await Neil's choice. No implementation change or automatic withdrawal of the
  existing HG direction has been made on the strength of this recommendation.

### Pool filters confirmed; NC deferred

- Neil: "I think yes to this: Topic/Subtopic, Tags, and Bloom filters match only
  the Pool's own metadata"; then "let's defer NC content as well".
- Decisions: named metadata filters use Pool values only. NC content joins ND
  content in the deferred section. Earlier current-scope NC approval is superseded.
- Firmness: Pool-only filtering is accepted with Neil's "I think yes" qualifier;
  NC deferral is explicit. Preserve that difference in confidence.
- Why: Pool-specific metadata describes the Pool; member metadata remains separate.
  NC was deferred following uncertainty about the hosted service's fee model.
- Manager handoff: current licenses remain CC0, CC BY, and CC BY-SA. Retain the
  NC compatibility example as deferred guidance. Do not expand NC acceptance in
  the current search work. Do not make a Pool match a Topic, Tag, or Bloom filter
  solely because one member matches.
- Remaining search decisions: keyword matching of member content; the meaning
  of Owner/Author filters for Pools; automatic versus owner selection of the
  Pool's compatible license. Ranking/promotion remains separate from metadata.

### Pool keyword search confirmed

- Neil selected: "Search only the Pool's own text and metadata."
- Decision: ordinary text search does not match a Pool through its member
  Questions' text or metadata.
- Firmness: explicit selection, settled.
- Why: the recommendation Neil selected keeps text search consistent with the
  Pool's own metadata. This is the presented rationale; no separate reason was
  supplied by Neil.
- Manager handoff: no member-text expansion or matching-member explanation is
  needed for Pool results. Member Questions still match their own search results.

### Pool Author field removed from design

- Neil selected: "Use the Pool owner and source-Pool link; no separate Pool
  Author field."
- Decision: the Pool identifies its owner and its source Pool when forked. It
  does not have separate author credits. Member Question owners and authors
  remain unchanged.
- Firmness: explicit selection, settled. Supersedes earlier optional Pool
  authorship language in HG.
- Why: the selected proposal uses the established owner and fork-source facts;
  Neil supplied no additional rationale. Preserve Question authorship separately.
- Manager handoff: show and filter Pool ownership as Owner, not Author. Do not
  match a Pool through member authors or fabricate a Pool Author from its owner.
  Existing Pool authorship storage and API behavior require alignment; docs-only
  discussion has not completed that work.

### Pool license calculated automatically

- Neil selected: "Calculate the Pool license automatically from its members."
- Decision: PLE calculates one Pool license from its members and rejects
  incompatible member combinations. Pool owners do not manually choose the
  Pool license. Each Question retains its original license.
- Firmness: explicit selection, settled.
- Why: the selected recommendation keeps owners from having to calculate license
  compatibility manually. Neil did not add a separate reason.
- Manager handoff: compute from current membership on creation and membership
  changes. Validate exact license versions using a compatibility table, not an
  assumed total order. Current support remains CC0, CC BY, and CC BY-SA; NC/ND
  remain deferred. The CC BY plus CC BY-SA example produces a CC BY-SA Pool.
- Pool promotion/ranking remains a separate search decision. No new product
  implementation was performed during this interview.

### Pool ranking question withdrawn

- Neil called the proposed preference for equally relevant Pool results
  "bikeshedding" and rejected spending time on it.
- Decision: do not add a Pool ranking preference from this interview.
- Why: it was an unnecessary proposed distinction, and the agent had assumed a
  relevance sort that the current Question search does not provide. The current
  `QuestionSearchSort` supports titleAscending and publishedNewest.
- Firmness: explicit rejection of this question. It does not answer whether
  Questions and Pools share one result set or use separate searches.
- Manager handoff: remove ranking/promotion from the unresolved implementation
  requirements. The pending combined-versus-separate result-set question affects
  real query, filtering, and paging responsibilities.

### Combined Question and Pool search confirmed as original intent

- Neil selected one combined search with Both, Questions only, and Pools only,
  then clarified that combining Questions and Pools was the original reason for
  starting this work and must be documented in HG.
- Decision: one combined Question Library search; result-kind filters select
  both Published Questions and Pools, Questions only, or Pools only.
- Firmness: explicit selection and correction; settled original requirement,
  not a new optional expansion.
- Why: Questions and Pools belong together in Library discovery. The original
  shared-search goal was combined discovery, not merely similar-looking pages.
- Manager handoff: combine matching records before server sorting and paging;
  do not merge separately paged lists in the browser. Preserve each kind's own
  searchable metadata and stable identity. The schematic now states this scope.
- Existing HG supplies picker scope: Pool members are Published Questions only;
  adding a Pool references it; explicit forks create independent Pools (Q29). These do not require new
  interview questions. Documentation is updated; implementation remains pending.

### Existing HG authority verified

- Neil emphasized that combined Question-and-Pool search should ALREADY be in HG.
- Verified against committed HG before this interview's edits: it already says
  "The Question Library is one global collection of Published Questions and
  Question Pools." Git history traces that statement to `eecc03ac`, September 16.
- Correction: combined discovery is an existing product requirement the agent
  failed to carry into the schematic. Asking Neil to choose separate searches
  reopened established intent. This is not a newly authorized scope expansion.
- The added Both/Questions only/Pools only wording makes the filter behavior
  explicit. The schematic now corrects the missing combined-query responsibility.

### Pool corrections after edge-case review

- Neil rejected the premise of unpublished Pools. HG already says Pools enter the Library
  immediately. The manager must use the existing lifecycle rather than invent a second category.
  Firmness: explicit correction. Why: the question contradicted the documented Pool model.
- A Pool is an unordered set of distinct Published Questions, with exact Revision pins retained.
  It cannot contain two copies of the same Question. This supersedes HG's ordered-list wording.
  Firmness: explicit direction. Why: Pool usage selects Questions randomly.
- Neil called overlap across Assessment entries speculative bikeshedding. Do not add restrictions
  or automatic deduplication because a directly added Question is also in an Assessment Pool.
  Firmness: explicit rejection of proposed gates. Why: Instructors choose their Assessment content.
- Later clarification (Q28): each Assessment entry stores its selection count. Save incomplete
  unreleased work, report that Assessment's Pool-use mismatch, and validate at release. The earlier
  claim that save/release timing was merely an implementation choice was an agent inference.
- When a member's Discipline or Subject changes so it no longer matches the Pool, flag that Pool
  and prevent release until the mismatch is resolved. Firmness: explicit behavior; the word
  "stale" is flexible. Why: membership requirements must not silently become invalid; this
  rationale follows from the instruction rather than a separate explanation supplied by Neil.
- Manager handoff: current SQL treats classification as an admission-only check and explicitly
  leaves existing memberships unchanged by reclassification. That behavior needs alignment.
  Use the existing Assessment release flow, not a new Pool publication stage. Already-released
  Assessments must be reconciled with existing post-issue limits without silently changing
  delivered Questions. No policy for that interaction has been inferred from this answer.
- Documentation only: no runtime behavior was changed or certified by this recording step.

### Pool editor display order

- Neil: "no reason for a pool to be ordered, other than display purposes"; member Questions
  should be "sortable like in a spreadsheet" when editing a Pool.
- Decision: Pool membership is unordered, while the editor supports sorting its member display.
  Firmness: explicit direction. Why: ordering serves inspection and editing, not random selection.
- Implementation consequence: a display-sort change is not a membership edit and must not advance
  the Pool's membership Edit Number or alter selection. Reuse the shared spreadsheet presentation.

### Pool mismatch terminology settled

- Use "Pool mismatch: The Pool no longer satisfies its current requirements." This supersedes
  the provisional "stale" wording in earlier interview entries.
- Pool mismatch causes include member Discipline or Subject mismatch, duplicate Questions, and
  other Pool membership violations. Q28 later distinguished an Assessment requesting too many
  Questions as a Pool-use mismatch for that Assessment; the Pool itself may still be valid.
- Firmness: explicit terminology supplied after reviewing alternatives. Why: "mismatch" is
  broader and more neutral than "stale" and describes failure to meet current requirements.
- Each Assessment entry owns its selection count. Before release, save incomplete work and show
  its Pool-use mismatch; release validates completeness. Pool membership remains a set of distinct
  Questions. These rules introduce no separate Pool publication workflow.
- Documentation only; state reporting and release enforcement remain unverified implementation work.

### Organized search default and released Assessments

- Neil clarified that both Questions and Pools should show up in search. This already covers
  the search/picker concern; do not reopen their combined discovery as a product question.
- Define "Questions in no Pool" as Published Questions with no Pool membership. Firmness:
  explicit request. This filter is distinct from showing all individual Questions.
- Preferred default: show Questions in no Pool plus Question Pools, with member Questions
  available through the filters. Firmness: current preference, retaining Neil's "probably".
  Why: redundant individual Pool members add significant noise to the combined search.
- Existing Pool-only search matching still applies. Agent observation: the preferred default
  does not expose a Question-only text or metadata match when its Pool does not also match.
  The broader member-inclusive search remains available; do not silently change Pool matching.
- If a Pool mismatch develops after an Assessment is already released, allow it to continue
  as-is. Firmness: explicit direction. Why: Neil sees no useful remedy for an already-released
  Assessment. This closes the previously open post-release mismatch question.
- Later clarification (Q28): an unreleased Assessment requesting too many Questions does not
  prevent a Pool membership save. Show the Pool-use mismatch and block release. Established
  post-issue editing restrictions remain; this record establishes no runtime compliance.

### Required metadata and pending Bloom assignment

- Question Type cannot be NULL; other required metadata must be complete before publication.
  Firmness: explicit requirement. Why: publication must not rely on incomplete required fields.
- Neil explained that Instructors may not have the attention to detail to fully fill out metadata.
  PLE should assign what it can automatically and require the remaining required content before
  publication. This does not make explicitly optional fields mandatory or assign all metadata to AI.
- Bloom is assigned initially by AI and may remain NULL while awaiting assignment. Neil first
  suggested 24 hours, then explicitly answered "do not enforce any time limit". The latter
  supersedes the proposed deadline. Initial AI classification remains deferred under existing HG.
- Firmness: no time-limit enforcement is explicit. Missing Bloom is not a publication failure;
  missing Question Type or another required field is. Existing Instructor Bloom corrections remain.
