# Instructor page visuals

This gallery is the visual map of PLE's Instructor interface. It is captured from a working demo
environment with one coherent set of Blueprint Courses, Course Instances, questions, roster records,
and grades. The complete Instructor and Sysadmin page map uses the fixed 1280 by 800 CSS-pixel
desktop 16:10 viewport profile. Student profiles remain variable and use the maintained viewport
profiles declared below.

Blueprint Courses show reusable course-level content and structure. Published Blueprint Courses are
visible to all vetted Instructors; drafts are private to their owner and authorized collaborators.
Course Instances are created from exactly one Blueprint parent and are private to their current equal
Teaching Team Members and enrolled Students. Course Instance pages own deadlines, releases,
accommodations, grades, and delivery settings. No Blueprint page shows Student records or live
delivery state.

All people and records are fictional. Elena Rivera, Mary Okafor, Morgan, and the other seeded
personas are deterministic documentation identities, not real Roosevelt participants. Deterministic
fixture addresses in the reserved `example.invalid` domain are permitted test data; real email
addresses and real identifying records are prohibited in public evidence. The capture checks visible
and announced page text plus browser paths for UUID exposure before it writes an image.

## Page map

| Page                     | Example route                                                | What the view establishes                                                                                                        |
| ------------------------ | ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| Courses                  | `/`                                                          | Instructor home, choose or create a Blueprint, and create a Course Instance                                                      |
| Course Instances         | `/courses/C-1`                                               | Course Instance identity, local navigation, and assignment scanning                                                              |
| Blueprint Courses        | `/blueprint-courses`                                         | Reusable Blueprint list, publication state, and owned drafts                                                                     |
| Blueprint detail         | `/blueprint-courses/:blueprintCourseRef`                     | Ordered modules and assignments, revision, publication, and fork actions                                                         |
| Assignment overview      | `/instructor/courses/C-1/assignments/A-1`                    | Assignment home opened from the linked title                                                                                     |
| Student assignment page  | `/courses/C-1/assignments/A-1`                               | Question count, grade policy, feedback, and practice entry                                                                       |
| New assignment           | `/instructor/courses/C-1/assignments/new`                    | Empty assignment authoring state and Question Library entry points                                                               |
| Assignment Questions     | `/instructor/courses/C-1/assignments/A-1/questions`          | Title, ordered questions, pools, discovery, reuse, and server samples                                                            |
| Assignment Policies      | `/instructor/courses/C-1/assignments/A-1/policies`           | Instance instructions, release, delivery, lifecycle, access, and checks                                                          |
| Assignment Student view  | `/instructor/courses/C-1/assignments/A-1/student-view`       | Stable-identity, answer-free Student landing with Instructor identity active                                                     |
| Grading operations       | `/instructor/courses/C-1/assignments/A-1/grading-operations` | Assignment-local automated-grading attention and recovery actions                                                                |
| Students                 | `/instructor/courses/C-1/students`                           | Invitation, enrollment policy, pending invitation, and roster context                                                            |
| Gradebook                | `/instructor/courses/C-1/gradebook`                          | Compact Student-assignment progress without expanded raw records                                                                 |
| Grade settings           | `/instructor/courses/C-1/grade-settings`                     | Weighted categories, assignment membership, totals, and audited export                                                           |
| Course appearance        | `/instructor/courses/C-1/appearance`                         | Applied Course Instance palettes, banner settings, and live theme context                                                        |
| Question Library         | `/library`                                                   | Published Question views, Starred, Watched, Question Search, filters, Question IDs, and navigation to the separate My Question Drafts workspace |
| Question Details         | `/library/7K3-M9QP`                                          | Human-facing identity, source context, Question Statistics, and Student-facing prompt                                            |
| My Question Drafts       | `/workspace`                                                 | Draft Questions and the selected Draft Question                                                                                  |
| My Question Draft editor | `/workspace/W-1`                                             | QTI import entry and PLE Question JSON authoring                                                                                 |
| Live Demo sign-in        | `/sign-in`                                                   | Deployment-gated seeded Account selector for the disposable demo                                                                 |

The authentication completion pages, invitation redemption, and Assignment Attempt pages are outside this
Instructor-workspace gallery. The approved end-to-end teaching loop remains in
[INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md) and [STUDENT_GUIDE.md](STUDENT_GUIDE.md).

Student view is an inspection of the current live assignment in the Instructor session. It is
answer-free and creates no Assignment Attempt, Question Attempt, submission, receipt, grade, or enrollment. Ordinary Student
entry creates real graded work through the visible Student path, and the Instructor sees the result
in the Course Instance Gradebook. The Student view keeps the Instructor session and clearly points
to ordinary Student entry for graded validation.

The former screenshot capture workflow and its consumers are absent from the current
tree. This retained historical screenshot reference does not claim current acceptance. Keep private
Instructor evidence separate from public or Student evidence under
`docs/screenshots/`; a restored browser owner must create and review fresh
evidence before a current UI change can claim visual acceptance.

## Visual gallery

Course Instance pages use the Grass palette in standard presentation. This makes the gallery useful
for evaluating normal theme character as well as density, hierarchy, navigation, and page-level
composition. The mounted Live Demo entry is intentionally distinct from the planned email-code and
passkey authentication adapters.

<!-- screenshots:begin (managed by screenshot-docs) -->

![Instructor teaching operations groups](screenshots/instructor/teaching_operations/01_teaching_operations_groups.png)
![Instructor active roster](screenshots/instructor/course_management/01_instructor_active_roster.png)
![Instructor Gradebook with Student progress](screenshots/instructor/grading/01_instructor_gradebook.png)
![Instructor grade settings conflict recovery](screenshots/instructor/grade_settings_conflict/02_retry_saved.png)
![Instructor assignment delivery preview](screenshots/instructor/assignment_preview/01_schedule_entitlement.png)
![Instructor item pool preview](screenshots/instructor/item_pool_delivery/01_pool_preview.png)
![Instructor assignment Policies workspace](screenshots/instructor/assignment_workspace/01_assignment_policies.png)
![Instructor answer-free assignment Student view](screenshots/instructor/assignment_workspace/02_student_view.png)
![Instructor Question Library discovery evidence](screenshots/instructor/question_library_discovery/01_disclosed_evidence_laptop.png)
![Instructor Blueprint Course workspace](screenshots/instructor/reusable_curriculum/01_reusable_curriculum_workspace_laptop.png)
<!-- Historical capture path retained as immutable evidence; the product term is Blueprint Course. -->

![Instructor Blueprint adoption review](screenshots/instructor/curriculum_adoption/01_alpha_fork_review_laptop.png)
<!-- screenshots:end -->

## Refreshing historical screenshot references

The retired screenshot capture workflow and its publication command are absent from the
current tree. This gallery is historical screenshot reference, not current acceptance.

Any Instructor UI, viewport, typography, theme, or navigation change requires
a restored real-browser owner and a new human visual review before it can claim
visual acceptance. Behavior tests remain distinct evidence for interaction,
authorization, answer secrecy, and teaching semantics.
