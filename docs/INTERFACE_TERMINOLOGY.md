# Interface terminology

This companion to [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines
canonical interface-surface names and their semantic ownership.
[UI_DESIGN_GUIDE.md](UI_DESIGN_GUIDE.md) owns placement, geometry, rendering,
and interaction behavior.

These are retained interface contracts. The current Live Demo stops at seeded
Account session entry and does not render the Ribbon, Course, Assignment, or
Student surfaces described below. See [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md)
for the executable boundary.

**Application Shell** is the persistent frame around the current PLE content
region. It owns the **Ribbon**, presentation settings, and the content origin.
Route content renders inside that frame.

**Ribbon** is the Application Shell-owned navigation surface. It persists
while route content changes and has one stable **Ribbon Schema** for each
combination of **Ribbon Scope** and **Product Role**. Every Product Role uses
the same Ribbon architecture with its own distinct menu. A page supplies its
task heading and workflow content inside the content region.

**Ribbon Schema** is the predefined ordered set of Ribbon Slots and Ribbon
Tasks selected by one Ribbon Scope and Product Role pair. **Ribbon Scope** is
the exact product context. The closed scopes are:

- **Product Ribbon Scope** for navigation across PLE surfaces without one
  selected Course Instance or Assignment Attempt.
- **Course Instance Ribbon Scope** for one live Course Instance.
- **Assignment Attempt Ribbon Scope** for one Student's exact Assignment
  Attempt.

Ribbon Scope and Product Role select the Ribbon Schema. Exact domain
relationships supply presentation availability. The current route supplies
selection. Loaded records supply labels and course appearance. Server and
Store boundaries continue to authorize every protected operation. Because
Product Role is immutable, one Account uses one Ribbon Schema for each Ribbon
Scope throughout its Authenticated Session.

**Ribbon Context Row** is the fixed row that identifies PLE, the current
Course Instance or Assignment Attempt when present, and the current Account
and Profile controls. A **Ribbon Context Control** is a utility destination
owned by the Context Row rather than a Ribbon Slot. Account and Profile are
Ribbon Context Controls. Context labels remain separate from the page's task
heading.

**Ribbon Tab Row** contains the primary **Ribbon Tabs** for the current Ribbon
Schema. A Ribbon Tab is a navigation link to one primary destination. The
**Selected Ribbon Tab** is the tab whose destination matches the current route.
A route reached through a Ribbon Context Control may have **No Selected Ribbon
Tab**; the selected Ribbon Schema remains present with no Tab selected. Account
Security, Instructor Course Invitations, and Sign In are Context Control routes
that use this state.

**Ribbon Task Row** contains secondary **Ribbon Tasks** for the Selected
Ribbon Tab. A Ribbon Task is a navigation link to one task-specific
destination, such as Overview, Questions, Policies, Grading Operations, or
Student View for an Assignment. A **Ribbon Task Area** is a presentation-only
heading for adjacent Ribbon Tasks with one shared purpose.

**Page Action** is a control that performs an operation on the current
content, such as Create Assignment, Save, Publish, or Submit. Page Actions live
with the content they affect. Ribbon Tabs and Ribbon Tasks navigate; Page
Actions perform operations.

**Ribbon Slot** is one stable ordered position in a predefined Ribbon Schema.
Its **Ribbon Availability** is one of:

- **Available** when current presentation facts make the destination
  appropriate to show as a live link.
- **Checking** while the exact relationship facts needed for presentation are
  loading.
- **Unavailable** when the known relationship excludes that destination from
  the current Ribbon.

Selection and loading are separate from Ribbon Availability. **Selected**
means the control's destination is the current route. **Loading** means a
navigation to that destination is still in progress. **Active** remains a
domain-state term for records such as Accounts and Course Memberships.

**Content Layout** is the route-selected composition below the Ribbon.
**Reading Layout** uses a bounded line length for prose. **Full-width Layout**
uses the available content width for the Question Library, teaching workspaces,
and dense records.

Canonical Product destination names are **Courses**, **Question Library**,
**Blueprint Courses**, and **Instructor Accounts**. Courses is the current
Account's Course Instance surface. Instructor Accounts is the Sysadmin surface
for Instructor Vetting, Create Instructor Account, and Account State management.
Account and Profile remain Ribbon Context Controls.

Canonical Question-area destination labels are **All Questions**, **My Questions**,
**My Question Drafts**, **Starred**, and **Watched**. All Questions means every
Published Question available through the Question Library, and My Questions is
the current Account's owned Published Question View. When a complete usable path
is backed, My Question Drafts will enter the separate private Authoring Workspace
Store; its retained interface placement does not grant Draft Questions Question
Library membership. Starred names a Question
Star relationship and Watched names a Question Watch relationship. Question
Folders, Question Tags, Saved Question Searches, and search facets organize or
find Questions in their applicable destination.

Canonical Course Instance destination names are **Assignments**, **Students**,
**Gradebook**, **Teaching Operations**, **Blueprint Updates**, and **Course
Setup**. Teaching Operations names the teaching and course-lifecycle surface.
Blueprint Updates names reviewed changes from the parent Blueprint Course.
Course Setup names Course Instance configuration; **Grade Settings** names its
grade-calculation configuration and **Appearance** names Course Appearance.
**Create Assignment** is a Page Action.

Canonical Assignment Attempt labels are **Attempt**, **Back to Assignments**,
and **Assignment Attempt Progress**. Attempt names the Student's current
Assignment Attempt surface. Back to Assignments names its course-Assignment
navigation destination. Assignment Attempt Progress names the current Question
position. Question positions are Attempt content rather than Ribbon navigation.
