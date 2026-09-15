# Instructor guide

## Current local Live Demo

The local Live Demo lets a reader enter as the seeded Elena Rivera Instructor
Account. The selector creates the ordinary authenticated session; it supplies
neither Course authority nor a browser role claim.

The current implementation exposes Question Library and authoring, Blueprint
Courses, Course Instances, roster import, Assessment editing/release,
answer-free Student Work inspection, Gradebook evidence, and protected Course
Invitation export. Some current routes and labels still use `assignment`; that
is an implementation gap, not current product vocabulary.

Start the stack through [USAGE.md](USAGE.md). Current route evidence is in
[API_CONTRACTS.md](API_CONTRACTS.md), and product intent is in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md).

## Teaching workflow

1. Create a Private Blueprint Course, use a Public Blueprint, or create an
   empty Course Instance.
2. When adopting a Blueprint, select one exact Public Blueprint Revision.
3. Review offered Blueprint Revisions before applying changes to existing
   daughter-Course Assessments. Newly added Blueprint Assessments arrive
   automatically as Unreleased Assessments.
4. Invite another Instructor as an equal co-Instructor when needed; the creator
   or first Instructor has no extra authority.
5. Add Students through the Course roster relationship.
6. Create and edit Course Instance Assessments using Published Questions and
   Question Pools.
7. Run Assessment Release Validation, correct every reported issue, and release
   only after it passes.
8. Use Student View for answer-free preview and the Gradebook for authorized
   results.

Instructors do not grade, regrade, or retry Student responses. The selected
Question Backend grades automatically and returns an immutable credit fraction.

## Assessment changes and Unrelease

An Assessment is current Course teaching configuration, not a Revision family.
An Edit Number may prevent stale saves. Fixed Questions and Pool selections
retain exact Revision evidence so later publication does not silently change an
existing Attempt.

The high-consequence **Unrelease** action requires the exact Assessment title.
It returns the Assessment to Unreleased and atomically deletes all Student Work
for that Assessment. It preserves the Assessment definition, Course
relationships, and shared Published Questions and Pools.

Changing a Question's point value recalculates scores from stored immutable
credit fractions. It does not regrade responses.

## Navigation

The Instructor Ribbon uses Courses, Questions, and Assessments. The task rows
and exact names are defined in [UI_DESIGN_GUIDE.md](UI_DESIGN_GUIDE.md). Sign
Out is in the Profile menu. Empty collections keep real destinations visible
and explain how to create the first item.

## Accessibility

Visible controls and keyboard behavior follow
[NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md).
Screenshots are rendered evidence only after fresh capture and review; they do
not establish authorization or current product intent by themselves.
