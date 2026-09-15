# Unresolved or ambiguous items

Temporary working report for decisions that Human Guidance does not resolve.

No choice recorded here authorizes implementation.

| Item | What Human Guidance settles | What remains unresolved |
| --- | --- | --- |
| Practice answer timing | Practice Question Assignments always show the correct answer after the Student responds; no Student-visible grading outcome appears before whole-Attempt submission | Whether and how immediate correct-answer feedback is separated from the grading outcome |
| Question Feedback timing | Included Question Feedback is always shown, and reading it is not required to complete the workflow | Its exact disclosure time relative to response save and whole-Attempt submission |
| Unanswered-position score treatment | A Question without a complete saved response remains unanswered when the whole Assessment Attempt is submitted | Whether an unanswered position contributes zero credit, changes a score denominator, or has another score treatment |
| Multiple-Attempt grade selection | Instructors set Attempt limits; Regular Assignments default to unlimited Attempts | Which Attempt, if any, contributes to the Course grade and how repeated Attempts appear in totals |
| Course grade model and export | Assessment scores derive from immutable fractions and current point values | Whether Course Grade Schemes, Grade Categories, weighting, or a Gradebook export should exist |
| Pool Revision wording | The Question Pool sections require stable Pool IDs, immutable Pool Revisions, and exact Pool Revision evidence | The general history summary names only Published Question and Blueprint revisions; whether that omission is intentional wording or a conflicting restriction |
| Student and Sysadmin Ribbon detail | Student navigation centers Courses/Coursework; Sysadmin navigation centers Accounts, Instructors, Courses, and system settings | The complete fixed Slot/Task decomposition for those roles |
| Retention intervals | Final Assessment deadline and Student activity drive notice, archive, recovery, deletion, and inactivity | Numeric inactivity, notice, recovery, and backup-erasure intervals |
| Blueprint workflow mechanics | Daughter update review, automatic new-Assessment copying, selective fork updates, Change Proposals, and canonical Blueprint JSON are required | Exact API routes, persistence/concurrency shapes, failure recovery, and treatment of inactive daughter Courses |
| Course copy and rollover | Course Instances may start empty or from a Blueprint and keep independent names | Whether a separate term-copy, rollover, or date-shift workflow exists |
| Course banner geometry | An Instructor can upload a small centered Course banner and select a three-color theme | Exact displayed dimensions, aspect ratio, rendition set, and crop behavior; the old 6:1 page-width selection is superseded |
| Backend continuation | A backend returns an immutable credit fraction for a complete response finalized with the Attempt; no public grading lifecycle exists | Whether a real backend may require server-only deferred completion and, if so, its bounded contract |
| Account permanent closure | Instructor deactivation/reactivation preserves identity and history; permanent closure is separate | The permanent-closure workflow and its interaction with authored content and Course retention |
| Archive confirmation detail | Question and Blueprint archive actions explain effects and require clear confirmation | Whether either archive action should use typed-title confirmation |
| Generated documentation | Generated artifacts are implementation evidence and do not override Human Guidance | Correcting their product labels requires generator/UI changes outside this docs-only task |
| Residual Assignment wording in Human Guidance | The Assessment section says Assessment is the generic object and limits Assignment to three Type names | Earlier Human Guidance sections still use Assignment generically for Course definitions, deadlines, scores, Pool import, and Question access; supporting docs interpret those as Assessment, but the authority wording itself needs clarification |

## Generated artifacts requiring a later source-owned refresh

- [docs/SCREENSHOT_ATLAS.md](../../../SCREENSHOT_ATLAS.md) and `docs/screenshots/`, including
  [docs/screenshots/current_capture_manifest.json](../../../screenshots/current_capture_manifest.json);
- [docs/ux/RIBBON_DESTINATION_LEDGER.md](../../../ux/RIBBON_DESTINATION_LEDGER.md) generated table; and
- [docs/GRAPHIFY.md](../../../GRAPHIFY.md) / [docs/GRAPHIFY_map.svg](../../../GRAPHIFY_map.svg) when the source model is reconciled.
