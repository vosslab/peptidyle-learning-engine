# Interface terminology

This companion to [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines
canonical interface-surface names and their semantic ownership.
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) is the product authority, and
[UI_DESIGN_GUIDE.md](UI_DESIGN_GUIDE.md) owns placement, geometry, ordering,
and interaction behavior.

**Application Shell** is the persistent frame around the current PLE content
region. It owns the **Ribbon**, presentation settings, and the content origin.
Route content renders inside that frame.

**Ribbon** is the Application Shell-owned navigation surface. It persists while
route content changes. A **Ribbon Schema** names the navigation vocabulary
applicable to one **Ribbon Scope** and **User Role** pair. A Ribbon Scope is
the exact product context: **Product**, **Course Instance**, or **Assessment
Attempt**.

**Ribbon Context Row**, **Ribbon Tab Row**, and **Ribbon Task Row** name the
three kinds of Ribbon region. A **Ribbon Context Control** is a utility
destination owned by the Context Row. A **Ribbon Tab** is a primary navigation
destination. A **Ribbon Task** is a task-specific navigation destination. A
**Ribbon Task Area** is a presentation-only name for adjacent related Tasks.
**Selected Ribbon Tab** names the Tab matching the current route; **No Selected
Ribbon Tab** describes a Context Control route whose schema remains present.

**Ribbon Slot** is one named position in a Ribbon Schema. **Ribbon
Availability** describes whether its destination is Available, Checking, or
Unavailable. Availability is distinct from selection, loading, Account State,
Course Membership state, and Blueprint Course lifecycle state.

**Page Action** performs an operation on current content, such as Create
Assessment, Save, Publish, or Submit. Ribbon controls navigate; Page Actions
operate.

**Content Layout** is the route-selected composition below the Ribbon.
**Reading Layout** uses a bounded prose measure. **Full-width Layout** uses the
available content width for dense records and teaching workspaces.

**SearchBlock** composes page-supplied criteria and controls, **DataSheetControls**,
and a required **DataSheetBlock**. **DataSheetBlock** owns all four modes and
shared sorting, filtering, selection, and record actions. **DataSheetTable**
provides the reusable Compact/List table and owns column configuration and
interactions: resize, show/hide, rearrange, responsive hiding, and independent
temporary widths, visibility, and order for Compact and List. **HeaderRow** is
the heading interface within DataSheetTable. It forwards sorting/filtering
input to DataSheetBlock and column input to DataSheetTable without becoming an
independent controller. **RecordItem** provides record content and direct
actions in every mode. **DataSheetControls** supplies surrounding controls
through the same state and callbacks, delegating collection behavior to
DataSheetBlock and column mechanics to DataSheetTable. Page/domain query
owners apply whole-result sorting and filtering before pagination.
The target modes are **Compact**, **List**, **Cards**, and **Visual Boxes**.
M13's verified extraction uses the existing RecordTable; the named DataSheetTable
boundary is M15 target work, not a claim that it has shipped.

**Courses**, **Questions**, and **Assessments** are the Instructor Product
Ribbon's canonical primary destination names. **Profile** and Account controls
remain Context Controls. **Question Library** is the canonical name for the
Question discovery capability, not a required top-level tab.

**My Blueprint Courses**, **My Active Courses**, **My Inactive Courses**,
**Search Blueprint Courses**, and **My Change Proposals** name the Courses tasks.
**My Questions**, **My Pools**, **My Drafts**, **Starred**, **Watched**,
**Search Question Library**, and **Browse Question Library** name the Questions tasks.
**Assessments Due Soon** and **My Assessment Templates** name the Assessments tasks. A retained
name does not itself claim a backed capability.

**Questions in no Pool** names a search filter for Published Questions with no Pool
membership. It does not mean every individually displayed Question. Question Library search
includes both Questions and Pools; HG currently prefers Questions in no Pool plus Pools by
default, with member Questions available through filters.

**Pool mismatch** means the Pool no longer satisfies its current requirements. Display the
specific cause alongside the state. It is not an unpublished or Draft Pool state.

**Assessment Question Editor** names the Instructor task for selecting, adding,
removing, and ordering Questions and Question Pools. **Assessment Properties
Editor** names the whole-Assessment settings task: dates, scoring, Attempts,
late work, release, disclosure, and related settings. These are distinct
editors over one current Assessment.

**Coursework** is the Student-facing collective term for Regular Assignments,
Practice Question Assignments, Bonus Assignments, Quizzes, and Exams. An
individual Student item uses its specific Assessment Type label and icon.
Student pages avoid the generic internal term Assessment.

**Attempt**, **Back to Coursework**, and **Assessment Attempt Progress** name
the Student Assessment Attempt surface, its Coursework destination, and its
current Question position. Question navigation and timing remain Attempt
content, not Ribbon vocabulary. **Submit Assessment** finalizes the whole
Assessment Attempt; saving a complete Question response is not a Question-level
submission.
