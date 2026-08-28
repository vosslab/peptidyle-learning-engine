# Instructor page visuals

This gallery is the visual map of PLE's instructor interface. It is captured from a working demo
environment with one coherent set of sample courses, questions, roster records, and grades. The
complete Instructor and Sysadmin page map uses the fixed `laptop` evidence profile at exactly 1280
by 800 CSS pixels in a desktop 16:10 viewport. Student profiles remain variable and use the
maintained profiles declared by the current screenshot corpus.

All people and records are fictional. Elena Rivera, Mary Okafor, Morgan, and the other seeded
personas are deterministic documentation identities, not real Roosevelt participants.
Deterministic fixture addresses in the reserved `example.invalid` domain are permitted test data;
real email addresses and real identifying records are prohibited in public evidence.
The capture checks visible and announced page text plus browser paths for UUID exposure before it
writes an image.

## Page map

| Page                    | Example route                                          | What the view establishes                                                                                                                                                |
| ----------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Courses                 | `/`                                                    | Instructor home, course creation, and recognizable course list                                                                                                           |
| Course assignments      | `/courses/C-1`                                         | Course identity, local navigation, and assignment scanning                                                                                                               |
| Assignment overview     | `/instructor/courses/C-1/assignments/A-1`              | Assignment-local Overview opened from the linked assignment title                                                                                                         |
| Learner assignment page | `/courses/C-1/assignments/A-1`                         | Question count, grade policy, feedback, and practice entry                                                                                                               |
| New assignment          | `/instructor/courses/C-1/assignments/new`              | Empty assignment authoring state and catalog entry points                                                                                                                |
| Assignment home         | `/instructor/courses/C-1/assignments/A-1`              | Assignment status, readiness, and the next focused teaching action                                                                                                       |
| Assignment Questions    | `/instructor/courses/C-1/assignments/A-1/questions`    | Title, ordered questions, pools, discovery, reuse, and server samples                                                                                                    |
| Assignment Policies     | `/instructor/courses/C-1/assignments/A-1/policies`     | Learner instructions, release, delivery, lifecycle, access, and delivery-check actions                                                                                   |
| Assignment Student view | `/instructor/courses/C-1/assignments/A-1/student-view` | Stable-identity, answer-free learner landing while the Instructor identity remains active                                                                            |
| Grading operations      | `/instructor/courses/C-1/assignments/A-1/grading-operations` | Assignment-local automated-grading attention, metadata-only recovery rows, and retry actions                                                                            |
| Students                | `/instructor/courses/C-1/students`                     | Invitation, enrollment policy, pending invitation, and roster context                                                                                                    |
| Gradebook               | `/instructor/courses/C-1/gradebook`                    | Compact learner-assignment progress without expanded raw records                                                                                                         |
| Grade settings          | `/instructor/courses/C-1/grade-settings`               | Weighted categories, assignment membership, letter bands, totals, and audited export                                                                                     |
| Course appearance       | `/instructor/courses/C-1/appearance`                   | Applied course palettes, banner settings, and live theme context                                                                                                         |
| Question library        | `/library`                                             | Full-width search, filters, Question IDs, and published results                                                                                                          |
| Question detail         | `/library/7K3-M9QP`                                    | Human-facing identity, source context, and learner-facing prompt                                                                                                         |
| Workspace               | `/workspace`                                           | Private draft list and the currently selected draft workspace                                                                                                            |
| Question editor         | `/workspace/W-1`                                       | QTI import entry and native flat-question authoring                                                                                                                      |
| Account                 | `/account/security`                                    | Optional contrast preference, passkeys, and sign-in email settings                                                                                                       |
| Curriculum adoption     | `/instructor/courses/:courseRef/curriculum`            | Blueprint/Alpha source selection and instantiation, rollover, DST-aware term shift, import inspection, fast-forward, divergence recovery, and immutable receipt evidence |
| Independent Alpha copy  | `/curriculum/:curriculumRef`                           | Review and apply an independent Alpha proposal with retained source lineage                                                                                              |

The authentication completion pages, invitation redemption, and student run pages are outside this
instructor-workspace gallery. The approved end-to-end teaching loop remains in
[INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md) and [STUDENT_GUIDE.md](STUDENT_GUIDE.md).

Student view is an inspection of the current live assignment in the Instructor session. Ordinary demo
Student entry creates real graded work through the visible learner path, and the Instructor sees the
resulting score and authorized evidence in the gradebook. This gallery documents the Instructor and
Sysadmin desktop composition; the variable Student profiles are documented by the current corpus.

`tests/e2e/browser_screenshot_corpus.json` owns the committed artifact corpus.
`tests/playwright/ui_corpus_manifest.ts` and
`tests/e2e/e2e_browser_screenshot_contract.py` strictly consume that source.
A screenshot is acceptance evidence only after a fresh capture and inspection;
the retained gallery does not claim that a current implementation has passed its acceptance gate.
Keep private instructor evidence separate from public or learner evidence under `docs/screenshots/`.

## Visual gallery

The course-scoped pages use the Grass palette in standard presentation. This makes the gallery useful
for evaluating normal theme character as well as density, hierarchy, navigation, and page-level
composition. The Account view shows where increased contrast can be selected without changing the
course theme or teaching behavior.

<!-- screenshots:begin (managed by screenshot-docs) -->

![Instructor teaching operations groups](screenshots/instructor/teaching_operations/01_teaching_operations_groups.png)
![Instructor active roster](screenshots/instructor/course_management/01_instructor_active_roster.png)
![Instructor gradebook with learner progress](screenshots/instructor/grading/01_instructor_gradebook.png)
![Instructor assignment access preview](screenshots/instructor/assignment_access/01_access_preview_allowed.png)
![Instructor grade settings conflict recovery](screenshots/instructor/grade_settings_conflict/02_retry_saved.png)
![Instructor assignment delivery preview](screenshots/instructor/assignment_preview/01_schedule_entitlement.png)
![Instructor item pool preview](screenshots/instructor/item_pool_delivery/01_pool_preview.png)
![Instructor assignment Policies workspace](screenshots/instructor/assignment_workspace/01_assignment_policies.png)
![Instructor answer-free assignment Student view](screenshots/instructor/assignment_workspace/02_student_view.png)
![Instructor automated-grading operations recovery](screenshots/instructor/automated_grading_recovery/01_instructor_operation_laptop.png)
![Instructor Gradebook after automated-grading recovery](screenshots/instructor/automated_grading_recovery/02_instructor_gradebook_laptop.png)
![Instructor catalog discovery evidence](screenshots/instructor/catalog_discovery/01_disclosed_evidence_laptop.png)
![Instructor problem curation workspace](screenshots/instructor/problem_curation/01_curation_workspace_laptop.png)
![Instructor reusable curriculum workspace](screenshots/instructor/reusable_curriculum/01_reusable_curriculum_workspace_laptop.png)
![Instructor curriculum adoption review](screenshots/instructor/curriculum_adoption/01_alpha_fork_review_laptop.png)
![Account security with passkey and presentation preference](screenshots/shared/account/01_account_security_passkey.png)
<!-- screenshots:end -->

## Refreshing the corpus

Run the repository-owned publication gate from the repository root whenever an
instructor UI, corpus, or viewport change requires fresh visual evidence:

```bash
./capture_screenshots.sh
```

The gate uses the fixed real-stack browser owner, stages the dynamic
manifest-owned corpus, verifies origin and provenance, and atomically publishes
the resulting `docs/screenshots/` artifacts. `./all_test.sh` exercises the
same stack's behavior and contract gates without rewriting documentation
assets.

Review the regenerated images together after shared layout, theme, typography, or navigation changes.
Behavior-focused browser tests remain the authority for interaction, authorization, answer secrecy,
and teaching semantics; these images are visual evidence for composition and current appearance.
The accepted B2 evidence covers the source/destination distinction, preview-before-apply, DST
correction, keyboard focus, recovery, privacy, and contrast at the canonical 1280 by 800 Instructor
profile. Accepted T6 evidence adds the focused Policies and answer-free Student-view surfaces. G1 is
the current active package and retains its own final-material-tree Validation gate. The two
automated-grading recovery images above are connected visual evidence for G1's Instructor operation
and Gradebook branch, pending final package closeout; they do not by themselves close G1 acceptance.
