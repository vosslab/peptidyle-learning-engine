# Instructor page visuals

This gallery is a historical visual reference for PLE's intended Instructor interface. Its retained
captures depict one coherent historical demo fixture with Blueprint Courses, Course Instances,
questions, roster records, and grades; they do not describe the current built-app Browser Surface.
The historical Instructor and Sysadmin page map uses the fixed 1280 by 800 CSS-pixel desktop 16:10
viewport profile. Student profiles remain variable and use the maintained viewport profiles declared
below.

## Current product correction

Do not use the historical routes, captions, or screenshots below as product
requirements. Current intent is in [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md):
Blueprints are Private, Public, or Archived; Private is owner-only; Course
Instances can adopt a Public Blueprint or start empty; generic teaching objects
are Assessments; current co-Instructors are equal; and Human Guidance does not
define Grade Categories, weighted Course Grade Schemes, or Instructor grading
operations.

In the historical product reference, Blueprint Courses show reusable course-level content and
structure. Its old publication/draft labels are superseded by the current
Private/Public/Archived lifecycle. Course Instances may now adopt a Public Blueprint or start empty
and are private to their current equal co-Instructors and enrolled Students. Course
Instance pages own deadlines, releases, accommodations, grades, and delivery settings. No Blueprint
page shows Student records or live delivery state.

All people and records are fictional. Elena Rivera, Mary Okafor, Morgan, and the other seeded
personas are deterministic documentation identities, not real Roosevelt participants. Deterministic
fixture addresses in the reserved `example.invalid` domain are permitted test data; real email
addresses and real identifying records are prohibited in public evidence. The historical capture
workflow checked visible and announced page text plus browser paths for UUID exposure before it wrote
an image.

## Page map

| Historical page reference | Historical example route                                     | What the historical view establishes                                                                                               |
| ------------------------- | ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| Courses                   | `/`                                                          | Instructor home, choose or create a Blueprint, and create a Course Instance                                                        |
| Course Instances          | `/courses/C-1`                                               | Historical Course Instance route with old generic-Assignment copy                                                                  |
| Blueprint Courses         | `/blueprint-courses`                                         | Historical Blueprint list with superseded publication/draft labels                                                                 |
| Blueprint detail          | `/blueprint-courses/:blueprintCourseRef`                     | Historical modules and generic-Assignment copy; current object is Assessment                                                       |
| Assessment overview       | `/instructor/courses/C-1/assignments/A-1`                    | Historical route spelling for the current Assessment object                                                                        |
| Student Assessment page   | `/courses/C-1/assignments/A-1`                               | Historical route spelling; Student navigation now calls the collection Coursework                                                  |
| New Assessment            | `/instructor/courses/C-1/assignments/new`                    | Historical route spelling for Assessment authoring                                                                                 |
| Assessment Questions      | `/instructor/courses/C-1/assignments/A-1/questions`          | Assessment Question Editor content                                                                                                  |
| Assessment Properties     | `/instructor/courses/C-1/assignments/A-1/policies`           | Assessment Properties Editor content                                                                                               |
| Assessment Student View   | `/instructor/courses/C-1/assignments/A-1/student-view`       | Answer-free Instructor preview; creates no Student Work                                                                             |
| Grading operations        | `/instructor/courses/C-1/assignments/A-1/grading-operations` | Retired status concept; it is not a current route or an Instructor grading action                                                  |
| Students                  | `/instructor/courses/C-1/students`                           | Invitation, enrollment policy, pending invitation, and roster context                                                              |
| Gradebook                 | `/instructor/courses/C-1/gradebook`                          | Compact Student-assignment progress without expanded raw records                                                                   |
| Grade settings            | `/instructor/courses/C-1/grade-settings`                     | Superseded mockup; Human Guidance does not define weighted categories or a Course Grade Scheme                                     |
| Course appearance         | `/instructor/courses/C-1/appearance`                         | Applied Course Instance palettes, banner settings, and live theme context                                                          |
| Question Library          | `/library`                                                   | Published Question views, Starred, Watched, Search Question Library, Browse Question Library, filters, and Question IDs             |
| Question Details          | `/library/AAAA-ZBBB`                                         | Human-facing identity, source context, Question Statistics, and Student-facing prompt                                              |
| My Draft Questions        | `/workspace`                                                 | Required current Question task; an implementation must show an honest empty state if not yet backed                                |
| My Draft Question editor  | `/workspace/W-1`                                             | Historical QTI import and PLE Question JSON authoring reference; unavailable in the current Browser Surface                        |
| Live Demo sign-in         | `/sign-in`                                                   | Deployment-gated seeded Account selector for the disposable demo                                                                   |

The authentication completion pages, invitation redemption, and Assessment Attempt pages are outside
this historical Instructor-workspace gallery. Their current role-owned routes are described in
[INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md), [STUDENT_GUIDE.md](STUDENT_GUIDE.md), and
[LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md). This gallery remains historical visual reference rather than
current workflow evidence.

In the historical reference, Student View is an answer-free inspection surface that creates no
Assessment Attempt, response, submission, grade, or enrollment. Historical ordinary
Student delivery then creates graded work that flows to the Instructor Gradebook. Current Student
delivery and Gradebook evidence are separate functional PLE workflows; these historic captures do
not demonstrate them.

`./devel/capture_screenshots.sh` rebuilds only manifest-listed current captures. This retained
historical screenshot reference does not claim current acceptance. Keep Instructor evidence under
`docs/screenshots/instructor/` and separate it from public or Student evidence; a fresh capture and
review remain required before a current UI change can claim visual acceptance.

## Current visual atlas

The historical Course Instance pages use the Grass palette in standard presentation. This makes the
gallery useful as a design reference for normal theme character, density, hierarchy, navigation, and
page-level composition. The current Live Demo runs the functional Instructor teaching surfaces
described in [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md); email-code and passkey authentication remain
required product capabilities and current implementation gaps.

[SCREENSHOT_ATLAS.md](SCREENSHOT_ATLAS.md#instructor) is the complete current
Instructor gallery. It groups Course operations, authoring, Blueprint Course,
Assessment release, and Gradebook states so related captures can be compared
without relying on retired path families.

## Refreshing historical screenshot references

The current capture command is `./devel/capture_screenshots.sh`; it rebuilds the current manifest
and generated atlas. Git history retains retired screenshots; this document does not keep broken
links to them as an active gallery.

Any Instructor UI, viewport, typography, theme, or navigation change requires a
fresh current capture and human visual review before it can claim visual acceptance. Behavior tests
remain distinct evidence for interaction, authorization, answer secrecy, and teaching semantics.
