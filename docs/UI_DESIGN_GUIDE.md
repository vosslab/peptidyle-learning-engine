# PLE interface design guide

## Design intent

PLE is a teaching workspace for repeated, open-book practice. It should feel calm, purposeful, and
specific to a course rather than like a collection of generic bordered forms. The interface gives
space to the current teaching decision, keeps secondary explanation available without making it
permanent noise, and makes the next action easy to recognize.

Instructor work is composed first for a 1280 by 800 CSS-pixel laptop viewport. Student work is
composed for that same canonical laptop viewport and for the high-priority 800 by 1280 tablet
target, and stays usable through a narrow-phone compatibility guard.

## Hierarchy

Use four typographic levels:

1. Compact page title: identifies the current place without consuming the workspace.
2. Section title: names one decision or data group.
3. Row label or Question Title: supports scanning inside that group.
4. Label, value, help, and metadata: ordinary text with weight used sparingly.

Page titles should generally occupy one line. Prefer normal or medium weight for ordinary labels and
values; reserve bold for the few words that establish structure. Status text should describe a useful
state or recovery action. Do not render confirmations such as "Question content ready" when the
visible content already proves the state.

## Space and layout

- Global shell: use nearly the full viewport, with bounded edge gutters rather than a narrow centered
  column. Reading-heavy student prose may use a local readable measure inside the wider shell.
- Instructor pages: prioritize horizontal scanning, compact rows, sticky local actions when useful,
  and one coherent primary work surface. Give Question Library results the primary work surface
  beside a compact, task-related sidebar.
- Student questions: keep prompt, response, feedback, navigation, and timer in one visual sequence.
  On large screens, use width to improve line length and grouping, not to separate the question from
  its answer across a long eye movement.
- Vertical rhythm: use small, consistent steps. Add space between groups; do not pad every child.
- Footer: global slogans do not occupy a permanent band on teaching workspaces.

### Adaptable layout tokens

Treat density as a design-system setting, not a collection of page-specific numbers. Shared CSS
custom properties in `src/style.css` own the geometry most likely to change after observation:

| Token category                                               | Controls                                                                      |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| `--ple-shell-*`                                              | Maximum application width, viewport gutters, header height, and shell padding |
| `--ple-layout-gap`, `--ple-section-gap`, `--ple-compact-gap` | Page columns, group rhythm, and dense local rhythm                            |
| `--ple-panel-padding`, `--ple-row-padding-*`                 | Work surfaces, Question Search rows, tables, and editor rows                  |
| `--ple-control-min-height`, `--ple-dense-row-min-height`     | Shared controls and compact instructor records                                |
| `--ple-reading-max-inline`, bounded-list geometry            | Reading measure and Question Library working height                           |
| `--ple-instructor-*-min-inline`, `--ple-filter-*-min-inline` | Assignment columns and Question Search filter allocation                      |
| `--ple-*-table-min-inline`, `--ple-*-block-size`             | Deliberate overflow thresholds for dense data and bounded lists               |
| `--ple-course-scope-*`, `--ple-course-theme-*`               | Course canvas extent, inset, color washes, identity rail, and surface fade    |
| `--ple-mobile-nav-*`                                         | Compact single-row phone navigation without changing its semantics            |

Adjust these tokens first when evidence supports a density change. Page styles may derive small
differences with `calc()`, but should not duplicate the governing measurement. Breakpoints are
behavior thresholds rather than distances: change one only when the composition actually stops
working, and validate 1280 by 800 plus the student tablet and phone targets after any token change.

## Navigation

Global navigation, course navigation, assignment progress, and page actions are different layers and
must look different. Global navigation is quiet and persistent. Course navigation sits inside the
course identity surface and clearly marks the active section. Assignment progress communicates
sequence and state. Page actions live with the content they affect.

The [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) owns Ribbon vocabulary and canonical visible
names. This guide owns which Slots and Tasks exist, their order, their placement, and their
presentation behavior. `src/ribbon/ribbon_contract.ts` is the planned executable owner of the same
schemas.

Use one ordered Ribbon Schema for each Ribbon Scope and immutable Product Role pair. Every role uses
the same Application Shell and Ribbon architecture with a completely distinct menu:

| Ribbon Scope       | Instructor                                                                             | Student     | Sysadmin            |
| ------------------ | -------------------------------------------------------------------------------------- | ----------- | ------------------- |
| Product            | Courses, Question Library, Blueprint Courses                                           | Courses     | Courses             |
| Course Instance    | Assignments, Students, Gradebook, Teaching Operations, Blueprint Updates, Course Setup | Assignments | Teaching Operations |
| Assignment Attempt | None                                                                                   | Attempt     | None                |

Product Role is available with the Authenticated Session, so one Account uses one stable schema for
each scope throughout its session. Exact server and Store checks continue to authorize every
destination and operation.

Place Account and Profile controls in the upper corner of the Ribbon Context Row. Account Security,
Instructor Course Invitations, and Sign In use those Context Controls. Their routes retain the
current Ribbon Schema and render with No Selected Ribbon Tab.

The Question Library interface area has five ordered Ribbon Tasks in two Ribbon Task Areas:

- **Question destinations**: **All Questions**, **My Questions**, **My Question Drafts**.
- **Question Relationships**: **Starred**, **Watched**.

Library means Published Question discovery, My means ownership, Draft means private authoring
state, Starred means endorsement, and Watched means a private subscription. My Question Drafts
navigates to the separate Authoring Workspace Store; its placement here does not make drafts part
of the Question Library. Question Folders, Question Tags, Saved Question Searches, and search
facets organize or find Questions in their applicable destination.

Course Setup has the ordered Ribbon Tasks **Grade Settings** and **Appearance**. Create Assignment is
a Page Action on Assignments. Add a future dashboard when it answers a distinct cross-course
monitoring task that these object-centered surfaces cannot answer directly.

Assignment Attempt uses one Student Ribbon Slot, **Attempt**, and one Ribbon Task, **Back to
Assignments**. Reserve a fixed-width, tabular-numeral position in the Ribbon Context Row for
**Assignment Attempt Progress**, such as `Question 3 of 7`. Keep Question navigation and timing in
the Attempt content so Assignment length never changes Ribbon topology. The Instructor and Sysadmin
Assignment Attempt schemas contain no Slots.

Within each Ribbon Schema, Slots available to every applicable Course Membership Role come first and
relationship-narrowed Slots form the remaining suffix. Resolve that suffix before displaying it,
omit Unavailable Slots, and preserve the relative order of visible controls. A later availability
result can therefore append controls without moving a visible control. This rule supports future
Course Observer, Student Observer, and Grader relationships that are independent of Product Role.

Use real links for navigation and buttons for mutations. Active navigation uses shape, position,
text, and color together; color alone is not the indicator.

## Controls and action priority

- Primary: one filled action for the page's main commitment, such as Save assignment or Submit
  answer.
- Secondary: a quiet filled or subtle bordered action for a meaningful alternative.
- Tertiary: text or icon-and-text controls for reversible local utilities such as Move, Copy, or
  Review.
- Destructive: visually restrained until invoked, with explicit wording and confirmation where the
  consequence warrants it.

Repeated ordering controls share one compact row pattern. Dragging is an efficient pointer option,
not the only path; directional controls and a direct position selector preserve keyboard and precise
operation. Controls should not acquire borders merely to look clickable when shape, placement, and
interaction styling already communicate that role.

## Grouping, borders, and surfaces

Group by shared background, alignment, and proximity before adding a border. Use a divider when two
adjacent rows need scan structure, a boundary when a component genuinely contains something, and a
surface tint when a whole region shares purpose. Avoid nested cards and boxed radio choices.

Tables use alignment and subtle row rules. Expanded detail uses an inset surface with a human summary
before any technical metadata. Empty states state what is absent, why that matters, and the primary
next step; a dashed placeholder alone is not a finished state.

## Course themes

Each stored three-color palette is meaningful. Standard presentation uses the full canvas anchor for
the course environment, then derives separate tinted work, grouping, and reading-card surfaces. The
raw secondary anchor identifies the active course-navigation section with a measured light or dark
label, while the accent anchor remains visible in the course rail and local composition. Links,
actions, text, focus, and quiet boundaries stay related to those same three anchors.

The shared `THEME_MIX` recipe in `course_theme_registry.ts` owns the projection percentages. Change that
recipe and the shared `--ple-course-theme-*` CSS controls before adding a theme-specific override;
reserve explicit overrides for a measured exception such as the Grass palette. The stored anchors do
not need to change merely because the presentation should become stronger or quieter.

Ordinary text meets at least 5.5:1 against its rendered background, but a pair already meeting the
target should not be darkened toward maximum contrast without another need. Shared standard-theme
text tokens stay at or below 8.25:1 so normal presentation does not collapse toward black and white;
this ceiling does not apply to action states or the user-selected increased-contrast presentation.
The theme chooser and visual contact sheet must preview applied palette roles rather than presenting
three tiny swatches or an unrelated banner as the theme's primary identity.

Increased contrast is an account-backed presentation option. It strengthens text, focus, selected
states, and necessary boundaries while retaining the same theme hue family and course identity. It
does not change course data, question content, grading, assignment behavior, or authorization.
Forced-colors is automatic browser/operating-system behavior and remains independent of the stored
preference.

## Focus and accessibility

All presentations keep semantic HTML, programmatic labels, keyboard operation, readable text,
non-color status cues, and live announcements. Focus uses a two-part indicator sized to the focused
element: a visible inner edge plus a modest outer halo or offset. Do not ring the whole page when a
child control has focus.

Student Question Response Controls provide comfortable targets and a clear group label. Instructor pointer
workflows may use denser controls while preserving keyboard reachability and enough spacing to avoid
accidental activation. Reduced motion and forced-colors preferences remain honored.

## Content identity

Show names and titles first. Never show or announce UUIDs. Application routes and copyable links use
short typed references: `C-n`, `A-n`, `R-n`, and `W-n`; questions use one `AAA-BBBB` Crockford Base32
Question ID without a public version suffix. A public reference identifies a resource for a person;
it never grants access, and the server resolves it within the existing course, role, membership, and
ownership boundary. Assignment import and existing-assignment checklists carry groups of questions;
direct Question ID entry remains an occasional recovery and communication path.

## Validation

Use behavior tests for durable interaction and authorization contracts. Use computed browser styles,
screenshots, and human inspection for geometry, density, hierarchy, theme character, focus, and
responsive composition. Canonical evidence includes 1280 by 800 instructor pages, student pages at
both 1280 by 800 and 800 by 1280, a narrow-phone overflow guard, and standard plus
increased-contrast theme samples.
The accepted implementation evidence and page-level findings live in
[UI_DESIGN_REVIEW.md](UI_DESIGN_REVIEW.md). Historical instructor screenshot references live in
[INSTRUCTOR_PAGE_VISUALS.md](INSTRUCTOR_PAGE_VISUALS.md).
