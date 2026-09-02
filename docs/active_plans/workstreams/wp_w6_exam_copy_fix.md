# WP-W6 closed-exam completion wording

## Status

**ACCEPTED SOURCE/TEST PREREQUISITE.** The source-of-truth student completion wording is corrected
and focused production completion-boundary tests cover the allowed, closed, and unresolved summary states.
The [WP-W6 copy review](../audits/wp_w6_exam_copy_fix_review.md) accepts this prerequisite only.
The paired live Mastery-versus-Exam keyboard journey, simulator report row, and independent browser
acceptance remain owned by the later WP-W6 walkthrough work.

## Scope

- `src/pages/assignment_attempt_page.tsx` shows neutral `Assignment Attempt complete` wording while the summary policy is
  unresolved.
- Once loaded, only `practiceAllowed: true` shows `Keep practicing with a fresh variation` and
  its corresponding action. A closed Assignment Attempt instead says `This Assignment Attempt is complete`.
- The page continues to expose its existing labelled Back to assignment action in every state.

## Evidence

- Focused production completion-boundary tests complete a server-issued rendered response and observe its normal summary request. They assert allowed,
  closed, and unresolved policy wording, action availability, and the persistent Back control.
- This accepted prerequisite does not claim the WP-W6 visible policy contrast or policy-engine
  behavior. It uses no arranged Exam, live stack, simulator report, or browser walkthrough evidence.
