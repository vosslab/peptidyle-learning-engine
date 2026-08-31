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
3. Row or question title: supports scanning inside that group.
4. Label, value, help, and metadata: ordinary text with weight used sparingly.

Page titles should generally occupy one line. Prefer normal or medium weight for ordinary labels and
values; reserve bold for the few words that establish structure. Status text should describe a useful
state or recovery action. Do not render confirmations such as "Question content ready" when the
visible content already proves the state.

## Space and layout

- Global shell: use nearly the full viewport, with bounded edge gutters rather than a narrow centered
  column. Reading-heavy student prose may use a local readable measure inside the wider shell.
- Instructor pages: prioritize horizontal scanning, compact rows, sticky local actions when useful,
  and one coherent primary work surface. Avoid placing a browse catalog below an unrelated narrow
  sidebar.
- Student questions: keep prompt, response, feedback, navigation, and timer in one visual sequence.
  On large screens, use width to improve line length and grouping, not to separate the question from
  its answer across a long eye movement.
- Vertical rhythm: use small, consistent steps. Add space between groups; do not pad every child.
- Footer: global slogans do not occupy a permanent band on teaching workspaces.

### Adaptable layout tokens

Treat density as a design-system setting, not a collection of page-specific numbers. Shared CSS
custom properties in `src/style.css` own the geometry most likely to change after observation:

| Token family                                                  | Controls                                                                      |
| ------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `--ple-shell-*`                                               | Maximum application width, viewport gutters, header height, and shell padding |
| `--ple-layout-gap`, `--ple-section-gap`, `--ple-compact-gap`  | Page columns, group rhythm, and dense local rhythm                            |
| `--ple-panel-padding`, `--ple-row-padding-*`                  | Work surfaces, catalog rows, tables, and editor rows                          |
| `--ple-control-min-height`, `--ple-dense-row-min-height`      | Shared controls and compact instructor records                                |
| `--ple-reading-max-inline`, `--ple-catalog-window-block-size` | Reading measure and catalog working height                                    |
| `--ple-instructor-*-min-inline`, `--ple-filter-*-min-inline`  | Assignment columns and catalog-filter allocation                              |
| `--ple-*-table-min-inline`, `--ple-*-block-size`              | Deliberate overflow thresholds for dense data and bounded lists               |
| `--ple-course-scope-*`, `--ple-course-theme-*`                | Course canvas extent, inset, color washes, identity rail, and surface fade    |
| `--ple-mobile-nav-*`                                          | Compact single-row phone navigation without changing its semantics            |

Adjust these tokens first when evidence supports a density change. Page styles may derive small
differences with `calc()`, but should not duplicate the governing measurement. Breakpoints are
behavior thresholds rather than distances: change one only when the composition actually stops
working, and validate 1280 by 800 plus the student tablet and phone targets after any token change.

## Navigation

Global navigation, course navigation, assignment progress, and page actions are different layers and
must look different. Global navigation is quiet and persistent. Course navigation sits inside the
course identity surface and clearly marks the active section. Assignment progress communicates
sequence and state. Page actions live with the content they affect.

For an instructor, **Courses is the home workspace**: it lists recognizable courses and starts a new
one. Library owns published-question discovery, Workspace owns drafts, and Account owns personal
settings. Do not add a generic Dashboard dropdown that merely duplicates those destinations or hides
them behind another navigation step. Once an instructor opens a course, its local navigation owns
assignments, students, gradebook, and appearance. Add a future dashboard only when it answers a
distinct cross-course monitoring task that these object-centered workspaces cannot answer directly.

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

The shared `THEME_MIX` recipe in `theme_catalog.ts` owns the projection percentages. Change that
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

Student response controls provide comfortable targets and a clear group label. Instructor pointer
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
[UI_DESIGN_REVIEW.md](UI_DESIGN_REVIEW.md). The current instructor surface corpus lives in
[INSTRUCTOR_PAGE_VISUALS.md](INSTRUCTOR_PAGE_VISUALS.md).
