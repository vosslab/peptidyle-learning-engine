# Instructor guide

This guide follows the supported local no-email teaching loop: create a course, add a configured
fictional student through a visible roster control, build a timed Mastery assignment from the
published problem corpus, and confirm learning in the gradebook. Start the local system first with
[USAGE.md](USAGE.md).

All people and course records shown in these captures are simulated. The fixed labels
`Dr. Fake Professor`, `Mary Fake Student`, and `Jack Fake Student` are intentionally unmistakable
placeholders.

<!-- screenshots:begin (managed by screenshot-docs) -->

![Instructor course overview showing the Assignments page and course-management navigation](screenshots/instructor_course_overview.png)

![Instructor roster showing Mary Fake Student and Jack Fake Student as active local course members](screenshots/instructor_roster_active_student.png)

![Instructor published problem catalog showing four Genetics Chapter 1 P-n-vn identities and Copy ID controls](screenshots/instructor_problem_catalog.png)

![Instructor assignment editor showing four selected Genetics Chapter 1 immutable versions and Mastery run policies](screenshots/instructor_assignment_settings.png)

![Instructor Assignment created confirmation showing Genetics Chapter 1 Practice and its Open assignment link](screenshots/instructor_assignment_created.png)

![Instructor Genetics Chapter 1 Practice gradebook showing Best and Latest at 100 percent, two completed runs, and keyboard focus on View run history](screenshots/instructor_gradebook_mastery_loop.png)
<!-- screenshots:end -->

## Before you begin

- Launch the normal local stack and open its loopback URL.
- Use the instructor value from the ignored `containers/local-login.txt` file.
- Use local-development identity only for this pilot. It does not require, configure, or test email
  delivery. Fastmail is a future provider decision, not a configured part of this teaching loop.

## Create a course

1. Sign in through the visible local-development form.
2. Enter a descriptive title in **Course title**.
3. Activate **Create course**.
4. Open the new course card.

The created course is a real PostgreSQL-backed course. It is not a browser fixture or an API-only
arrangement.

## Add the local student

1. Open **Students** from the course navigation.
2. Activate **Add Mary Fake Student**. The page also offers **Add Jack Fake Student** when a second
   simulated learner is useful.
3. Confirm that the focused roster row reports **active**.

These buttons resolve only the configured local fictional learners and add canonical course
membership. They are intentionally not an alias-entry, invitation, or email-activation workflow.
Production enrollment and email identity remain separate work.

## Build an assignment

1. Return to the course and open **New assignment**.
2. Enter the assignment title.
3. In **Published problem catalog**, find each of the four Genetics Chapter 1 questions. Confirm
   its visible human-readable identity has the form `P-<number>-v<version>`, then use **Copy ID**.
4. Paste all four copied IDs, one per line or comma-separated, into **Add by question ID** and
   activate its add button. This is a multi-ID paste: do not substitute UUIDs or add questions one
   at a time from a private tool.
5. Confirm the selected list contains exactly those four immutable published versions: WeBWorK MC,
   WeBWorK MATCH, PLE flat MC, and PLE flat MATCH.
6. Confirm **All questions correct**, **Highest run score**, and **Allow unlimited practice**.
7. In **Time limit for each practice run**, leave **Timed** selected and confirm **Minutes per
   practice run** is `15`.
8. Activate **Create assignment** and open the resulting course assignment.

Only corpus publication is arranged outside the browser. Course creation, roster activation, ID
copy/paste, exact-version selection, timing, and assignment construction use visible instructor
controls. If an ID is malformed, unavailable, unauthorized, or already selected, the editor keeps
the pasted text and selected questions unchanged so it can be corrected and tried again.

## Review learning

After the student completes and repeats the assignment, open **Gradebook** and expand **View run
history** for the assignment row. The captured pilot shows Best and Latest at 100 percent, Completed
at 2, and two completed run-history entries.

The companion [STUDENT_GUIDE.md](STUDENT_GUIDE.md) follows the learner path. The platform keyboard
contract is documented in
[NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md).
