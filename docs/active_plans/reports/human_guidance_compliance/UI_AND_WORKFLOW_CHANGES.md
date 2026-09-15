# UI and workflow changes

Temporary working report for the corpus-wide Human Guidance compliance pass.

## Instructor interface changes

- The dense Product Ribbon uses Courses, Questions, and Assessments with the exact task names listed
  in [TERMINOLOGY_AND_MODEL_CHANGES.md](TERMINOLOGY_AND_MODEL_CHANGES.md).
- Search Question Library and Browse Question Library remain distinct workflows.
- Required backed destinations remain visible for empty collections; unavailable future controls do
  not appear usable.
- Assessment composition and whole-Assessment settings remain separate editors.
- Blueprint Updates shows the current/source Revisions and reviewable differences. Existing
  Assessment changes require approval; a newly added Blueprint Assessment appears automatically as
  Unreleased.
- Student View keeps Instructor identity, is answer-free, and creates no Student Work.

## Student and shared shell

- Student work is Coursework collectively and uses the specific Assessment Type for each item.
- The Attempt shows one Question at a time, navigation to all Questions, saved status, stable
  geometry, and a separate Submit Assessment action.
- Optional response-input Enter behavior activates Save response; the whole Assessment Attempt has
  the separate submission action. Backend-owned documents cannot introduce a Student upload
  capability.
- Every signed-in role has a stable top bar, Profile menu with Sign Out, and permanent breadcrumb
  row. Profile is not Instructor-only.
- Students choose a provided playful avatar and have no upload capability; Instructors and Sysadmins
  may add a Profile image.
- Current API inventories identify missing Student and Sysadmin Profile routes as implementation
  gaps rather than narrowing the shared Profile requirement.
- Role cues use tomato red for Sysadmin, teal green for Instructor, and lavender/purple for Student,
  with visible labels so color never acts alone.
- Assessment Types retain their exact labels and Font Awesome icons. Course themes use biome/habitat
  names, a three-color palette, and an optional small centered 5:1 banner. The recommended banner
  authoring size is 1280 by 256 pixels; higher-resolution 5:1 images use the same responsive
  geometry across supported viewports.
- The former 6:1 page-width Course hero, 5:2 card crop, and dual-rendition design remain labeled
  implementation evidence; they no longer prescribe the product geometry.

## Consequence and evidence

Assessment Unrelease alone requires typing the Assessment title. Question and Blueprint archive
actions explain their effect and require clear confirmation, but Human Guidance does not require a
typed title for them.

Current screenshots and the generated Ribbon ledger still show legacy implementation labels. The
current visual guides and screenshot contract now identify those as gaps; durable regeneration
requires source/UI work outside this docs-only pass.

Completed loose plans that still describe an Instructor-only Profile control, separate top-bar Sign
Out, or old Ribbon tasks now begin with a direct historical-authority notice. They remain useful
implementation evidence without presenting those superseded decisions as current product intent.
