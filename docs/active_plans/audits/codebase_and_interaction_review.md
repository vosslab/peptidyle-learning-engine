# Codebase and interaction review

Read-only review of the Rust workspace, the SolidJS browser, the documented contracts, and the
committed screenshot corpus, with `OTHER_REPOS/adapt` as comparison evidence.

Review date: 2026-08-18. Tree identity: `bfdbdd7f5d597adf0aa5c2108785ca9a22cfb7b3`.
Every finding, citation, and disposition is in
[codebase_and_interaction_review_evidence.md](codebase_and_interaction_review_evidence.md); this
document carries the decisions.

This review recommends. It accepts no work package and completes none of the work it proposes.

## Evidence boundary

This is an expert inspection with no participants. Findings rest on reading current source, on the
committed screenshot corpus, and on tooling run during the review. **No application stack was
started and no browser capture was run**, so no claim here describes runtime behavior, and every
finding that depends on a screenshot is recorded as unresolved rather than confirmed.

Screen-reader and human accessibility evaluation stay outside this review.
`docs/NO_MOUSE_ACCESSIBILITY_CONTRACT.md:281-286` already requires representative VoiceOver and NVDA
walkthroughs before accessibility is claimed for the Fall pilot, and `:277-279` names the response
families still resting on component evidence rather than a full route.

One earlier reading was withdrawn by evidence gathered here: the mock-captured screenshots are not
current. Tooling built during the review shows every committed artifact predates the current browser
sources, the mock set by one commit and the live set by three.

## What is working

The system has several boundaries that are stronger than the norm, and the recommendations below
leave all of them intact.

- The answer-key boundary is enforced by the compiler and by tests, not by convention: the Wasm
  dependency closure is asserted to be exactly `{wasm_bridge, domain, question_model}`, `grading` is
  absent from both the Wasm and export crates, and `FeedbackContent` is proven to lack serialization
  derives and to be absent from browser-facing files.
- Publication immutability is enforced by PostgreSQL triggers, so a compromised application role
  cannot rewrite published questions.
- Tenancy has one non-defaultable entry point, forced row-level security, and eleven least-privilege
  roles.
- The `AAA-BBBB` question identity is implemented exactly as specified, and no internal identifier
  reaches a browser route: public references are branded types resolving at four functions.
- API decoding is closed-world, so an unexpected server field becomes an error rather than silently
  flowing into components.
- Memory and PostgreSQL backends share one conformance suite.
- TypeScript discipline is unusually high: no `any`, two non-null assertions, one lint suppression
  across roughly 35,700 lines.

## Highest-risk issues

Ranked by consequence rather than by how wrong they are.

1. **Students can read the published question catalog (SEC-1).** Five catalog routes carry no role
   requirement and the Library navigation link is ungated, against the stated rule that the Library
   serves vetted Instructor accounts. A student can read prompts of questions on work they have not
   started, which defeats timed practice. Answer keys are unaffected. This is the one finding where
   current behavior contradicts a settled owner requirement.
2. **The published evidence misrepresents the product (EVD-3, EVD-5, EVD-6).** All five `README.md`
   images come from the older capture set; one displays the `P-2-v1` identity scheme the project
   replaced, and a sibling image shows a raw learner-UUID column. A design review cites one of these
   images as proof of a layout the current code implements differently. The public front page of the
   project currently advertises a version of the interface that no longer exists.
3. **The product's central promise is now observable for learners (STU-2).** Assignment
   overview now includes learner-visible mastery outcome fields from `student_assignment_summary`
   before practice starts.
4. **Timing is shown before a timed run starts (STU-1).** The student now sees the assignment time limit on
   the overview screen before starting practice.
5. **Schema drift is caught only by suites that do not run (ENG-1).** Roughly 450 hand-written
   queries, no compile-time checking, and the checking suites are explicitly excluded from
   `check_rust.sh`. This is the largest correctness exposure in the backend.
6. **A repository gate is currently red (ENG-10).** `check_codebase.sh` step 2 fails on a
   pre-existing type error in a test file, unrelated to this review.

## Category A: settled requirements that current behavior has yet to meet

These follow from owner decisions already recorded, so they are not proposals.

| ID | Requirement | Current behavior |
| --- | --- | --- |
| SEC-1 | The Library serves vetted Instructor accounts | Catalog routes and the navigation link are open to any authenticated session |
| SEC-2 | Student access stays narrow | Nine instructor-only routes are gated per page with no single declaration |
| EVD-5 | No sequential or version identity, and no UUID, in visible content | Both appear in images shipped from `README.md` |
| INS-5 | A direct position selector accompanies directional controls | Only directional controls exist |
| DOC-7 | Layout adapts with media and container queries | No container query exists |

## Category B: recommendations from evidence

Grouped into six areas rather than itemized, so scope decisions stay at the right altitude.

**1. Student access boundary.** Close the catalog read routes at the server, where the established
`Instructor | Sysadmin` pattern already exists, and derive navigation gating from one declarative
role requirement on the route contract rather than per page. The server is the authority; hiding the
link alone would not change what a session can fetch. Decide deliberately whether a student may
resolve a single assigned question, which is a different route than browse and search.

**2. Learner-facing teaching loop.** Show the run time limit before the student commits; project the
existing summary fields into a learner-visible mastery view; replace seed and attempt vocabulary
with teacher language; settle on one verb for entering practice across the three surfaces and the
keyboard contract. These are small and they address the gap between what the product promises and
what the learner can see.

**3. Instructor observation.** The item-analysis capability is complete server-side with no
interface, and the gradebook is a flat cross-product list where the instructor's mental model is a
matrix. ADAPT's gradebook demonstrates the matrix with sticky learner columns and export; PLE's
mastery cell content is richer and worth carrying into that shape rather than adopting ADAPT's
single-score cell.

**4. Evidence trustworthiness.** Regenerate the corpus from the manifest introduced during this
review, replace the `README.md` image set, re-cite the design review, and rewrite the palette audit
against colors that exist. Decide whether corpus verification becomes a standing gate; the tooling
supports it and the failure mode it prevents has already occurred once.

**5. Visual system conformance.** Eleven observations from the corpus await fresh capture. Treat
them as one pass after regeneration rather than eleven separate tasks, since several may retire.

**6. Foundational engineering before more code lands.** Compile-time SQL checking, the `Store`
facade's delegating twins, the adapter that reaches persistence for one type, the duplicated
cross-crate types, the mock client in the production bundle, the second HTTP transport, and the
CSS-in-TS colors outside the token system. WP-RC5 will add eight families of authoring widgets into
exactly the files that are already at the size ceiling, so this work is cheaper before that lands
than after.

## Category C: open product decisions

Carried forward rather than guessed.

- Whether a student may open a single assigned question outside a run, once browse and search close.
- Whether the Library becomes course-scoped in addition to instructor-only.
- Whether dark mode is a deliberate omission; no `prefers-color-scheme` handling exists.
- Whether any consumer needs the aggregate `Store` facade, which bounds how far that cleanup goes.
- Whether the unowned corpus image `peptide_bond_mastery_overview.png` is adopted or removed; it is
  produced by no pipeline and cited by no document.

## Before the pilot, and later

Recommended future work, not work this review performed. The pilot is roughly two weeks out.

**Recommended before the pilot**

- The student access boundary (SEC-1, SEC-2). It contradicts a settled requirement and concerns
  student-visible content.
- The public evidence refresh (EVD-3, EVD-5, EVD-6, EVD-9). The README currently misrepresents the
  product to anyone who reads it, including pilot participants.
- The retention authority correction (DOC-2). `docs/API_CONTRACTS.md` states a FERPA-adjacent
  boundary that disagrees with both the policy document and the code.
- A copy pass on the Chapter 1 corpus. One published question has a visible grammar defect.

**Reasonable to defer**

- The gradebook matrix and the item-analysis interface. Both are substantial and neither blocks the
  teaching loop for a single pilot course.
- Compile-time SQL checking, capability-ownership cleanup, layering repair, and browser boundary
  work. All are durable improvements whose cost rises after WP-RC5, and none changes pilot behavior.
- Palette derivation and the visual conformance pass, pending fresh capture.
- The wider documentation drift register beyond DOC-2.

This split rests on source-confirmed findings. The visual conformance items are excluded from the
pre-pilot list precisely because their evidence is unresolved.

## Suggested next step

Regenerate the corpus and run the catalog probe. Both are inexpensive, both convert unresolved
findings into confirmed or retired ones, and the probe tests the highest-risk finding directly. A
probe result contradicting the source reading would be a valuable outcome and should be reported
rather than reconciled.
