# UI and workflow changes

Implementation findings for the shell and the Instructor, Student, and Sysadmin
interfaces. Open readings are recorded once in
[unresolved_or_ambiguous_items.md](unresolved_or_ambiguous_items.md).

## Shell and Instructor tasks

- Sign out is only the Profile menu command. Ribbon tabs and tasks do not repeat it
  for Student, Instructor, or Sysadmin.
- Frequent Instructor tasks are the linked Courses, Questions, and Assessments
  groups. Future destinations stay future and are not usable links.
- Instructor lists use dense semantic rows. A large Question Library row shows the
  title, Question ID, discipline, and author together.
- Public Blueprint search shows Course name, classification, adoptions, and students.
  It sorts by name, adoptions, and students, and restores the search on return.
- Course banners keep a 5:1 frame as the width changes.
- The shared visual tokens live in `src/style.css`. Record titles use the accent,
  and record rows stay compact.
- Breadcrumb trails keep intermediate Course and Assessment levels. A descendant
  page also keeps the selected Ribbon tier when that destination was missing, and
  adjacent crumbs do not repeat one href.

## Open readings in this area

Interface design has 24 open rows.

- Positions 103 and 114 ask whether the shipped task ribbons and the precision field
  console already meet the broad design sentences.
- Position 154 asks whether the role badge stays immediately left of the logo, or
  follows the later top bar order.
- Positions 215 and 225 ask whether a private Profile image must appear on other
  people's screens.
- Positions 269 and 270 ask whether Blueprint results and sorts must grow fields the
  summary view does not have.
- Positions 317 through 327 are the Question Library search notes, including hover
  preview, a new tab, advanced-search comparisons, and a Question ID poster.
- Position 388 asks whether the Student guidance line is a heading or a new surface.
- Positions 460, 466, 470, 472, and 475 are the Sysadmin menu, approval, settings,
  secondary navigation, and the unlocked Ribbon layout.

Interface design: 345 verified, 24 open, 4 not applicable.
