# PLE interface design review

## Outcome

PLE presents one teaching product with stable page geometry, role-specific
workspaces, honest empty states, and visible keyboard-operable actions.
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) is the interface authority;
[UI_DESIGN_GUIDE.md](UI_DESIGN_GUIDE.md) provides the working design contract.

Older screenshots, routes, and mockups remain evidence about a previous build.
They do not preserve old Product Ribbon tabs, generic Assignment terminology,
Blueprint draft/publication states, Grade Categories, response-level
finalization, or Instructor grading operations.

## Current findings

| Area | Current resolution |
| --- | --- |
| Global shell | Keep stable Ribbon, Context Row, Task Row when applicable, breadcrumbs, page heading, and content geometry. Sign Out belongs in the Profile menu. |
| Instructor primary navigation | Courses, Questions, and Assessments. Do not split Question Library or Blueprint Courses into competing primary tabs. |
| Course tasks | My Blueprint Courses, My Active Courses, My Inactive Courses, and Search Public Blueprint Courses. |
| Question tasks | My Questions, My Draft Questions, Starred, Watched, Search Question Library, and Browse Question Library. Search and Browse remain distinct actions. |
| Assessment tasks | Assessments Due Soon and My Assessment Templates. |
| Assessment editor | Use Assessment Question Editor for composition and Assessment Properties Editor for settings. An Edit Number is not a Revision. |
| Student work | Use Coursework collectively and the specific Assessment Type for an item. Present one Question at a time with navigation to all Questions and saved status. |
| Submission | Save complete responses without grading disclosure; submit the whole Assessment Attempt with one clear action. |
| Empty collections | Keep required backed destinations visible and explain how to create the first item. Do not offer unavailable future controls as usable. |
| Student View | Keep Instructor identity and authority; show an answer-free preview and create no Student Work. |
| Gradebook and roster | Show authorized point-based Assessment scores, using the highest submitted Attempt. PLE has no separate Question weights, Grade Categories, weighted categories, Course Grade Scheme, or Course percentage calculation. |
| High-consequence actions | Put Assessment Unrelease, Published Question Archive, and Blueprint Course Archive in a Danger Zone. Unrelease requires the typed Assessment title; archive actions explain their effect and require clear confirmation. |
| Responsive layout | Preserve hierarchy, reading order, keyboard access, and visible saved/submission state without horizontal page overflow. |

## Assessment Type appearance

The five Types are Regular Assignment, Practice Question Assignment, Bonus
Assignment, Quiz, and Exam. Assignment is not an object, category, or parent
Type. Type appearance can help scanning but must preserve text labels and
contrast.

## Historical visual evidence

[SCREENSHOT_ATLAS.md](SCREENSHOT_ATLAS.md) groups the current captured files,
many of which still show older implementation terminology. The atlas must label
those discrepancies. [INSTRUCTOR_PAGE_VISUALS.md](INSTRUCTOR_PAGE_VISUALS.md)
and [STUDENT_PAGE_VISUALS.md](STUDENT_PAGE_VISUALS.md) explain how to interpret
historical captures.

Pixels do not prove authorization, submission semantics, grading secrecy, or
backend ownership. Those need behavior evidence. A screenshot becomes current
visual evidence only after fresh capture at the required viewport and human
review.

## Visual and accessibility measurements

Retain the measured course palettes, focus visibility, forced-colors support,
reduced-motion behavior, compact choice rows, and variable Student viewport
profiles. Shared design tokens should own shell width, gutters, vertical rhythm,
control size, responsive navigation, and Course-theme surfaces.

The primary no-mouse path uses native controls, Tab/Shift+Tab, Enter for links,
and Space for choices and buttons. Extensions such as arrows, digits, or Enter
from a response field never replace visible controls.

## Validation boundary

Use type/lint/format checks for source integrity, focused browser tests for
role navigation and interactions, no-transport assertions for denied routes,
and fresh screenshots for rendered review. Do not turn a current route count,
DOM ancestry, exact Tab count, or screenshot inventory into a permanent product
test.
