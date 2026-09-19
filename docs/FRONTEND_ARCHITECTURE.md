# Frontend architecture

PLE is a SolidJS single-page application backed by a same-origin Rust API and
an answer-free Rust/Wasm boundary. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) owns
product behavior. Current source directories, routes, and DTOs with
`assignment` or old Blueprint-state names are implementation gaps.

## Application structure

```text
src/route_contract.ts and src/routes.ts
  -> role-admitted application shell
    -> page composition in src/pages/
      -> feature workflows in src/features/
        -> typed same-origin API client in src/api/
          -> strict runtime decoding of server JSON
```

Route Product Role checks control presentation only. Every request repeats
authorization on the server. An opaque route ID locates a candidate; it
never grants access.

Generated TypeScript reflects Rust wire contracts. Authored decoders reject
unknown or malformed data before a page uses it. An existing wire name may be
documented for migration, but new product copy uses the Human Guidance term.

## Role workspaces

### Instructor

The primary Ribbon tabs are Courses, Questions, and Assessments.

- Courses: My Blueprint Courses, My Active Courses, My Inactive Courses, and
  Search Public Blueprint Courses.
- Questions: My Questions, My Draft Questions, Starred, Watched, Search
  Question Library, and Browse Question Library.
- Assessments: Assessments Due Soon and My Assessment Templates.

The Question Library is one subject-agnostic collection designed for thousands
of Questions. Search, Browse, filters, sorting, dense results, and bulk metadata
editing support finding and cleaning up large imports without opening every
Question.

Course Instance pages expose Course-local roster, Assessments, Gradebook,
appearance, and other implemented teaching tasks. Every current co-Instructor
has equal access. Student View is an answer-free preview, not another Product
Role or a Student Work creator.

### Student

Student work is collectively Coursework. A particular Assessment uses its
Assessment Type name. The Attempt page presents one Question at a time while
keeping navigation to all Questions and saved-status information available.
The primary completion action is **Submit Assessment** and targets the whole
Assessment Attempt.

### Sysadmin

The Sysadmin shell exposes implemented platform administration only. It does
not imply ambient Course membership or FERPA access. Scoped support access is
deliberate and recorded.

## Shell behavior

The shell owns stable Ribbon and page geometry, Account context, Profile menu,
breadcrumbs, responsive behavior, and focus restoration. Sign Out is in the
Profile menu. Required backed destinations remain visible when a collection is
empty and show an honest empty state. Future capabilities are not rendered as
usable buttons.

Exact labels and composition are in
[UI_DESIGN_GUIDE.md](UI_DESIGN_GUIDE.md) and
[INTERFACE_TERMINOLOGY.md](INTERFACE_TERMINOLOGY.md).

## Assessment editor

The Instructor uses the Assessment Question Editor for composition and the
Assessment Properties Editor for settings. An Assessment is current state; an
Edit Number may protect saves, but no Assessment Revision exists.

Release is explicit. Unrelease is a Danger Zone operation with exact-title
confirmation and deletes all Student Work for that Assessment. The UI must
state that consequence before the action.

Current implementation under paths such as `src/pages/assignment_workspace/`
or clients named `assignment_*` remains the code migration site. Those names do
not define product copy.

## Blueprint editor

A new or forked Blueprint is Private and starts with Revision 1. The owner-only
editor holds unsaved work locally until explicit Save. A meaningful Save creates
the next immutable Blueprint Revision; a no-op creates none. Rename and
Private/Public/Archived lifecycle changes do not create content Revisions.

Public Blueprints are adoptable. Private Blueprints are owner-only. Archived
Blueprints are read-only, hidden from ordinary discovery, available only by
explicit archived inclusion, and forkable. Blueprints have no dates, Students,
time zones, or relative schedules.

Daughter Courses expose newer Blueprint Revisions for Instructor review and
approval. Existing Assessment changes are never silently applied; newly added
Blueprint Assessments appear automatically as Unreleased Course Instance
Assessments. Forks expose later source changes for selective review. Blueprint
Course Change Proposals show proposed differences to the receiving owner and do
not directly change daughter Courses.

Public and Archived Blueprint detail supports visible Stars and private
Watches. Star counts and the vetted Instructors who Starred are visible to
vetted Instructors; Watch membership is visible only to the watcher. Adoption
and forking do not create either relationship.

## Student Attempt boundary

The Attempt page requests one answer-free Question presentation and the
Student's current saved response. A complete response can be saved and replaced
while the Attempt is open. An incomplete response is not saved as complete.

Saving changes only the working response and does not expose a grading outcome.
Whole-Assessment submission or the deadline closes the Attempt and finalizes
all saved responses together. Results and feedback appear only when policy
allows them.

Wasm may provide answer-free validation and formatting. It never owns Account
authority, Course access, Answer Keys, backend credentials, grading, credit, or
submission state.

## Backend-owned documents

The frontend hosts an authorized backend document in an isolated frame and
captures the bounded opaque response shape defined by that backend. It does not
inspect controls, infer Question Type, or rewrite the interaction into native
PLE semantics.

## Browser data rules

- Protected API requests use the typed same-origin client and `no-store`.
- Credentials, Student responses, grades, private Question source, Answer Keys,
  and backend state do not enter persistent browser storage.
- Recoverable errors preserve the visible Student/Instructor edit when safe.
- Cursor pages follow server cursors and do not synthesize another paging model.
- Pages use semantic labels, visible validation, keyboard-equivalent actions,
  and accessible focus recovery.

## Verification

Focused TypeScript tests protect strict decoding, route/reference binding,
current-state concurrency, answer-free DTOs, and Attempt saving. Real-stack
browser acceptance protects visible role workflows, whole-Assessment
submission, authorization, empty states, responsive layout, and destructive
actions. Screenshots are supporting rendered evidence, not product authority.
