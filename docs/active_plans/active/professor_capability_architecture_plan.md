# Plan: professor capability architecture and teaching-system roadmap

## Status

The documentation package WP-PROF-S1 was accepted on 2026-08-18 after independent acceptance review
returned ACCEPT with no P0/P1/P2 finding. The evidenced M0 release-truth packages WP-R0, WP-R1, WP-R2, and WP-PY-L1 are
accepted for this professor roadmap. The global current-package handoff is recorded only in
[implementation_status.md](../implementation_status.md). This plan owns the professor dependency
queue; it does not create a second current-package handoff. The professor track may use the shared
pre-production migration ledger while release acceptance and production activation remain open. Nothing in this track
accepts or implies live email authentication, mailbox delivery, production onboarding, deployment,
or release acceptance.

The four product decisions recorded by WP-PROF-S1 are preserved in
[docs/HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md). Conditional architecture and component ownership
remain authoritative in this plan.

## Context

Peptidyle proves a focused teaching loop today: create a course, manage a roster, assemble an
assignment from published questions, let a student practice it repeatedly, and read the gradebook.
The Playwright suite also covers catalog browsing at scale, authoring and publication, QTI import,
reuse by Question ID, course appearance, pagination, keyboard accessibility, and recovery.

The rest of the professor cycle is not yet a system:

```text
   discover --> inspect --> curate --> assemble --> rehearse --> teach --> intervene
      ^                                                                       |
      |                                                                       v
    reuse <-- revise <-- learn from evidence <---------------------------- grade
```

This document replaces the earlier root-level professor-capability roadmap and its spine rewrite.
It is the single active direction for professor capability; older versions remain only as history.
[docs/HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) is the product authority. ADAPT in `OTHER_REPOS`
is comparison evidence, never a specification.

The plan answers two questions, in this order:

1. How does Peptidyle reach the capabilities professors already expect?
2. What can Peptidyle let professors do **substantially better**, because published questions have
   stable shared identities and issued runs retain exact immutable evidence?

The second question is the more valuable one, and it drives the evidence commons in section 6.

## Thesis 1: the work divides into three classes, and only one of them is projection

An inventory across the schema, the domain crates, the store, the server routes, and the frozen
19-route browser contract in `src/route_contract.ts` sorts every gap into three classes. They carry
very different cost, and conflating them would make teams underestimate the foundational work.

- **Class P, projection**: the capability is stored and computed; no professor page reaches it. Cost
  is contract and interface work.
- **Class O, ownership**: schema exists with no code, or with code but no defined rules for who may
  change it and what it means. Cost is deciding semantics, then implementing them.
- **Class N, new architecture**: the concept does not exist anywhere. Cost is design, schema,
  domain, store, server, and interface.

| Capability | Schema | Domain or store | Server | Browser | Class |
| --- | --- | --- | --- | --- | --- |
| Lifecycle, available, due, closes, late policy | yes | yes | partial | no | P |
| Per-student and per-group policy exceptions | yes | yes | partial | no | P |
| Manual item grading and receipts | yes | yes | yes | no | P |
| Course item analysis | yes | yes | yes | no | P |
| Retention notify, archive, extend, delete | yes | yes | yes | no | P |
| Anonymous catalog statistics with k-anonymity | yes | yes | partial | no | P |
| One learner's work, opened from the gradebook | yes | partial | partial | no | P |
| Item pools, draw count, ordering, algorithm version | yes | partial | no | no | O |
| Instructor-selected attempt scoring | yes | partial | no | no | O |
| Scoring generation, recalculating and failed states | yes | yes | partial | no | O |
| Problem collections and members | yes | none | none | none | O |
| Search keywords, taxonomy, capabilities | yes | partial | partial | no | O |
| Catalog `quality_signal` column | yes | none | none | none | O |
| Entitlement: who has this assignment, and why | partial | partial | partial | no | O |
| Question assignability and lineage semantics | yes | partial | partial | no | O |
| Course term, dates, time zone | none | none | none | none | N |
| Effective policy resolver with provenance | none | none | none | none | N |
| Learner disclosure policy | partial | partial | partial | no | N |
| Course grade scheme and course total | none | none | none | none | N |
| Preview plane and rehearsal | none | none | none | none | N |
| Question usage reverse index | none | none | none | none | N |
| Improvement threads | none | none | none | none | N |

Three consequences drive the plan.

- Class P is real scope, not polish: a stored policy that no page projects is an unfinished
  capability.
- Class O must be resolved deliberately. `problem_collection`, `problem_collection_member`,
  `catalog_search_document.keywords`, and `catalog_search_document.quality_signal` have no code
  anywhere in `crates/` or `src/`. Each is adopted with defined semantics or removed; none stays as
  dead weight.
- Class N is where the foundational cost lives, and it must land before the workflows that depend on
  it. That is why the milestone order changed from the earlier roadmap.

## Thesis 2: the architecture is judged by professor actions

Every abstraction in this plan is stated with the professor action it unlocks. An abstraction that
cannot name one does not belong here.

| Abstraction | Professor action it makes possible |
| --- | --- |
| Course term and zone | "Move my whole term back five days" without editing 14 assignments |
| Effective policy resolver | "Why is Mary's copy due Friday?" answered on the page, with the source |
| Learner disclosure policy | "Release solutions when the assignment closes", set once |
| Entitlement | "Only my Thursday lab gets this assignment", without duplicating it |
| Group model | Sections, labs, cohorts, and accommodation groups without new tables each time |
| Course grade scheme | "What is Mary's grade in the course?" and a defensible export |
| Preview plane | "Show me exactly what my students will see, before anyone sees it" |
| Rehearsal run | "Let me take it myself first" without creating a student record |
| Item pools | "Everyone gets 10 of these 40, drawn fresh" |
| Usage index | "Which of my assignments use the question I just corrected?" |
| Evidence commons | "Find questions that are proven to work, not just questions that exist" |
| Improvement thread | "I noticed this item is broken; here is what I decided and what happens next term" |
| Attention queue | "What needs me today, across all my courses?" |

## Product authority and ADAPT comparison

### Deviations that are intentional and stay

- Three human roles only: Sysadmin, Instructor, Student. No Grader, Head Grader, Tester, Manager,
  or Publisher. Co-instructors are ordinary approved Instructors.
- Shared immutable published questions, not per-course question copies.
- Server-only grading and answer material.
- Human-readable references; never a UUID in visible content, navigation, or copyable links.
- Courses are the instructor home workspace; no generic dashboard without a demonstrated
  cross-course task.
- No learning trees, discussions, clickers, H5P, LMS roster synchronization, or LTI in this scope.
- No live Alpha/Beta tether that can silently alter an active teaching course.
- Privacy-first retention; a sysadmin gets no general course access.

### Gaps ADAPT exposed that this plan adopts, in Peptidyle form

- Assignment groups with weights and letter grades -> course grade scheme, derived below from
  Peptidyle's own practice-first workflows rather than copied.
- Test student, student view, `login-as` -> preview plane and rehearsal runs, with no fake
  enrollment.
- Extensions and bulk score override -> entitlement and accommodation pages plus explicit
  assignment-grade overrides.
- Grade by item across students -> manual-grading queue with a by-item pivot.
- Question usage and replacement impact -> usage index and explicit replacement impact.
- Course-wide ungraded work -> instructor attention queue.
- Term-level date shifting with preview -> term shift through the preview plane.
- Assignment templates and course import -> blueprints, Alpha courses, rollover.

### Where Peptidyle is structurally stronger, and this plan presses the advantage

Because questions are shared and immutable, and issued runs retain exact `(ProblemId, VersionId)`
and seed evidence, Peptidyle can compute things ADAPT cannot compute at all: cross-course item
behavior and comparison of explicitly linked replacement questions with their sources. Section 6
turns that into professor capability rather than a statistics page.

## Design philosophy

Apply **Fix the design, not the symptom**, **Design for adaptability**, **Long-term over
short-term**, and **Dream big**. Peptidyle is pre-production with no durable data, so foundational
schemas change directly and carry no compatibility readers.

Ownership tree; each level adds context without changing the layer beneath it:

```text
Question ID -> personal collection -> assignment blueprint -> Alpha course assignment
            -> teaching-course assignment -> issued student run
```

Shared questions stay immutable publications. Reusable curriculum stays answer-free current state.
Teaching assignments stay mutable current state governed by issued-run evidence. Student runs retain
exact immutable snapshots.

Four rates to optimize: minutes (find, inspect, save, add, rehearse), weeks (schedule, accommodate,
grade, intervene), terms (clone, shift, run sections, improve a curriculum), years (attribution,
accumulated evidence, cross-instructor reuse).

## 1. The dependency spine

```text
+---------------------------------------------------------------------------------+
| L4  Evidence and improvement                                                     |
|     catalog statistics | usage index | improvement threads | attention queue      |
+---------------------------------------------------------------------------------+
| L3  Reuse                                                                        |
|     blueprints | Alpha courses | clone | rollover | term shift | fast-forward      |
+---------------------------------------------------------------------------------+
| L2  Teaching operations                                                          |
|     lifecycle | entitlement | accommodations | pools | rehearsal | grading         |
+---------------------------------------------------------------------------------+
| L1  Course spine                                                                 |
|     term + zone | effective policy | disclosure | groups | entitlement | grade     |
|     scheme | preview plane                                                       |
+---------------------------------------------------------------------------------+
| L0  Shared identity and content                                                  |
|     Question ID | published version + lineage | public byline | typed references   |
+---------------------------------------------------------------------------------+
```

Reading rule: no L2, L3, or L4 capability re-derives an L1 concept locally. A page that needs to
know whether work is late asks the resolver; it never compares timestamps itself.

## 2. Course spine: concepts the earlier roadmap assumed but never owned

The first roadmap promised relative schedules, daylight-saving refusal, accommodations, audiences,
rollover, and analysis while three concepts underneath them had no owner. A second pass over the
schema and the store found four more.

### 2.1 Course term

The teaching course gains a term: start date, end date, and an IANA time zone. Absolute assignment
dates are accepted only once a term exists. It anchors relative curriculum schedules, rollover, term
shift, daylight-saving validation, retention timing, and archive labelling. `public.course` stores
only a title today.

### 2.2 Effective assignment policy (see section 3)

### 2.3 Learner disclosure policy (see section 3)

### 2.4 Entitlement: derived authority, materialized receipt

`public.enrollment` is per **assignment**, not per course. It is normalized durable evidence: the
row binds the assignment, course, stable learner identity, membership episode, first materialization
time, purpose, evaluator version, and actor-or-rule authority. One basis row and the evaluator's
applicable policy-scope rows are inserted with it and sealed before commit; the compact summary is
typed relational state rather than a JSON authority. Current access is never read from this receipt.

Settle it as a split, because neither pure derivation nor pure persistence is correct:

- **Authority is derived.** "Is this learner entitled to this assignment right now" is computed from
  roster state, audience, group membership, and lifecycle. It is never read from a stored row, so an
  audience edit and a roster change cannot silently disagree.
- **Materialization is persisted.** The enrollment record is created at the first entitlement-bearing
  event (first run start, first grade-bearing action, or an explicit instructor issue), and it
  carries provenance: what granted it, when, and by which actor or rule.
- **Revocation ends authority without erasing evidence.** A revoked or removed learner loses derived
  entitlement immediately; the materialized record and its issued runs remain as educational
  evidence under the retention policy, and the gradebook shows the row as revoked rather than
  deleting it.
- **Historical runs bind to the materialized record**, so a later audience change never orphans
  issued work.

Interactions to prove in conformance tests: roster removal with issued runs; audience narrowed after
issue; group membership changed mid-term; assignment unpublished after issue; learner revoked and
re-invited at a new address; and a materialized record whose derived authority is now false.

### 2.5 Group membership model (see section 5)

### 2.6 Question assignability and lineage

`catalog_search_document` already stores `lifecycle`, `lifecycle_reason`, `previous_version_id`,
`derived_from_problem_id`, and `derived_from_version_id`. The roadmap says assignments reference
"currently assignable" questions without owning what assignability means, who may change it, and
what happens to assignments already using a question that becomes deprecated. This plan makes
assignability an explicit contract and ties deprecation to the usage index and the attention queue.

### 2.7 Scoring generation and regrade

`assignment.scoring_generation` and `scoring_status` (`current`, `recalculating`, `failed`) exist,
plus staging tables and a worker. Regrade is therefore already a state machine, but no professor
surface shows it. A professor who removes an item, applies an override, or corrects a question needs
to see that scores are recalculating, and to see failure honestly rather than reading a stale total.

### 2.8 Assignment points model

An assignment's total points are derived from fixed items plus pool draws (`points_per_item` times
`draw_count`), with `scoring_mode` values `normal`, `full_credit`, `extra_credit`, and `excluded`
already defined per item. The professor-facing points model is the derived total and its extra-credit
share, shown while editing. The grade scheme in section 4 consumes this and must not redefine it.

### 2.9 Orphan schema decisions

- `problem_collection` and `problem_collection_member`: **adopt**. They match the collections design
  and already carry `visibility` of `private`, `institution`, or `public`. Version 1 exposes private
  and institution; public collections wait until an owner needs them.
- `catalog_search_document.keywords` and `taxonomy`: **adopt** as the discovery vocabulary, with the
  assisted-tagging pathway in section 6.5.
- `catalog_search_document.quality_signal`: **adopt** as the evidence-derived ranking input in
  section 6.1, with its computation defined; otherwise remove it rather than leaving an unexplained
  ranking column in the schema.

## 3. Policy composition as a first-class design problem

Base policy, lifecycle, entitlement, sections, accommodations, individual exceptions, late rules,
and disclosure become a mess unless one model orders them. This is that model.

### 3.1 Gates and modifiers

Two kinds of input, resolved in a fixed order. Gates decide whether an assignment exists for a
learner at all; modifiers shape the window and limits.

```text
   GATES (hard, evaluated first, cannot be widened by a modifier)
   G1 lifecycle        draft | published | closed | archived
   G2 entitlement      roster state + audience + group membership   (owned by WP-PROF-S5)
   G3 authorization    tenant, membership, revocation

   MODIFIERS (most specific non-null wins; each carries its source)
   M1 base assignment policy      available / due / closes / limits / late rule
   M2 section or group offset     purpose-scoped schedule shift
   M3 group accommodation         extend-only by default
   M4 individual exception        extend-or-override, explicitly flagged
   M5 late rule                   accept | reject | mark_late, applied last
```

Resolution rules, stated once so no surface invents its own:

- A gate that denies ends resolution. No modifier can grant access that a gate denied.
- **Ownership split, so two packages cannot both claim the verdict**: the entitlement component
  (`WP-PROF-S5`) owns the entitlement decision and its reason, and is the only place that reads roster
  state, audience, and group membership. The resolver (`WP-PROF-S3`) *consumes* that decision as gate G2
  and owns everything downstream of it: window, limits, lateness, and per-field provenance. A caller
  asks the resolver once and receives both, so no surface composes them itself.
- Modifier precedence is M4 > M3 > M2 > M1 per field, never per record: an exception that sets only
  a close time leaves the time limit resolved from the lower layer.
- Accommodations are extend-only unless the professor explicitly marks an override, so the common
  case cannot accidentally shorten a learner's window.
- The resolver returns, for every field, the value **and** the layer that produced it. Professor
  surfaces show that provenance in plain language: "Closes Friday 23:59 (Mary's extension)".
- Lateness is computed once, by the resolver, from the effective close time and the late rule.

### 3.2 Disclosure

Disclosure is the same shape at read time: one assignment-owned policy stating what a learner may
see and when, evaluated server-side for every learner-facing projection.

```text
   score | per-item correctness | feedback text | correct answer or solution | class statistics
   x
   during attempt | after submit | after due | after close | never
```

The migration removes the former `feedback_disclosure` authority. `feedback_release` records remain
immutable, retention-fenced audit evidence of an instructor action; they are never a policy input
and cannot unlock or otherwise change a learner projection.

### 3.3 Student projection and access evidence

S4 owns the learner-facing projection of the S3 disclosure result, but it does not own entitlement
or policy resolution. Learner projections include assignment lists, run lists and details, attempt
start, submission feedback, summaries, and other learner-facing grade or progress views. They do not
include instructor roster or gradebook pages.

Separately, the browser uses one centrally derived, fail-closed role boundary before instructor
components or transport requests mount. A student session reaching any instructor-only route,
including roster and gradebook, is denied before instructor transport. Direct route probes and
no-transport assertions are required because a screenshot cannot prove authorization. The same
boundary governs direct navigation and in-app links; it is not a learner projection or a second
entitlement/policy authority.

Permanent visual evidence uses the exact CSS-pixel matrix 1280 by 800 (16:10), 800 by 1280 (10:16),
393 by 852 (iPhone Pro aspect), and 800 by 800 (square), with planning weights of 40%, 30%, 20%, and
10%.
Professor evidence remains desktop at 1280 by 800 or larger. Student evidence includes an allowed
student surface and the visible denial of instructor-only routes. The committed corpus is organized
under `docs/screenshots/` by instructor, student, and the student/access boundary, with
`tests/playwright/ui_corpus_manifest.ts` as its sole screenshot ownership authority.

Live evidence uses local-development credentials or invitations because email is unavailable; it
must not claim email delivery. Fictional deterministic fixture addresses in `example.invalid` are
permitted test data, while real email and identifying records remain prohibited. Public and private
evidence remain separate. The accepted S4 evidence includes fresh capture, native-size inspection,
manifest/provenance verification, and direct no-transport route proofs for all four viewports.

### 3.4 Derived state versus durable transitions

The repository already answers this, and the plan follows its convention rather than inventing a
second one. `feedback_release` and the retention dispatch, stage, and notification tables persist
*receipts of things that happened*, while decisions themselves are computed. State the rule once:

- **Everything the resolver answers is derived from (policy, now).** "Closed", "late", "available",
  "entitled", and "what may this learner see" are computed at read time. No job writes them.
- **`assignment.lifecycle` is professor intent, not clock state.** `draft`, `published`, `closed`,
  and `archived` record what the professor decided. "Closed right now because the close time passed"
  is derived, and the interface shows both plainly: "Published, closed since Friday 23:59".
- **The worker owns only effects that cannot be derived**: notifications, retention purges and their
  manifests, statistics contributions and aggregate refresh, score recalculation runs, and export
  artifacts. Each is an existing job family with a real handler and atomic committer.
- **Every worker effect writes a durable receipt**, so recovery after a crash re-derives decisions
  and skips completed effects. No worker outage can change a professor-visible verdict, only delay a
  side effect.

This removes the ambiguity that would otherwise appear between lifecycle, preview, late handling,
and worker recovery: preview and production compute identically, because both call the resolver.

### 3.4 One consumer contract

All of the following read the resolver and the disclosure policy and nothing else: learner
assignment list, run start authorization, run summary, gradebook cells and totals, accommodation
editor, entitlement preview, clone and term-shift previews, attention-queue deadlines, and export.
A conformance suite proves that each surface returns the same decision for the same inputs.

## 4. Course grade scheme, derived from Peptidyle's workflows

The need is real: `assignment.gradebook_included` already anticipates a course total,
[docs/HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) requires manual LMS grade export to be practical,
and per-assignment scores cannot answer the question professors actually ask. The **model** is
derived from what Peptidyle is, not from ADAPT's category weights.

Peptidyle's distinguishing facts: repeated practice is a feature, not an exception; a first
completion or a perfect score must not end practice; assignments already carry a completion policy
and an attempt selection policy; items already support extra credit and exclusion.

Version 1 ships two aggregation modes, one selected per course:

1. **Total points** (default, no configuration): sum of included assignment scores over sum of
   points possible, using each assignment's own attempt-selection policy.
2. **Weighted categories**: ordered categories, each with a percentage weight and a set of
   assignments; optional drop-lowest-N inside a category.
3. **Completion-based (deferred)**: a possible later-package mode would count assignments meeting
   their own completion policy over a required count. It is retained here as design work only; it is
   not part of the shipped S6 selector, domain enum, migration, Store contract, or HTTP API.

Completion mode needs its interactions settled before it ships, because "count completed over
required" meets several existing features:

| Interaction | Rule |
| --- | --- |
| Assignment excluded from the gradebook | not counted, and not part of the required count |
| Assignment marked extra credit | may count above the required count, never toward it |
| Late work under `mark_late` | counts as complete; lateness is reported, not scored away |
| Late work under `reject` | not complete, because no submission was accepted |
| Instructor-selected run | completion is evaluated against the selected run |
| Assignment grade override | override sets completion explicitly and says so on the row |
| Required count exceeds available assignments | refused at configuration time with a clear message |

Total points and weighted categories are the two shipped modes. Completion mode is deferred: lane C
keeps the three representative-course examples (a pure practice course, a mixed practice-plus-exam
course, and a course that adds an assignment mid-term) as later-package design work. If any example
needs a rule outside the table above, completion remains a separate package.

Nothing downstream may assume completion mode exists. `WP-PROF-S6`, the gradebook, the course export,
`WP-PROF-G2`, and the M6 journey are scoped to the two shipped modes; the course export names the mode
that produced the file. If completion mode ships later it adds a mode to an existing selector and
changes no consumer contract.

All modes share: an explicit rounding rule reusing the existing score rounding decisions, optional
letter bands, an included/excluded flag per assignment (`gradebook_included`), and one course-level
export that states its mode and rounding inside the file.

Out of scope, and stated as non-goals: formula languages, curve engines, per-student schemes, and a
student-facing what-if calculator.

## 5. Groups: general model, section semantics on top

A section is useful; "exactly one section per learner" is too narrow for cross-listed courses, labs
that cut across sections, cohorts, and accommodation groups. The model stays general and puts the
constraint in policy.

- `course_group` gains a typed `purpose`: `section`, `lab`, `cohort`, `accommodation`, `work`.
- Membership stays many-to-many. Nothing in the schema forbids a learner in a section and two labs.
- Each purpose declares capabilities: may it carry a schedule offset, may it be an assignment
  audience, may it carry accommodations, is it learner-visible.
- Exclusivity is a course-level rule per purpose (default: sections warn on multiple membership,
  labs and cohorts do not), enforced in the command layer with a clear message, never by a schema
  constraint that a future cross-listing case would have to fight.
- Roster import can create and populate groups, since section membership usually arrives with the
  roster.

## 6. The evidence commons: what Peptidyle can do that ADAPT cannot

Published questions have one identity everywhere they are used. Issued runs retain the exact version
and seed. Fork lineage is already stored. Statistics aggregation and contribution receipts already
exist. That combination supports six professor capabilities no per-course-copy product can offer.

### 6.0 Validity contract, before any statistic is shown

The evidence commons is only worth building if its numbers are defensible, so the validity rules
come before the features. Peptidyle already owns the first one:
`crates/domain/src/statistics/disclosure.rs` suppresses every view below the configured k-anonymity
minimum cohort size, and suppresses discrimination separately when the scored cohort is too small.
This plan extends that contract rather than inventing a parallel one.

- **Comparable observations only.** An aggregate combines observations of the same exact internal
  `(ProblemId, VersionId)` under comparable delivery: same response family, and item scoring not
  excluded. A comparison is only between explicitly linked replacement questions and their source;
  their observations are stated separately and never blended.
- **Independence.** Repeated practice is a product feature, so repeated attempts by one learner are
  not independent observations. Difficulty and discrimination use the learner's first scored attempt
  per issued run; attempt counts and time are reported separately as behavior, not as difficulty.
- **Context is disclosed, not hidden.** Every figure carries cohort size and course count. A
  cross-course figure never claims causal comparability between courses; it describes the pooled
  cohort.
- **Insufficient evidence is a first-class answer.** Below threshold the interface says
  "insufficient evidence" and offers no ranking contribution, no comparison, and no flag. It never
  substitutes a weak estimate.
- **No professor-level or course-level identification.** Aggregates never reveal which course
  produced which observation, and a figure computed from a single course is suppressed at the
  cross-course boundary.
- **Explainability over formula tuning.** Any composite is shown decomposed into its inputs. The
  exact weighting is a tunable; the disclosed inputs and thresholds are the contract.

### 6.1 Discover questions that are proven, not merely present

Search ranks on relevance today. With cross-course evidence, discovery can also answer "has this
worked elsewhere". `catalog_search_document.quality_signal` is the existing home for this. It is
computed only from disclosed aggregates that pass section 6.0, and it is always shown decomposed
with its sample size: "used in 14 courses, 612 first attempts, difficulty 0.62, discrimination
0.42", never a bare score. Questions with insufficient evidence are neither promoted nor penalized;
they rank on relevance alone and say so.

### 6.2 Understand where a question is used, before changing anything

The **usage index** maps a published question to the assignments, courses, collections, and Alpha
curricula that reference it, with tenant-safe aggregation: an author sees counts and their own
courses by name, never another instructor's course. It powers the "used in my courses" facet, an
explicit replacement-impact review, safe deprecation, and analysis navigation.

### 6.3 Measure whether a replacement actually helped

Attempts carry the exact internal version evidence for the question that was issued. A correction
publishes a distinct replacement question with a new Question ID, while the source keeps its
evidence. Peptidyle can therefore compare explicitly linked source and replacement questions under
the section 6.0 rules: each question must independently clear the disclosure threshold, the
comparison names both Question IDs and cohort sizes, and it is presented as a description of two
cohorts rather than a causal claim. Where the cohorts differ materially in course composition, the
comparison says so instead of reporting a difference. No publication or background action moves an
assignment; an Instructor explicitly chooses a replacement. ADAPT cannot compute this at all,
because each course holds a private copy. This makes a replacement a measurable act rather than a
hopeful one.

### 6.4 Compare a linked replacement with its source

Explicit immutable provenance records lineage. A replacement question and its source can be compared
on disclosed aggregates under the same rules, so an instructor choosing between them sees evidence
rather than titles, and an author learns whether the replacement improved on its source.

### 6.5 Make the corpus discoverable with assisted tagging (separable package)

Discovery fails when questions are untagged, and hand-tagging a large corpus does not happen in
practice. Assisted tagging closes that gap, and it is deliberately **not on the critical path**: the
architecture must succeed with human-managed taxonomy alone. It is scoped as one optional package
(`WP-PROF-D3`) that depends on discovery contracts and that nothing else depends on, so it can be
deferred, run late, or dropped without touching the teaching system. Its cost is model execution,
provenance, operator policy, batching, and a confirmation interface; that cost buys corpus
discoverability and nothing else in this plan.

Boundaries, if it is built:

- Input is **public published question content only**: prompt, choices, title, family. Grader-only
  material, answer keys, private author source, and learner responses are never sent.
- Output is a **proposal**, never a publication: suggested taxonomy terms and keywords, each with a
  confidence and the model identity that produced it.
- An author, or a curator for their own corpus, confirms or rejects each proposal. Confirmed tags
  carry provenance (model, version, date, confirming user); rejected proposals are not re-proposed.
- It runs as a worker job family with a real handler and atomic committer, in batches, offline from
  the request path. A local model through the existing operator-selected runtime is acceptable; an
  external API requires a recorded operator decision because it discloses public content to a third
  party.
- It writes only to the existing `keywords` and `taxonomy` fields. It never writes question content,
  points, difficulty, or answers.

This is metadata assistance, not generated educational content; the standing non-goal on generated
content is unchanged.

### 6.6 Give authors the feedback loop that improves the corpus

An author sees disclosed aggregate evidence for their own published questions: where behavior looks
wrong, which items are widely adopted, and whether their explicitly linked replacements helped. That closes the loop
between authoring and teaching across the whole installation, which is the long-term reason the
shared-immutable-question design exists.

## 7. From analysis to action: the improvement loop

Item analysis must not terminate at a chart. The loop is the product.

```text
   notice ----> inspect ----> understand reach ----> act ----> decide ----> carry forward
   flagged      learner       usage index and        fork,     recorded    next term's
   item in      work and      catalog evidence       publish   decision    assignment or
   my course    distribution                         replace,  with        Alpha shows the
                                                     retire,   reason      decision
                                                     keep
```

- **Notice**: course-local item analysis flags items by difficulty, discrimination, distribution
  shape, and time, scoped to the course and readable only by its instructors.
- **Inspect**: learner work inspection opens one learner's exact issued variant, responses, timing,
  and scoring, under an audited authorization written to the existing `record_access_log` and
  `audit_event` tables. This is the missing gradebook drill-down.
- **Understand reach**: the usage index and catalog statistics say whether the problem is local to
  this class or visible everywhere, which is the difference between "my teaching" and "this item".
- **Act**: open the author workspace when owned, publish a distinct linked replacement when a content
  change is needed, deliberately replace the item in a future assignment, or retire it from a
  collection or blueprint. Issued evidence is never altered.
- **Decide and carry forward**: an **improvement thread** records the decision and attaches to the
  question and to the assignment or blueprint, so the reason a question was replaced survives the
  professor's memory.

The improvement thread is small on purpose. Its complete lifecycle is fixed here so it cannot grow
into a second task-management system:

- **Fields**: subject (question plus optional assignment item), observation, state, action taken,
  reason, actor, created and resolved timestamps. Nothing else.
- **States**: `open` -> `resolved` (with one action: replacement published, forked, replaced,
  retired, kept) or
  `dismissed`. No reopening; a later concern is a new thread.
- **Ownership**: the instructor who created it. Co-instructors of the same course may resolve it.
- **Visibility**: course instructors always; the question's author sees an anonymized existence
  signal only when the thread resolves to replacement published or forked, because that is feedback about their
  published question.
- **Propagation**: a clone or rollover copies resolved threads as **read-only annotations** on the
  affected item. Copies never re-open, never accumulate a chain, and never travel to a third
  generation; the annotation records the origin course reference and date.
- **Attention queue**: only `open` threads older than one term boundary appear, and each has exactly
  one action, "resolve".
- **Explicit non-features**: no assignees, comments, attachments, due dates, priorities, or
  notifications.

## 8. Preview plane and rehearsal

Rehearsal generalizes. The underlying capability is answering "what would X see, under policy Y, at
time Z", without mutating anything.

- **Non-mutating previews**, all built on the resolver and disclosure evaluation:
  - schedule table: every learner or group and their effective window and limits, with sources;
  - accommodation effect: this learner before and after the exception;
  - disclosure state: what a learner sees now, at due, at close;
  - entitlement: exactly who has this assignment and why;
  - pool draw sample: a representative draw with its algorithm version;
  - clone and term-shift previews: resolved dates before committing, with DST refusal.
- **Rehearsal run**: the one mutating case, an instructor-owned run against a teaching or Alpha
  assignment. It reuses the run pipeline, delivery, timing, and rendering, and is structurally
  incapable of producing an enrollment, gradebook row, item-analysis observation, catalog
  contribution, or export row. It is labelled as rehearsal everywhere, visible only to its
  instructor, and discarded when the assignment definition changes.

**Preview subject.** A rehearsal never impersonates a learner. Its subject is a `PreviewSubject`
value: group memberships, policy modifier values, and a chosen moment. It is built in one of two
ways:

- **Synthetic**: the professor picks the groups and modifiers directly ("a Thursday lab member with
  a 48-hour extension, at 09:00 next Monday").
- **Derived from a learner**: the resolver produces that learner's effective policy, and only the
  resolved values and their layer names are copied into the subject. No user identity, roster
  identity, email, or record reference travels into the rehearsal, and the label reads by role, not
  by name.

This keeps the realism that makes rehearsal worth running while keeping FERPA identity out of an
instructor-owned synthetic execution. Deriving a subject from a learner is itself a record read and
is audited like any other.

This replaces ADAPT's test student and `login-as`, which create a fake enrollment inside the FERPA
record set and then filter it out downstream. Structural impossibility is the safer contract.

### WP-PROF-LD1 live-demo installation lifecycle contract

WP-PROF-LD1 implements only the durable lifecycle that makes the approved
[live-demo specification](../../LIVE_DEMO_SPEC.md) an ordinary PLE installation with seeded baseline
data. It is allocated `2026081808_live_demo_install_state.sql` in the shared
[implementation-status registry](../implementation_status.md). The package creates one durable
installation state with only `installing` and `complete` states and takes one advisory lock for
single-writer first-install coordination.

While the state is `installing`, deterministic Base Course seeding is resumable after an
interruption and retries reuse the same generation-bound storage receipt. A fresh PostgreSQL and
object-storage pair is required for this path. A retained `complete` pair starts normally without
seed writes, storage inspection, or equality scans. A pre-marker database or mixed database/storage
pair fails closed and directs fresh regeneration of both stores; no partial baseline is adopted as
retained live data. Fresh database and storage regeneration restores the baseline.

LD1 owns the migration and live lifecycle evidence for first install, interruption/resume, retained
restart, fail-closed mixed-state handling, and fresh regeneration. `learning-data-access` is the
sole SQL, PostgreSQL-lock, durable install-state, migration, and Store owner. It does not add
account, demo persona, role, session, passkey, authentication, origin, or replica behavior or
schema. WP-RC8 retains those account and security boundaries. The Base Course itself is ordinary
live data after provisioning.

The focused product crate `crates/base-course-installation/` (`base_course_installation`) owns the
narrow typed request/receipt API, ordinary Base Course recipe, and deterministic installation
orchestration. `project-tools` is only the direct `cargo tools base-course` CLI adapter; the product
crate has no HTTP route or server-start hook. The baseline recipe, install-state transitions, and
command contract are product-crate owned.

Evidence stays KISS: pure product-crate tests cover typed request, receipt, recipe, and deterministic
convergence; the existing LDA PostgreSQL live oracle covers schema and lock behavior; and the existing
`tests/e2e/e2e_live_demo_baseline.py` covers the connected lifecycle. LD1 does not add a second
product-specific PostgreSQL harness or an exhaustive live matrix.

### WP-PROF-LD2 seeded demo entry contract

WP-PROF-LD2 follows accepted WP-PROF-LD1 and the
necessary existing WP-RC8 account-session/passkey/origin contracts. LD2 can implement and validate
the seeded-entry seams against those contracts while unrelated WP-RC8 provider, mailbox,
multi-replica, security, and HCI gates remain open. It adds a deployment-controlled Student/Instructor
persona selector that resolves only to seeded ordinary accounts and then follows the normal
account-session and course/role-selection path. It extends, rather than replaces, WP-RC8's production
authentication boundary. Its selector behavior and claim, passkey, account, and session data and
semantics remain non-schema. A completed boundary review allocated `2026081809` for exactly two
least-privilege execute-only PostgreSQL brokers: safe normal Sysadmin approval-candidate discovery,
and a read-only completed live-demo installation-generation lookup that binds the configured
first-ownership proof. The generation-read broker is the narrow schema authorization seam for that
otherwise non-schema ownership flow; it grants no role and writes no lifecycle, identity, passkey,
or session state. The separate `2026081810` allocation is only for the
discovered Student pre-tenant account-course context repair: it must retain active Student contexts
without disclosing archived, deleted, or started-retention course records, leave Instructor behavior
unchanged, and prove connected Student login. Sysadmin remains a normal account-ownership and passkey
flow with full ordinary Sysadmin capabilities.

The approved live-demo handoff is `WP-PROF-LD1` -> `WP-PROF-LD2` -> `WP-PROF-BS1` ->
`WP-PROF-T3` -> `WP-PROF-T4`. LD2 was accepted on 2026-08-21. The sole current-package handoff
is recorded in [implementation_status.md](../implementation_status.md).

`WP-PROF-BS1` now replaces the parallel mock-backed browser application and separate
screenshot/browser owners with one canonical disposable real-stack suite. It establishes the
production `dist/` and HTTPS gateway path before T3 browser acceptance. T3 retains its frozen scope
as the planned successor after BS1; its focused active plan owns the execution detail.

### WP-PROF-T3 preview contract

WP-PROF-T3 implements the non-mutating preview plane. The only permitted durable effect is one
private `audit_event` atomically appended after a successful learner-derived subject construction.
WP-PROF-T4 alone owns the later rehearsal run. T3 does not create or change an enrollment, run,
attempt, receipt, gradebook row, analysis observation, catalog contribution, export, job, session,
membership, retention record, or preview record.

T3 implements the currently ready preview families:

- an Instructor-only entitlement and schedule inspection table;
- an accommodation comparison with Before and After effective values;
- disclosure projections at Now, Due, and Close; and
- entitlement evaluation with safe reasons and effective-policy provenance.

The contract leaves typed extension seams for the WP-PROF-T5 pool-draw sample and the later
WP-PROF-B2 clone and term-shift preview. Those packages are not implemented or accepted by T3.

**Inspection and subject boundary.** The exact-course, direct-Instructor inspection plane may show
safe `M-` references and display labels in its schedule or entitlement table. It is a FERPA-authorized
diagnostic view, not a preview subject or a rehearsal input. At an authenticated route boundary, an
instructor can select one active learner reference and derive a `PreviewSubject`. The Store resolves
the learner, assignment, entitlement, effective policy, and required prior-run facts in one writable
repeatable-read snapshot; it atomically records the private record-read audit and returns only the
sanitized subject. The learner reference is discarded before serialization.

`PreviewSubject` is a closed, ephemeral, self-contained serializable value. It contains only its
synthetic or derived kind, bounded group-role and group-purpose facts, compatible modifier values,
resolved value/source-layer labels, selected moment, assignment reference revision, and any bounded
prior-run fact required by policy. It contains no `U-`, `M-`, `CI-`, or `PV-` locator; `UserId`,
`StudentId`, membership ID, name, email, UUID, history, answer, score, audit ID, enrollment, run, or
attempt reference. A derived label names a role or layer, never a person. The subject is neither
persisted nor signed: it conveys only hypothetical values and grants no authority. Caller-controlled
source labels grant no rights. The assignment/reference revision and selected moment bind evaluation;
stale revision requests fail instead of silently reusing a subject.

Synthetic construction validates active course-local groups, group purposes, and compatible modifier
combinations. It cannot supply arbitrary identifiers or assert learner entitlement. Derived
construction alone crosses an `M-` reference, only after exact-course direct-Instructor authorization
and assignment-to-course binding. Denied, malformed, foreign, inactive, or unauthorized derivation
does not append an audit. The successful audit is private and checksum-protected: it records actor,
course, assignment, internal target, action, schema version, and no student name, email, public
reference, group membership, policy value, entitlement outcome, answer, response, feedback, or score.

**Evaluation and transport.** The server owns the ordered S5 entitlement, S3 effective-policy, and
S4 disclosure evaluation with authoritative course-zone time. The browser renders a strict closed
projection and never reconstructs entitlement, precedence, timing, or disclosure. Every route is
`no-store`, uses strict closed allowlists, authorizes the direct Instructor and course/assignment
binding before body or learner-reference parsing, and conceals protected data on denial. A denial
union carries no resolved time, policy, provenance, disclosure, or subject metadata.

**Instructor task model.** The professor enters from an assignment to diagnose delivery. A persistent
"Preview only - no learner work or grades are created" cue remains visible while the professor scans
the schedule/entitlement table, derives a role-only subject or constructs a synthetic subject,
compares accommodation Before and After, and scrubs Now, Due, and Close disclosure moments. The page
shows safe provenance and explicit shown/withheld text. Failures and stale revisions preserve the
hypothetical draft and provide a focused retry or reload. The route is keyboard-complete, compact at
1280x800, and responsive at the project corpus widths. Learner and outsider direct navigation mounts
no protected transport.

**Installed Base Course.** After WP-PROF-LD1 accepts its lifecycle contract, every standard fresh
installation provisions the persistent simulated Base Course through the ordinary migration and
first-run setup path. Its fictional Instructor, existing students, assignments, attempts, and grades
exercise the same PostgreSQL, RLS, server, and browser boundaries used by an ongoing course. T3
connected acceptance selects its derived learner from those persisted course memberships and
preserves the course across repeated check-ins. Focused test-double coverage remains a subordinate
engineering lane.

**Schema and acceptance.** T3 receives no migration allocation. It reuses forced-RLS `audit_event`
and the existing writable repeatable-read snapshot; accepted
`2026081807_teaching_operations.sql` remains immutable. Acceptance requires:

| Layer | Required evidence |
| --- | --- |
| Domain and qmodel | Closed subject/result types reject identity and answer-bearing fields; S5 -> S3 -> S4 parity, revision refusal, source labels, denied union, and disclosure moments are covered. |
| Memory | Direct-Instructor derivation creates exactly one PII-minimal audit and no other state change; synthetic construction creates none; authorization, foreign, inactive, malformed, and denied paths create none. |
| PostgreSQL live | Fresh baseline proves forced RLS, atomic audit snapshot, checksum and PII-free payload, concealment probes, and table-count proof of zero enrollment/run/attempt/grade/export/job mutation. |
| Server | Authorization precedes decode and lookup; exact-course binding, `no-store`, strict decoders, denial allowlist, and no identity/answer/score/audit transport are covered. |
| Browser | A real-stack Instructor journey covers schedule scan, derived and synthetic subjects, Before/After, Now/Due/Close, recovery, and keyboard behavior; Playwright, accessibility, and fresh screenshots cover the required viewports and direct-route no-transport denial. Test-double tests remain subordinate and do not count as connected acceptance. |
| Independent review | Architecture, security/privacy, HCI, and documentation/evidence reviewers find no unresolved P0--P3 issue. |

The existing named T2 policy-preview remains an Instructor teaching-operations inspection surface.
T3 does not rebrand, remove, or use it as its identity-free subject contract.

## 9. Discovery, curation, and assembly

- Repair the current search contract: full-text plus trigram matching; exact Question ID first, then
  relevance, then similarity; a stable opaque relevance cursor; disclosed statistics rendered on the
  detail page. Both gaps are promises the active release plan already made.
- Extend filters: author byline, response family, backend, tag, taxonomy, license, capability,
  disclosed evidence, and course usage.
- Adopt `problem_collection`: private and institution visibility, one built-in Favorites collection
  per Instructor, flat named collections, revision-checked.
- `SavedProblemSearch` stores a normalized query, never a frozen result list.
- One `ProblemPicker` serves every source: catalog, my published questions, a collection, retained
  course definitions, and Alpha curricula. Library, assignment editor, blueprint editor, and Alpha
  authoring all use it, so selection behavior and metadata vocabulary cannot diverge.
- **Item pools**: project the existing selection-group schema through the assignment editor. Draw N
  of M, per-item points, ordering policy, stored algorithm version, filled directly from a
  collection. Pools obey the first-issued-run rules and appear in the preview plane.
- Add a `PublicInstructorByline` with a typed public reference and approved display name, carrying
  no UUID, email, roster identity, or private institution record.

## 10. Reusable curriculum

- Alpha courses are a separate shared-curriculum aggregate, not a kind field on the FERPA-bearing
  course. Public to approved Instructors, human route such as `AC-123`, creator-only editing,
  inspect/fork/instantiate for everyone else.
- Students cannot join, receive assignments, run, or generate grades, because Alpha records have no
  relationship to teaching membership or activity tables.
- An Alpha stores an ordered curriculum with module or week labels and calendar-relative
  availability, due, and close defaults, using calendar days and local wall-clock values.
- Alpha assignments may carry **evidence context**: the disclosed catalog statistics for their items,
  so an instructor adopting a curriculum sees expected difficulty before teaching it.
- Cloning copies definitions, policies, theme defaults, and reviewed offsets. It never copies
  students, invitations, groups containing students, accommodations, runs, responses, grades,
  retention state, or co-instructors. It requires the target course term and previews every resolved
  date.
- Each imported assignment records its source and a normalized baseline manifest. Untouched
  definitions before the first issued run may fast-forward; diverged ones offer side-by-side selected
  copying and never an automatic merge. Delivery dates and accommodations are teaching-owned.
- Improvement threads travel with the curriculum, so an instructor adopting an Alpha sees why an item
  was replaced last term.
- Term shift applies one offset across a course with full preview and the same DST validation.

## 11. Teaching operations, grading, and the learner record

- Activate the `draft`, `published`, `closed`, `archived` lifecycle; remove the redundant `visible`
  boolean; derive availability from lifecycle plus the resolver.
- Project instructions, schedule, late policy, run timing, attempt selection, and lifecycle through
  typed Store, API, and browser contracts. `attempt_selection_policy = instructor_selected` gains its
  professor action: choose which run counts for one learner.
- Add co-instructor invitations for globally approved Instructor accounts only.
- Add course-local pages for groups and sections, entitlement, accommodations, schedule, retention,
  and archive. Accommodation editing always shows the resolved outcome, not the raw exception.
- Manual-grading queue: paginated, no raw response in the list, protected detail fetched per item,
  and a by-item pivot as well as by-learner order, because grading one question across learners is
  faster and more consistent.
- Private feedback snippets insert text and never calculate correctness.
- `AssignmentGradeOverride` is separate current state with reason, actor, strong revision, explicit
  clear action, and visible distinction from the computed score. Pending manual items stay
  independent of it.
- Surface `scoring_status`: recalculating and failed states are visible wherever a total is shown.
- Preserve the post-issue rules: adding or replacing questions blocked, reordering affects future
  runs, points and policies editable, removal via Delete and Regrade.

## 12. Instructor attention queue: defined by a predicate, not a list

[docs/HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) forbids a generic dashboard and permits a separate
surface for a demonstrated cross-course task. Rather than freeze an allowed list, define what
qualifies, so the surface can evolve without becoming a dashboard.

A row qualifies when all five hold:

1. it names one specific object the instructor owns or co-teaches;
2. it has a deadline, or a state that stays wrong until someone acts;
3. it has one primary action, reachable in one navigation from the row;
4. it can be resolved, dismissed, or snoozed by that action;
5. it would otherwise require visiting several courses to notice.

Disqualified by construction: statistics tiles, counts without an object, informational rows, and
anything whose action is "go look at a page".

Rows that qualify today: attempts pending manual grading, assignments closing soon, retention
notify/archive/delete deadlines, corrected or deprecated questions used by an active assignment,
imported assignments eligible for fast-forward, failed score recalculation, and unresolved
improvement threads from last term.

## 13. Autonomy boundary

Section 3.3 fixes the state model; this section fixes who acts.

- **Derived, no actor**: closure at the effective close time, lateness, availability, entitlement,
  and what a learner may see. These need no job and no professor click; they are computed.
- **Worker, non-derivable effects only**: retention notifications, purges and manifests, statistics
  contribution and aggregate refresh, score recalculation runs, export artifacts, and optional
  assisted-tagging batches. Each writes a durable receipt and is idempotent on replay.
- **Professor, always explicit**: publish, close early, override a grade, delete and regrade, archive
  early, extend retention, confirm tags, fork or replace a question, resolve an improvement thread,
  and instantiate or fast-forward curriculum.
- No new scheduler; every worker effect is an existing job family with a real handler and an atomic
  committer.

## Non-goals

- No live Alpha/Beta tethering, no three-way merge engine, no in-product contribution proposals.
- No learning trees, discussions, clickers, LMS roster sync, research exports, generated educational
  content, or a generic dashboard.
- No Manager, Publisher, Grader, Tester, or teaching-assistant roles.
- No answer keys, grading implementations, or author source exposed to non-authors, and none sent to
  any assisted-tagging model.
- No course-assignment history and no instructor-facing assignment versions.
- No grade formula language, curve engine, per-student scheme, or student what-if calculator.
- No dedicated search service while indexed PostgreSQL remains sufficient.
- No compatibility readers or legacy migration paths for nonexistent production data.

## Milestones

```text
M0  Release truth        Close discovery, statistics, and immutable-question release truth.
M1  Course spine         Term, policy resolver, disclosure, entitlement, groups, grade scheme.
M2  Teaching projection  Lifecycle, schedule, accommodations, pools, preview plane, rehearsal.
M3  Discovery commons    Search metadata, collections, picker, usage index, evidence validity.
                         Assisted tagging is an optional package, not on the critical path.
M4  Reusable curriculum  Blueprints, Alpha courses, clone, rollover, term shift, fast-forward.
M5  Evidence to action   Manual grading, overrides, work inspection, analysis, improvement
                         threads, attention queue.
M6  Connected term       Prove the whole professor cycle at term scale on the final tree.
```

### M0 Release truth

The evidenced release-truth packages are accepted. This milestone is recorded independently of the
open release-track activation gates. It delivers trigram and relevance search,
relevance-bound cursors, available-statistics rendering, a live discovery journey, and the
immutable-question release truth. WP-R1 closeout also replaces the Chapter One pilot/browser and
aggregate-acceptance shell orchestration with Python over the existing typed `local_stack_control`
boundary. Exit: exact Question ID behavior intact; content changes publish
a new Question ID with fresh opaque hidden `ProblemId` and `VersionId`; no publication or background
action advances an assignment; no sequential or version-chain question identity survives; broad and
misspelled searches return intended fixtures; facets and pages are snapshot-consistent;
representative plans use indexes. Two lanes maximum.

Status on 2026-08-14: WP-R0 is accepted with its named Memory, server, source-line, clean
PostgreSQL baseline, and independent-review evidence. WP-R1 is accepted with disclosed statistics,
Python-owned Chapter One and aggregate acceptance orchestration, a designated renderer name with
per-run OCI configuration-ID provenance, and final Validation: repository, Rust, 4,865-case pytest,
and seven-lane local-stack acceptance gates are green. WP-R2 is accepted with immutable-question
release truth. WP-PY-L1 is accepted on 2026-08-15 after final offline/live Validation and its named
independent final reviews. M0 is accepted for this professor roadmap from those four evidenced
packages; M1 is the next professor milestone.

### M1 Course spine

Depends on accepted M0 (including WP-R2 and WP-PY-L1). M1 is not one serial block. Only a small shared core is genuinely serial;
the rest parallelizes once that core is frozen.

#### M1 migration reservation

The shared migration ledger and allocation registry in
[implementation_status.md](../implementation_status.md) is authoritative for both roadmaps. The
`release integrator` owns migration ordering and allocations; this plan does not create a second
ledger. The six named M1 schema packages have the reservations recorded there, without placeholder
SQL or amendment/renumbering of accepted migration files. Future schema packages receive an
allocation before implementation; non-schema packages do not implicitly own a migration.

The actual clean-cluster baseline replacement requires both professor WP-PROF-E2 readiness and completion
of all repository-owned release schema packages/RC12, immediately before first production data. WP-PROF-E2
may prepare and review a candidate baseline earlier, but it must not replace the active ledger early.

```text
  serial core (WP-PROF-S1, WP-PROF-S2, WP-PROF-S7)
    decisions recorded | course term and zone | typed references, value types,
    migration allocation, RLS shape
        |
        +--> lane B  WP-PROF-S5 entitlement and typed group purposes
        |                |
        |                +--> lane A  WP-PROF-S3 resolver  --> WP-PROF-S4 disclosure
        +--> lane C  WP-PROF-S6 grade scheme, two shipped modes, deferred completion examples
```

- The serial core is deliberately small: the decisions, the course term, and the shared types,
  migration numbering, and RLS shape that three lanes would otherwise collide on.
- Lane B owns entitlement and group purposes. It defines the typed `EntitlementDecision` and its
  reasons, applicable group-purpose policy scopes, the derived-authority evaluation, and the
  enrollment/materialization seam. It consumes the term but not the resolver.
- Lane A starts after accepted Lane B output: WP-PROF-S3 consumes that contract and owns policy
  composition, window, limits, lateness, and per-field provenance; it must not derive entitlement
  from roster, audience, group membership, or enrollment. Disclosure follows the resolver inside
  the same lane because it consumes the resolver's output directly.
- Lane C owns the grade scheme; it consumes the term and the assignment points model, not the
  resolver, so it can proceed while lane A is still landing.
- The lanes integrate through the frozen types from the serial core, and the integrator owns route
  registration and migration ordering, so no lane edits another lane's capability module.

Exit: the entitlement component is the single source of the entitlement verdict and its reason; the
resolver is the single source of window, limit, and lateness, consuming that verdict as gate G2 and
returning both with per-field provenance; disclosure is evaluated server-side for every learner
projection; grade totals compute in the two shipped modes with documented rounding; Alpha records
cannot participate in any enrollment relationship. Completion-mode examples remain deferred design
work and do not enter the S6 evaluator or consumer contracts; M1 exits with the two shipped modes.
Three lanes after the serial core.

### M2 Teaching projection

Depends on M1. Lanes: lifecycle, schedule, late policy, instructions, scoring status; groups,
entitlement, accommodations, co-instructors, retention and archive pages; preview plane, rehearsal
runs, and item pools. Exit: every stored teaching policy in the inventory table is reachable,
editable, and keyboard-complete at 1280 by 800; a rehearsal leaves no enrollment, gradebook,
analysis, contribution, or export trace; a pool delivers its draw and respects the issued-run lock;
each preview names the layer that produced every value. Three lanes plus one reviewer.

### M3 Discovery commons

Depends on M1 and accepted WP-R2; runs beside M2. Delivers expanded search metadata, the usage index, collections and
Favorites, saved searches, bulk selection, the shared `ProblemPicker` adopted by Library and
assignment editor, the validity contract, and quality-signal computation with disclosed inputs.
Assisted tagging (`WP-PROF-D3`) is an optional package inside this milestone: nothing else depends on it,
and M3 exits without it.

Exit: one selection component and one metadata vocabulary across sources; usage and quality
aggregates leak no cross-tenant record and suppress below threshold; human taxonomy editing is
sufficient to make the corpus discoverable. If `WP-PROF-D3` ships, no tag reaches the catalog without a
confirming user and recorded model provenance. Two lanes plus the optional package.

### M4 Reusable curriculum

Depends on M2 and M3. Lanes: blueprints and Alpha authoring; clone, rollover, term shift, date
resolution; provenance, fast-forward, selected copy, and evidence context. Exit: two Instructors in
different tenants discover and clone one Alpha; only the creator edits it; the derived course
contains no student record; the preview refuses an ambiguous local time. Three lanes.

### M5 Evidence to action

Depends on M2, M4, and accepted WP-R2. Lanes: manual grading with by-item pivot, snippets, overrides;
learner work inspection with audit, grade-scheme-aware gradebook; course analysis, catalog evidence,
linked replacement comparison, improvement threads, attention queue. Exit: a mixed assignment moves
from pending manual work to a current course total; a flagged item leads through inspection and usage
to a distinct linked replacement; the decision is recorded and visible next term; the replacement's
effect is measurable against its source. Three lanes.

### M6 Connected term

Depends on M4 and M5. Delivers the integrated journeys below, live PostgreSQL and RLS evidence,
visual review, documentation, baseline migration closeout, and the full Validation suite. Exit:
every required gate green on the final material tree with no required skip, and no unresolved P0 or
P1 finding.

## Work packages

| ID | Owner | Scope | Depends on |
| --- | --- | --- | --- |
| WP-R0 | Catalog | Ranked full-text and trigram discovery, same-snapshot facets; accepted 2026-08-14 | none |
| WP-R1 | UI | Accepted 2026-08-14: disclosed statistics rendering, live broad-discovery evidence, and Python conversion of Chapter One pilot/browser plus aggregate acceptance lanes over existing typed `local_stack_control` | WP-R0 |
| WP-R2 | Release truth | Accepted 2026-08-14: immutable Question-ID publication, fresh opaque hidden evidence, explicit revision-checked assignment replacement, optional immutable provenance, and real host-seed manifest recovery | accepted WP-R1 |
| WP-PY-L1 | Python orchestration | Accepted 2026-08-15: focused Python modules replace `local_stack_control/launch.sh`, `_restart.sh`, and `containers/local_identity_bootstrap.sh`; final offline/live Validation and named independent final reviews passed | accepted WP-R2 |
| WP-PROF-S1 | Architect | Record spine decisions in guidance and this plan; accepted 2026-08-18 after independent ACCEPT with no P0/P1/P2 finding | accepted M0 |
| WP-PROF-S2 | Expert coder | Course term, zone, validation, migration (serial core); accepted 2026-08-18 after full Validation and independent final ACCEPT reviews | WP-PROF-S1 accepted |
| WP-PROF-S7 | Expert coder | Typed references, shared value types, migration allocation, RLS, and immutable public bylines (serial core); accepted 2026-08-19 after full Validation and independent final ACCEPT reviews | WP-PROF-S1 |
| WP-PROF-S3 | Expert coder | Accepted 2026-08-19: effective-policy resolver, ordered gates, grant-filtered modifiers, per-field provenance, and sealed attempt receipts (lane A); full Validation and three independent final reviews passed | WP-PROF-S2, WP-PROF-S7, WP-PROF-S5 |
| WP-PROF-S4 | Expert coder | Accepted 2026-08-19: assignment-owned five-field disclosure, learner-safe projections, fail-closed student access, class-statistics privacy, and the four-viewport role-based visual contract; full Validation and independent final reviews passed | WP-PROF-S3 |
| WP-PROF-S5 | Expert coder | Accepted 2026-08-19: entitlement authority, typed decision/reasons and applicable group-purpose scopes, derived authority, and materialization (lane B); full Validation and three independent final reviews passed | WP-PROF-S2, WP-PROF-S7 |
| WP-PROF-S6 | Expert coder | Accepted 2026-08-19: two-mode course-grade scheme, deferred completion examples, totals, and audited export; full Validation and three independent final reviews passed | WP-PROF-S2, WP-PROF-S7 |
| WP-PROF-T1 | Expert coder | Lifecycle, schedule, late policy, instructions, scoring status | WP-PROF-S3 |
| WP-PROF-T2 | Expert coder | Groups, entitlement, accommodations, co-instructors, retention | WP-PROF-S5, WP-PROF-T1 |
| WP-PROF-LD1 | Integrator | Accepted 2026-08-20: `base_course_installation`, LDA-owned SQL/lock/migration lifecycle, deterministic product evidence, and real-stack lifecycle proof | WP-PROF-T2 accepted |
| WP-PROF-LD2 | Expert coder | Seeded Student/Instructor entry and initial Sysadmin claim through ordinary WP-RC8 account-session paths; `2026081809` owns exactly two least-privilege execute-only brokers: Sysadmin approval-candidate discovery and read-only completed-installation-generation lookup for configured first-ownership proof; `2026081810` only repairs Student pre-tenant account-course retention | WP-PROF-LD1 accepted; necessary existing WP-RC8 account-session/passkey/origin contracts |
| WP-PROF-BS1 | Integrator | Current canonical disposable real-stack browser suite for Playwright, acceptance, and screenshots; UI-first scenario state; retirement of the test-only browser application and mock transport | WP-PROF-LD2 accepted |
| WP-PROF-T3 | Expert coder | Planned frozen-scope preview plane; resumes after accepted real-stack browser foundation | WP-PROF-S4, WP-PROF-T1, WP-PROF-LD1 accepted, WP-PROF-LD2 accepted, WP-PROF-BS1 accepted |
| WP-PROF-T4 | Expert coder | Rehearsal runs on the preview plane | WP-PROF-T3 accepted |
| WP-PROF-T5 | Coder | Item pool authoring over selection groups | WP-PROF-T1 |
| WP-PROF-D1 | Expert coder | Search metadata, usage index, validity contract, quality signal | WP-PROF-S7, WP-R2 |
| WP-PROF-D2 | Coder | Collections, Favorites, saved searches, bulk actions, ProblemPicker | WP-PROF-D1 |
| WP-PROF-D3 | Coder | Assisted tagging: worker, proposals, confirmation, provenance. **Optional; nothing depends on it** | WP-PROF-D1 |
| WP-PROF-B1 | Expert coder | Personal blueprints and public Alpha aggregates | WP-PROF-D2, WP-PROF-S7 |
| WP-PROF-B2 | Expert coder | Fork, instantiate, rollover, term shift, manifests, fast-forward | WP-PROF-B1, WP-PROF-T1 |
| WP-PROF-G1 | Expert coder | Manual-grading queue, by-item pivot, snippets, overrides | WP-PROF-T2 |
| WP-PROF-G2 | Expert coder | Learner work inspection with audit; grade-aware gradebook | WP-PROF-S6, WP-PROF-G1 |
| WP-PROF-G3 | Coder | Course analysis, catalog evidence, explicitly linked replacement/source comparison | WP-PROF-G1, WP-PROF-D1 |
| WP-PROF-G4 | Coder | Improvement threads | WP-PROF-G3, WP-PROF-B2 |
| WP-PROF-G5 | Coder | Attention queue against the actionability predicate | WP-PROF-G4, WP-PROF-T2 |
| WP-PROF-E1 | Playwright | Behavior-named professor journeys and live-stack evidence | all behavior WPs |
| WP-PROF-E2 | Integrator | Final gates, visual review, docs, changelog, baseline procedure | WP-PROF-E1 |

Each package owns its capability modules. The six named M1 schema packages and the accepted
post-M1 WP-PROF-LD1 allocation are recorded in the shared registry; WP-PROF-LD2 has the
`2026081809` two-broker allocation (Sysadmin approval-candidate discovery and completed-installation-
generation lookup for configured first-ownership proof) and the separate `2026081810` Student
pre-tenant account-course retention-boundary repair allocation. Every later schema package
receives a release-integrator allocation before implementation, and non-schema packages receive no
migration implicitly. Shared route registration and migration ordering belong to the integrator.

**WP-PROF-T1 current contract.** One revisioned `AssignmentTeachingSettings` aggregate owns the
closed Draft/Published/Closed/Archived lifecycle, validated learner instructions, and the absolute S3
base policy for availability, due, close, whole-run and attempt limits, late behavior, and deadline
behavior. New assignments are Draft and only stored Published opens G1. The instructor HTTP boundary
accepts strict course-local timestamps with the course IANA zone; the server authorizes and checks the
current revision before body interpretation, converts DST/term/order/bounds centrally, and commits
the aggregate plus active-attempt re-resolution atomically. Content edits remain a separate mutation
under the same assignment revision. Instructor reads return stored intent plus a closed current-state
union derived from authoritative time, including the course-local boundary for a scheduled or
clock-closed Published assignment; the browser performs no time comparison. Learners receive only the dedicated S5/S3-authorized detail with
plain-text instructions and resolved delivery facts, never policy intent, provenance, tenant/course
keys, or clocks. Recalculating/Failed scoring status suppresses every learner aggregate, run,
attempt-result, and disclosed-point numeric without
changing the semantic disclosure/activity state. The package allocates no migration and directly
removes the historical `AssignmentTimingPolicy`/`assignmentTiming` API.

**WP-PROF-T2 contract.** The shared migration allocation and package disposition are owned by
[implementation status](../implementation_status.md); this plan owns the frozen contract and
acceptance criteria. Course-group membership remains many-to-many. Each course persists an
`allow | warn` multiple-membership policy per group purpose:
Section defaults to `warn` and Lab, Cohort, Accommodation, and Work default to `allow`. A warning
never blocks a valid write. Deleting a group refuses while an assignment audience or policy modifier
references it. Group membership or purpose writes and M2--M4 modifier writes atomically re-evaluate
current S5 entitlement and S3 effective policy for affected active work while preserving sealed
evidence.

Global Instructor approval is a separate operator-owned eligibility record, never a platform role or
course authority. A co-instructor invitation targets an existing approved account, stores no email
in course authorization state, is bound to that account, expires 30 days after creation, and remains
visible and acceptable from the authenticated account's pending-invitations surface without email
delivery.
Acceptance rechecks current approval and atomically creates an ordinary direct Instructor membership.
Sysadmin status grants no ambient course authority, and no command may remove the final active
Instructor from a course.

T2 browser pages consume the existing retention engine and server-derived S5 entitlement and S3
effective-policy previews. They neither duplicate retention lifecycle state nor reconstruct
entitlement, modifier precedence, or provenance in the browser.

**WP-R2 acceptance.** Remove the Memory and PostgreSQL successor/propagation mechanisms, including
the pre-production trigger and exceptional correction authority, instead of adding a compatibility
shim. Remove sequential `ProblemPublicId`/`P-...`, `ProblemVersionNumber`, `previous_version`, and
predecessor/version-chain production identity, schema, parser, generated-type, and migration paths.
Assignment creation and editing select one immutable Question ID; retained assignments and issued runs
stay exact until an Instructor performs a visible, revision-checked replacement. Prove that every
content change receives a distinct Question ID plus fresh opaque `ProblemId` and `VersionId`, and
provenance may link source to replacement. Convert real persisted native and WeBWorK host seed
publishers to mint fresh opaque publication IDs; reruns use a protected explicit manifest or verified
existing record, never tenant-derived question UUIDs. Deterministic fixed IDs remain only in isolated
unit fixtures, derived render/cache identities, and non-question seed records. Historical attempts
replay their original evidence, and no instructor-facing route, selector, or latest-resolution path
accepts or exposes an internal version identity. Hidden exact transport and audit references remain
available where the authorization boundary requires them.

**WP-R2 result.** Accepted on the final material tree: `./check_codebase.sh` passed all five steps
with 260 Node tests; `source source_me.sh && python3 -m pytest tests/` passed 4,856 tests;
`./check_rust.sh` passed the full Rust suite; and `source source_me.sh && python3 local_stack.py
acceptance` passed all seven lanes. Those lanes covered ordinary browser behavior, two visual
verifiers, the canonical walkthrough, Chapter One pilot, Chapter One browser with four live
Question-ID replacements, and WebWork render/grade/outage. Test, UI, and architecture reviews each
returned ACCEPT with no P0/P1 finding. The designated canonical renderer image was rebuilt only for
the acceptance run; cleanup then removed all disposable containers, images, and volumes. The
professor roadmap's M0 evidence is accepted; WP-PY-L1 is accepted on 2026-08-15 after final
offline/live Validation and named final reviews.

**WP-R1 Python closeout.** WP-R1 is accepted on 2026-08-14. Chapter One pilot/browser and aggregate
acceptance lane sequencing now use Python with typed `local_stack_control` process, disposable-owner,
private-input, preflight, cleanup, and result boundaries. The browser journey remains real visible
Playwright interaction. A retained shell entry directly `exec`s the documented Python command. The
focused typed Python lifecycle is the current default `containers` owner. `containers/env.example` supplies the
designated local renderer image name as the stable selection and rebuild target, and each live run
records the inspected immutable OCI image configuration ID as exact runtime provenance. Rebuilding the
configured target supplies a new selectable local artifact after pruning while the receipt preserves
the configuration used. The professor roadmap's M0 evidence is accepted; WP-PY-L1 is accepted on
2026-08-15 after final offline/live Validation and named final reviews.

**WP-R2 evidence boundary.** WP-R2 uses inline builders by default and adds no fixture directory.
`crates/learning-data-access/tests/conformance/publication.rs` and `assignments.rs` own focused offline
Memory Store conformance, Question-ID-only commands, replacement preservation/refusal, and replay;
`crates/server/src/catalog/tests/publication.rs` and
`crates/server/src/course/tests/assignment_revision.rs` own server request behavior. The disposable
PostgreSQL/RLS driver `tests/e2e/e2e_wp_r2_postgres_rls.py` owns migration, forced RLS, cross-tenant
refusal, rollback, and persisted replay. `crates/project-tools/src/e2e_seed/tests.rs` owns manufactured
manifest convergence, while `tests/e2e/e2e_wp_r2_host_seed_renderer.py` owns real host-seed/renderer
publication evidence for native and WeBWorK without predicting assigned Question IDs. The authored mock
decoder/client/model owner is `tests/test_assignment_editor_ui.mjs`; its mock-backed visible assignment
replacement owner is `tests/playwright/assignment_editor.spec.ts`. The aggregate's only real replacement
browser route is `local_stack_control/acceptance_lanes.py`; the M6 composition journey is only
`tests/walkthrough/run_ui_walkthrough.py`. Durable M0 package evidence is recorded in
[implementation_status.md](../implementation_status.md) and [CHANGELOG.md](../../CHANGELOG.md);
one-time inventories, rendered screenshots, and timing observations are historical evidence only and
are not referenced through an ignored scratch artifact.

**Python orchestration validation.** WP-PY-L1 follows WP-R2 with the direct lifecycle, restart, and local
identity bootstrap conversion implemented above. The remaining complex E2E and canonical WeBWorK
acceptance scripts migrate afterward in their release-package dependency order: the renderer/host-seed
acceptance owner first, then the release-candidate composition owner. Each migration places state,
parsing, subprocess, private-environment, polling, and cleanup behavior in Python; any retained shell
entry is a direct `exec` facade. This schedule preserves WP-R1's bounded Chapter One and aggregate work.

## Acceptance criteria

- A professor searches broadly, tolerates a typo, filters by evidence and tag, inspects safe details,
  favorites a problem, places it in a collection, and adds it to an assignment without typing an ID.
- One resolver answers every entitlement, window, limit, and lateness question, and every professor
  and learner surface shows the same answer with its source named in plain language.
- Disclosure is set once per assignment and holds across run summary, gradebook, and analysis.
- A course has a term and zone; absolute dates require one; ambiguous local times are refused with a
  correction path; a term shift previews every resolved date before committing.
- The gradebook shows a course total under the selected mode, and one audited click opens exactly
  what a named learner saw and answered.
- A professor rehearses an assignment under a chosen learner-policy context at a chosen moment, and
  no enrollment, gradebook row, analysis observation, catalog contribution, or export row is
  created. The rehearsal carries no learner identity.
- A pool assignment delivers its draw per learner and honors the issued-run lock.
- An Alpha course is non-enrollable, public to approved Instructors, creator-editable, cloneable, and
  carries evidence context and improvement threads into the clone.
- Fast-forward updates only untouched reusable fields before the first issued run; divergence
  produces selected copy or new assignment, never an automatic merge.
- A flagged item leads from analysis to learner evidence to usage to a distinct linked replacement,
  and the decision is recorded and visible in next term's material.
- After an explicitly linked replacement is published, the professor can compare its disclosed
  behavior with its source's, with both Question IDs and sample sizes shown.
- Human taxonomy editing alone makes the corpus discoverable. If assisted tagging ships, no tag
  enters the catalog without a confirming user and recorded model provenance, and no grader-only
  material, answer key, or learner response is ever sent to a model.
- Every disclosed statistic passes the validity contract: below threshold the interface says
  insufficient evidence and contributes nothing to ranking or comparison.
- Entitlement authority is derived, materialization carries provenance, and a revoked learner's
  issued evidence survives with the row marked revoked.
- The attention queue contains only rows satisfying the actionability predicate, each resolvable from
  its own row.
- No public, learner, non-author, collection, blueprint, or Alpha response contains answer keys,
  grading implementations, private source, email, UUID, or FERPA data.
- Professor pages stay compact and keyboard-complete at 1280 by 800; student pages keep the tablet
  and narrow-phone guards.
- Student acceptance evidence includes an allowed student surface and fail-closed denial of
  instructor-only routes at the exact 1280 by 800, 800 by 1280, 393 by 852, and 800 by 800 CSS-pixel
  matrix. No-transport assertions and direct route probes accompany screenshots; pixels alone do not
  prove authorization.

## Test and verification strategy

- Derived-state tests: the same inputs produce the same verdict through the resolver, the preview
  plane, and production reads; an injected worker state and fixed clock prove delayed receipts preserve
  the verdict; replay of a completed job is idempotent. A stopped-worker process check belongs to its
  named E2E lane.
- Entitlement tests: each interaction case in section 2.4, including a materialized record whose
  derived authority is now false.
- Validity tests: suppression below the k-anonymity minimum, discrimination suppressed separately
  when the scored cohort is small, first-attempt independence, comparison of explicitly linked
  replacement/source questions with separately scoped exact-version evidence, and the
  insufficient-evidence answer contributing nothing to ranking.
- Domain tests: resolver precedence and provenance for every gate and modifier combination,
  extend-only accommodation semantics, disclosure evaluation across the time axis, grade computation
  in both shipped modes with rounding and drop rules, relative-calendar scheduling and DST refusal,
  pool draw determinism by algorithm version, clone manifest normalization, fast-forward eligibility,
  quality-signal computation with insufficient-sample behavior, and issued-run structural locks.
- Memory conformance: ordinary crate tests cover entitlement, group purposes and exclusivity policy,
  collection ownership, usage-index aggregation boundaries, public Alpha reads and creator-only writes,
  cross-tenant cloning, rollover exclusions, overrides, audited work inspection, rehearsal exclusion
  from every aggregate, tagging provenance, and retention.
- PostgreSQL/RLS proof: a named disposable PostgreSQL E2E exercises the same selected Store semantics
  where transactions, persistence, roles, and forced RLS are the contract. It is opt-in and separate
  from ordinary Cargo, Node, and pytest gates.
- Server tests: authentication, role checks, non-enumeration, strict decoding, strong revisions,
  idempotency, cache policy, audited reads, and absence of secret fields in every new response.
- TypeScript and Node: strict decoders, short route references, query and cursor recovery, local
  state preservation.
- Playwright, named for behavior rather than milestones: discovery to collection to assignment;
  schedule and accommodation with resolved-outcome and provenance checks; entitlement preview;
  rehearsal leaving no trace; pool delivery; Alpha clone across instructors; fast-forward and
  divergent selected copy; rollover and term shift preview; manual grading by item with override;
  gradebook total and audited learner work inspection; analysis to fork to recorded decision;
  attention-queue routing; keyboard, recovery, and canonical viewport behavior.
- S4 browser evidence must cover the student/access contract: allowed learner projection, direct
  roster and gradebook denial probes, a centrally derived fail-closed route boundary before transport,
  and no instructor payload on denied navigation. Fresh capture and inspection are required before
  screenshots count as acceptance evidence.
- Keep the canonical pilot walkthrough on its existing known-ID teaching loop; new journeys are
  separate visible evidence. `tests/walkthrough/` owns the one live M6 professor-to-two-student
  composition journey, with Playwright as its interaction engine; the aggregate acceptance invokes it
  once after package invariants are green.
- Final acceptance requires the full Validation suite in
  [docs/TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md) green on the final material tree with
  no required skip.

### Invariants proven before M6, not by M6

The connected journey proves that the pieces **compose**. It must never be the first place a core
behavior is verified, or a failure late in the narrative becomes expensive to diagnose. Each
invariant below is green in a small permanent behavior test owned by its package before M6 starts:

- resolver precedence, extend-only accommodations, and per-field provenance;
- derived-versus-durable state: lifecycle intent versus computed closure, and worker receipts;
- entitlement authority, materialization, and revocation with issued runs;
- disclosure evaluation at each point on the time axis;
- grade totals in every shipped mode, including the rounding and drop rules;
- rehearsal producing no enrollment, gradebook, analysis, contribution, or export row;
- pool draw determinism for a given algorithm version;
- clone and term-shift date resolution, including daylight-saving refusal;
- statistics suppression below the k-anonymity threshold and the insufficient-evidence answer;
- audited learner-work reads;
- improvement-thread state transitions and read-only propagation.

M6 then runs the narrative once, and a failure there means composition, not unit behavior.

### The M6 connected term journey

One narrative journey, run live, exercising the architecture as a system rather than visiting pages:

1. Dr. Fake Professor searches the catalog by concept, filters to questions with disclosed evidence,
   collects eight, and confirms three assisted tag proposals along the way.
2. She builds an Alpha course with two modules, one fixed assignment and one pool assignment, using
   relative dates.
3. She instantiates it into a Fall term with a start date and time zone, two sections and one lab.
4. She previews the schedule table, grants Mary Fake Student an extension, and confirms the resolved
   window shows the extension as its source.
5. She rehearses the pool assignment as a lab member at a moment after the due date, sees the late
   marking and the disclosure state she expects, and publishes.
6. Two students practice repeatedly; one triggers manual grading; one submits late.
7. She grades by item, applies one override with a reason, and watches the course total recalculate
   in the selected grade mode.
8. Analysis flags one item; she inspects a learner's exact variant, checks catalog usage and
   cross-course behavior, publishes an explicitly linked replacement with a new Question ID, records
   the decision, and deliberately replaces the item in next term's Alpha.
9. She rolls the course into a Spring term with a term shift; the preview resolves every date, the
   improvement thread is visible on the replaced item, and no student record travels.
10. After the second term, she compares the linked replacement question's behavior with its source.

## Migration and compatibility policy

- Preserve the active migration ledger until RC12 and this plan's accepted schema packages complete.
- No durable production data exists, so foundational schemas change directly with no compatibility
  readers. Course term, group purpose, entitlement, grade scheme, disclosure, usage index, and
  improvement threads join the existing epoch rather than arriving as bolt-ons.
- Remove `assignment.visible` once lifecycle plus the resolver replace it. Remove
  `catalog_search_document.quality_signal` if section 6.1 is not adopted; do not leave it unowned.
- WP-PROF-E2 may prepare and review a candidate clean-cluster baseline before all packages finish, but the
  actual baseline replacement requires both professor WP-PROF-E2 readiness and completion of all
  repository-owned release schema packages/RC12, immediately before first production data. If
  durable pilot data exists first, stop consolidation and use forward-only migrations; WP-PROF-E2 must not
  replace the active migration ledger early.
- Keep Alpha and shared-curriculum tables outside FERPA course-record ownership.
- Keep PostgreSQL search behind the existing repository boundary.

## Risk register

- **Spine deferral**: shipping a visible feature before the spine exists; mitigated by the small
  serial core that freezes the term and shared types before any lane starts, by lane gates that
  require the resolver and entitlement verdicts to be green before M2 consumes them, and by the
  no-local-re-derivation rule.
- **Precedence sprawl**: new policy layers added ad hoc; mitigated by the fixed gate/modifier model,
  per-field provenance, and one conformance suite every surface must pass.
- **Grade-scheme creep**: requests for formulas, curves, or completion grading; mitigated by the
  closed two-mode scope and explicit non-goals.
- **Rehearsal leakage**: implementing rehearsal as a filtered ordinary run; mitigated by structural
  impossibility at the store boundary and conformance tests on every aggregate.
- **Evidence misreading**: professors treating small or non-comparable samples as fact; mitigated by
  the section 6.0 validity contract: existing k-anonymity suppression, first-attempt independence,
  separately scoped exact-version evidence, comparison only of explicitly linked replacement/source
  questions, disclosed cohort sizes, and an explicit insufficient-evidence answer.
- **Acceptance-oracle concentration**: M6 becoming the first place core behavior is checked;
  mitigated by the invariant list that must be green in small permanent tests before M6 runs.
- **Entitlement drift**: derived authority and materialized records disagreeing after roster or
  audience changes; mitigated by the interaction conformance cases listed in section 2.4.
- **Tagging trust**: model proposals entering the catalog unreviewed, or content disclosure to an
  external service; mitigated by confirmation-before-write, recorded provenance, public-content-only
  input, and a recorded operator decision before any external model is used.
- **Attention-queue drift**: rows that inform rather than act; mitigated by the five-part predicate
  and a review of every new row type against it.
- **Alpha synchronization complexity**: background updates or merges; mitigated by current-source
  comparison, untouched fast-forward, and selected copy only.
- **Cross-tenant leakage**: usage, quality, or collection queries joining tenant records; mitigated
  by aggregate-only exposure, separate shared-curriculum stores, forced RLS, and clone-time
  reauthorization.
- **Answer leakage**: richer previews, rehearsal, tagging, or grading aids; mitigated by author-only
  protected preview, server-only grading, and answer-free non-author paths.
- **Scope collapse**: running every lane at once; mitigated by dependency-ordered milestones,
  one-owner packages, focused gates, and independent review.

## Documentation close-out

- Preserve only settled owner decisions in [docs/HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md), in the
  owner's voice: course term required, rehearsal instead of a test student and never carrying learner
  identity, the attention predicate, and the evidence-disclosure stance. Conditional architecture
  (completion mode, assisted tagging) and component ownership stay in this plan.
- This document is the active professor-capability scope and dependency direction; link it from
  implementation status without replacing the current release plan or global current-package
  handoff.
- Update architecture, file structure, contracts, database structure, install and usage, instructor
  guidance, test evidence, and troubleshooting only when their owning capability changes.
- Add one categorized [docs/CHANGELOG.md](../../CHANGELOG.md) entry per accepted package.
- Archive under `docs/archive/` only after M6 and the full final Validation suite pass.

## Assumptions and recorded decisions

- A teaching course requires a term with an IANA zone before it accepts absolute assignment dates.
- Accommodations are extend-only unless explicitly marked as an override.
- Entitlement authority is derived from roster, audience, group, and lifecycle; the enrollment record
  is its materialized receipt with provenance; revocation ends authority and preserves evidence.
- Effective state is always derived from (policy, now). `assignment.lifecycle` records professor
  intent. Workers own only non-derivable effects and write a durable receipt for each.
- A rehearsal's subject is a `PreviewSubject` of resolved policy values and group roles, never a
  learner identity; deriving one from a learner is an audited record read.
- Grade scheme version 1 ships total points (default) and weighted categories with drop-lowest;
  completion-based totals remain a later package while the three worked examples stay as design
  evidence. Letter bands and rounding are shared across the shipped modes.
- Assisted tagging is optional and off the critical path; the architecture must succeed with
  human-managed taxonomy.
- Improvement threads have exactly the fields, states, visibility, and propagation rules in section
  7, and no assignees, comments, or notifications.
- Every disclosed statistic obeys the section 6.0 validity contract, extending the existing
  k-anonymity disclosure policy rather than adding a parallel one.
- A Question ID names one immutable published question. Every content change receives a new Question
  ID with optional immutable provenance; retained assignments and issued runs remain exact until an
  Instructor deliberately replaces an assignment item. Internal `(ProblemId, VersionId)` evidence
  is never an instructor-facing selector.
- Group membership is many-to-many; section exclusivity is a course policy that warns, not a schema
  constraint.
- Rehearsal runs are instructor-visible only, discarded when the assignment definition changes, and
  never exported.
- Attention-queue membership is governed by the five-part predicate, not by a fixed list.
- Assisted tagging sends public published question content only, writes proposals only, and requires
  a confirming user; an external model requires a recorded operator decision.
- `quality_signal` is adopted with a defined, explainable computation or removed from the schema.
- Catalog-wide statistics stay anonymous under the existing disclosure boundary and always show
  sample size.
- "Public Alpha course" means visible and cloneable by approved Instructors, not publicly runnable.
- Alpha courses and teaching courses are separate aggregates, not convertible kinds.
- Fork lineage is retained; contribution proposals stay outside this plan.
- Existing answer-secrecy, role, first-issued-run, retention, human-reference, and
  no-generic-dashboard decisions remain authoritative.

## Appendix: design derivation

The abstractions above came from a structured expansion rather than a feature comparison. Recorded
so a reviewer can check the reasoning and repeat the method.

```text
+---------------------------------------------------------------------------------+
| operators: substitution, cross-domain re-instantiation                           |
| organon:   dictionary (genus and differentia per coined term)                     |
| axes:      autonomy, intentionality, size            pivot axis: size             |
+---------------------------------------------------------------------------------+
| substitution  scale course -> term         => course term, zone, term shift       |
| substitution  scale course -> section set  => typed group purposes, entitlement    |
| substitution  scale course -> item pool    => professor surface over selection     |
|                                               groups already present in schema     |
| substitution  autonomy professor -> system => scheduled close, disclosure,          |
|                                               retention, tagging batches           |
| re-instantiate  theater: rehearsal         => preview plane; rehearsal run as its   |
|                                               one mutating instance                |
| re-instantiate  theater: repertory + notes => catalog evidence commons and the      |
|                                               improvement thread                    |
+---------------------------------------------------------------------------------+
| second pass over the store found four more unowned concepts:                      |
|   entitlement (enrollment is per assignment, not per course);                     |
|   question assignability and lineage semantics;                                   |
|   scoring generation and regrade visibility;                                      |
|   orphan schema with no code (collections, keywords, quality signal).             |
+---------------------------------------------------------------------------------+
| not yet surfaced, for a later pass:                                               |
|   the learner's own view of this system (progress, practice recommendation);      |
|   the program scale above the course (shared sections across instructors,         |
|   program outcomes, department-level evidence);                                   |
|   the author economy (ownership transfer, co-authoring, attribution when a fork    |
|   outgrows its source).                                                           |
+---------------------------------------------------------------------------------+
```
