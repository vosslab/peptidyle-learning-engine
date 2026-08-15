# Plan: rewrite the professor capability architecture plan around a course spine

## Context

`PROFESSOR_CAPABILITY_ARCHITECTURE_PLAN.md` (repo root, untracked) already names most of the
professor cycle: discovery, collections, blueprints, Alpha courses, cloning, manual grading, item
analysis. It is a strong plan, but the investigation below found that it is organized as a
**capability list layered on top of the current design**, and it misses several load-bearing
abstractions that multiple professor workflows all depend on.

The decisive evidence is that Peptidyle's professor gap is mostly a **projection deficit**, not a
storage deficit. The schema and domain crates already hold assignment lifecycle, availability/due/
close, late policy, per-student and per-group policy exceptions, item pools with draw counts,
`gradebook_included`, `attempt_selection_policy = instructor_selected`, manual grading, course item
analysis, catalog statistics disclosure, and retention operations. The frozen 19-route browser
contract (`src/route_contract.ts`) exposes none of them. Two capabilities are missing at the
foundation instead: the course has **no term or time zone**, and there is **no course grade scheme**.

This plan rewrites that document: same ambition, reorganized around seven named abstractions, with
the newly found gaps folded in, and moved into the location `docs/REPO_STYLE.md` requires.

---

## Ideonomy artifact

```
  ___          __                      ___      _
 | _ \_ _ ___ / _|___ ______ ___ _ _  / __|_ __(_)_ _  ___
 |  _/ '_/ _ \  _/ -_|_-<_-</ _ \ '_| \__ \ '_ \ | ' \/ -_)
 |_| |_| \___/_| \___/__/__/\___/_|   |___/ .__/_|_||_\___|
                                          |_|
```

```
+- TUPLE - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -+
|  * OPERATORS    substitution * cross-domain-reinstantiation                    |
|  * ORGANON      dictionary                                                     |
|  * DIMENSIONS   autonomy * intentionality * size                               |
+- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -+
```

```
+- DIMENSIONS ------------------------------------------------------------------+
|                                                                               |
|  autonomy         professor-driven O=====================o system-driven      |
|                   every state change today needs a professor click; retention  |
|                   jobs already prove a worker exists that could close, release,|
|                   and notify on schedule                                       |
|                                                                               |
|  intentionality   designed O===========o========== emergent                    |
|                   policy is designed in the schema, emergent in the UI: due    |
|                   dates, extensions and late rules exist as columns but reach  |
|                   the professor as nothing, so real courses would grow ad-hoc  |
|                   workarounds around them                                      |
|                                                                               |
|  size        *    one item o=========O===================o department          |
|                   the plan is written at course scale. Below it sits the item  |
|                   pool (schema-present, invisible). Above it sit term, section |
|                   set, and multi-instructor program. * pivot - both ends are   |
|                   unowned, and both are where professors actually live         |
|                                                                               |
+-------------------------------------------------------------------------------+
```

```
===================================================================================
  *  SUBSTITUTION  *   holding "professor capability" fixed, swapping its scale
                       value: course -> term, course -> section-set, course -> item pool
===================================================================================
```

Substituting the scale value exposes three unowned scales. At **term** scale the course has no
start, end, or IANA time zone, so relative Alpha schedules, rollover date shifting and
daylight-saving refusal (all promised by the current plan) have nothing to resolve against. At
**section-set** scale `course_group` exists with no purpose type, so one course taught to three
sections cannot express per-section windows or audiences. At **item-pool** scale
`assignment_selection_group` and `assignment_selection_candidate` already store draw counts,
per-item points, ordering policy and an algorithm version, and no browser route reaches them; a
professor cannot build "10 drawn from 40" even though the engine can deliver it.

```
===================================================================================
  *  CROSS-DOMAIN RE-INSTANTIATION  *   abstract form "authored score, rehearsed
                       privately, performed once, reviewed after" -> theater production
===================================================================================
```

Theater separates **script** (reusable, editable), **rehearsal** (private, no audience, no box
office), **performance** (one live take, unrepeatable, recorded in the prompt book), and **notes**
(the director's after-review that changes the next performance, never the one already given).
Peptidyle owns script (blueprint / Alpha), performance (issued run) and notes (item analysis), and
has **no rehearsal**. ADAPT's answer is a test student and "log in as student", which manufactures a
fake enrollment inside the FERPA record set. The theater answer is better and is available here: an
instructor-owned rehearsal run bound to the assignment definition, structurally incapable of
producing an enrollment, a gradebook row, or an item-analysis observation.

The same re-instantiation supplies **repertory**: a play runs in many houses from one score, and the
house reports back to the author. Because Peptidyle publishes shared immutable questions rather than
per-course copies (ADAPT cannot do this), item statistics can aggregate across every course that
ever used a question. That is a capability ADAPT structurally cannot have, and the plan currently
treats it as a detail-page metric rather than as the flagship analysis surface.

```
===================================================================================
  *  DICTIONARY  *     eleven terms the plan needs; genus, differentia, and the
                       workflows each one unblocks
===================================================================================
```

**Course term** - a course-owned value carrying start date, end date, and IANA time zone. It is the
only thing that makes a relative curriculum schedule resolvable. Unblocks: rollover, Alpha
instantiation, DST refusal, "shift my whole term by five days", retention timing, archive naming.
Today the `course` table stores only `title`.

**Effective assignment policy** - a server-side resolver, not a table: given (assignment, learner,
now) it returns one window, time limit, attempt limit, lateness verdict, and the reason for each.
It composes base policy, audience, group exception, personal exception, and late rule. Unblocks:
assignment list badges, run-start authorization, late marking, gradebook lateness, accommodation UI,
clone preview. Without it, five surfaces each re-derive lateness and drift apart.

**Learner disclosure policy** - one assignment-owned statement of what a learner may see and when:
score, per-item correctness, feedback text, correct answer, solution, class statistics. Genus:
policy; differentia: it is evaluated at read time by the server for every learner-facing projection.
Today `feedback_disclosure` covers part of this at attempt scale only. Unblocks: "release solutions
after close", "hide scores until everyone finishes", statistics visibility, and the run summary.

**Course grade scheme** - an ordered set of weighted categories with drop rules, rounding, and
letter bands, owned by the teaching course, producing one course total per learner. `assignment.
gradebook_included` is its only current trace. Unblocks: course total column, letter grades, LMS
grade export at course scale, student progress, and any intervention judgement more useful than
per-assignment scores.

**Rehearsal run** - an instructor-owned run against a teaching assignment that cannot create an
enrollment, gradebook row, or analysis observation, and that is visibly labelled as rehearsal
throughout. Genus: run; differentia: no educational record. Replaces ADAPT's test student and
"login as". Unblocks: pre-flight checking of a new assignment, verifying accommodations, checking
what students actually see, and confidence before a pilot.

**Question usage index** - a reverse index from a published question to the assignments, courses,
collections, and Alpha curricula that reference it, exposed with tenant-safe aggregation. Unblocks:
search facets ("used in my courses"), correction-impact review, safe deprecation, item-analysis
navigation, and "which of my assignments contain the question I just corrected".

**Assignment item pool** - a professor-facing surface over the existing selection group: draw N of M
candidates, per-item points, ordering policy. Genus: assignment content; differentia: the delivered
set differs per learner while the definition stays one object. Unblocks: practice variety beyond
seed variation, exam integrity, and reuse of large collections.

**Instructor attention queue** - a bounded, cross-course list of *actionable* items only: pending
manual grading, assignments closing, retention deadlines, corrected questions in use, imported
courses that can fast-forward. `docs/HUMAN_GUIDANCE.md` forbids a generic dashboard and explicitly
permits a separate surface for a demonstrated cross-course task; these five are that demonstration.
It contains no statistics tiles and every row links into the owning object page.

**Learner work inspection** - an authorized, audited professor view of one learner's exact issued
variant, responses, and scoring for one attempt. Genus: FERPA-scoped read; differentia: it is the
only place raw learner response text reaches an instructor outside manual grading. Unblocks the
missing gradebook drill-down: today a professor sees summary rows and cannot answer "what did Mary
actually answer".

**Course-local item analysis vs catalog item statistics** - two distinct products from one
observation stream. Course-local answers "is this item working in my class"; catalog-wide answers
"is this item working anywhere", is anonymous, and is disclosed under the existing statistics
disclosure rules. Both exist in the domain crate; neither reaches a professor page.

**Section** - a typed purpose on `course_group` (`section` vs `accommodation_group` vs
`work_group`), plus assignment audiences that target one. Genus: course group; differentia: a
learner belongs to exactly one section, and sections may carry their own schedule offsets.

```
[ideonomy * 6 moves * 3 dims * 1 organon * "professor capability architecture"]
  dim * pivot:     size - the plan is course-scaled; item-pool below and term/section/program
                   above are both unowned, and professors work at all three
  * substitution:  scale course -> term          -> course term (start/end/IANA zone)
  * substitution:  scale course -> section-set   -> typed group purpose + assignment audience
  * substitution:  scale course -> item pool     -> professor surface over selection groups
  * substitution:  autonomy professor -> system  -> scheduled close/release/notify on the job worker
  * x-domain:      theater (rehearsal)          -> rehearsal run with no educational record
  * x-domain:      theater (repertory + notes)  -> catalog-wide item statistics as flagship analysis
  organon:         dictionary - 11 entries, 7 of them coinages the current plan lacks
  not surfaced:    the student's own view of the same system (progress, what-if grade, practice
                   recommendation); the department/program scale above the course (shared sections
                   across instructors, program-level outcomes); and the professor-as-author economy
                   (ownership transfer, co-authoring, attribution when a fork outgrows its source).
                   These are negations of the plan's implicit "one instructor, one course, one term"
                   frame that the drawn dimension-prompts did not reach. Worth a follow-up tuple.
```

---

## Evidence behind the rewrite

Verified in this session:

| Capability | Schema / domain | Server route | Browser route | In current plan |
| --- | --- | --- | --- | --- |
| Assignment lifecycle, availability, due, close, late policy | yes | partial | no | yes |
| Per-student / per-group policy exceptions | `assignment_policy_exception` | partial | no | yes |
| Item pools (draw N of M) | `assignment_selection_group` | no | no | **no** |
| `attempt_selection_policy = instructor_selected` | yes | no | no | **no** |
| Manual grading | yes | `/api/attempts/{a}/manual-grade` | no | yes |
| Course item analysis | `crates/domain/src/item_analysis.rs` | `/.../item-analysis` | no | yes |
| Catalog statistics + disclosure | `crates/domain/src/statistics/` | partial | no | partial |
| Retention archive/extend/delete | yes | `/.../retention/*` | no | yes |
| Course term / time zone | **absent** | absent | absent | assumed, never defined |
| Course grade scheme / weights / letter grades | **absent** | absent | absent | **no** |
| Instructor rehearsal of an assignment | **absent** | absent | absent | **no** |
| Learner work inspection from gradebook | absent | partial | no | **no** |
| Question usage reverse index | absent | absent | absent | filter only |

Playwright coverage read as the workflow map: course creation, roster/import/export, assignment
editor policies (completion, grade policy, continued practice, variation, time limit only), catalog
browsing, authoring/publication, QTI import, repeated runs, feedback, recovery, keyboard, gradebook
summary + pagination, appearance, and the J1-J5 plus Chapter 1 walkthroughs. No test exercises a
date, a lifecycle transition, an accommodation, a pool, a manual grade, an override, an analysis
page, or a professor looking at one learner's work - consistent with the table above.

ADAPT comparison verdicts (route inventory, `OTHER_REPOS/adapt/routes/api.php`):

- **Intentional, keep the difference**: no Grader/HeadGrader/Tester roles, no learning trees,
  discussions, clickers, H5P, LMS/LTI sync, no live Alpha/Beta tether, no per-course question
  copies, no `login-as`.
- **Peptidyle design is better, keep and exploit**: shared immutable questions (enables
  catalog-wide statistics ADAPT cannot compute), server-only grading, Question IDs, issued-run
  immutability.
- **Genuinely missed**: assignment groups with weights and letter grades (`/assignmentGroupWeights`,
  `/final-grades/*`), test-student rehearsal (`/tester`, `/user/toggle-student-view`), extensions and
  score override upload at scale (`/extensions`, `/scores/upload-override-scores`), grade-by-item
  pivot (`/grading/{assignment}/{question}/...`), question usage and revision impact
  (`/non-updated-question-revisions/course/{course}`, `/question-bank/potential-questions-with-course-level-usage-info`),
  ungraded-work queue across a course (`/submission-files/ungraded-submissions/{course}`), and
  term-level bulk date shifting (`/assignments/{course}/shift-dates`, `/assignments/preview-shift-dates`).

## What changes in the plan document

Rewrite `PROFESSOR_CAPABILITY_ARCHITECTURE_PLAN.md` and relocate it with `git mv` to
`docs/active_plans/active/professor_capability_architecture_plan.md` (REPO_STYLE requires
snake_case working plans under `docs/active_plans/active/`). Keep it ASCII-only with `+-|` diagrams
per `docs/MARKDOWN_STYLE.md`; the Unicode artifact above stays in this session, not in the repo.

Sections to add or replace:

1. **Thesis section "The projection deficit"** - replaces the current three-bullet "backend
   capability without professor capability" list with the evidence table above, and states the
   design rule: a stored policy that no professor page projects is an unfinished capability, not a
   future feature.
2. **"Seven load-bearing abstractions"** - the dictionary entries, each with genus, differentia,
   ownership, mutability, and the list of workflows it unblocks. This replaces the plan's flat
   "missing shared abstractions" bullets and becomes the organizing spine of the milestones.
3. **Course spine section** - course term (start, end, IANA zone), grade scheme, effective-policy
   resolver, disclosure policy, typed group purpose. Explicitly stated as prerequisites of M2-M4
   rather than as later features.
4. **Rehearsal run contract** - instructor-owned, no enrollment, no gradebook row, no analysis
   observation, visibly labelled, reuses the existing run pipeline and question delivery. Includes
   the explicit rejection of ADAPT's test-student and login-as designs and the FERPA reason.
5. **Item pool authoring** - professor surface over the existing selection-group schema, its
   interaction with the first-issued-run lock, and its relationship to blueprints and Alpha
   assignments.
6. **Two-level analysis** - course-local item analysis and anonymous catalog-wide item statistics as
   one observation stream with two disclosure boundaries; the revision/fork/replace loop hangs off
   both.
7. **Usage index and correction impact** - reverse index contract, its search facets, its
   deprecation and correction-impact uses, and the tenant-safe aggregation rule.
8. **Attention queue** - the five actionable row types, the "actionable only, no statistics tiles"
   rule, the link-into-owning-page rule, and the citation of the HUMAN_GUIDANCE clause that permits
   it.
9. **Autonomy boundary** - which transitions the server performs on schedule (close at `closes_at`,
   release disclosure at close, retention notify/archive/delete) versus which always require a
   professor action (publish, override, delete-and-regrade, archive-early). Reuses the existing job
   worker rather than adding a scheduler.
10. **Revised milestones** - M1 becomes "Course spine and shared foundations" and absorbs term,
    grade scheme, policy resolver, disclosure policy and group purposes; M2 gains a third lane for
    rehearsal + pools; M4 gains learner work inspection and the grade-scheme-aware gradebook; M5
    gains the term-scale acceptance journey (build in Alpha, instantiate into a term with three
    sections, rehearse, teach, grade, analyze, roll over).
11. **Work packages and acceptance** - extend the WP list with the spine, rehearsal, pool,
    inspection, usage-index and attention-queue owners, and add matching acceptance criteria and
    Playwright journeys for each.
12. **Risk register additions** - grade-scheme complexity creep (mitigate: categories + drop rules
    + letter bands only, no formula language), rehearsal leakage into records (mitigate: structural
    impossibility, not filtering), attention-queue drift into a dashboard (mitigate: actionable rows
    only, hard cap on row types).

Assumptions I will record in the document rather than ask about:

- Course term is stored on the teaching course, is required before an assignment may carry an
  absolute date, and uses an IANA zone identifier.
- Grade scheme v1 is weighted categories + drop-lowest-N + rounding + letter bands. No formula
  language, no curve engine, no per-student scheme.
- Rehearsal runs are retained until the assignment changes, are visible only to their instructor,
  and never appear in exports.
- The attention queue is capped at the five row types listed; adding a sixth requires editing the
  plan first.
- Catalog-wide statistics remain anonymous and continue to obey the existing statistics disclosure
  boundary; no course, section, or learner is identifiable through them.

## Files

- `PROFESSOR_CAPABILITY_ARCHITECTURE_PLAN.md` -> `git mv` to
  `docs/active_plans/active/professor_capability_architecture_plan.md`, then rewritten in place.
- `docs/HUMAN_GUIDANCE.md` - append the settled decisions only (course term required, grade scheme
  scope, rehearsal instead of test-student, attention-queue permission and cap). Keep the owner's
  voice; engineering detail stays in the plan.
- `docs/CHANGELOG.md` - one entry under `### Decisions and Failures` and `### Additions and New
  Features` for the rewritten plan and the recorded decisions.

No production code changes in this task. The plan document is the deliverable.

## Verification

- `pytest tests/test_markdown_links.py tests/test_ascii_compliance.py` - the rewritten plan must
  pass link and ASCII gates in its new location.
- `pytest tests/` - full fast lane, to confirm the move breaks no path-referencing test.
- Manual read-back: every claim in the evidence table is cited to a file path in the document, so a
  reviewer can check it without re-deriving the inventory.
