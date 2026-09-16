# ADAPT UI comparison audit

Date: 2026-09-16. Status: ADAPT screenshot observations and comparison comments.

## Scope and authority

Evidence is the 21 ADAPT screenshots supplied in this conversation. Image numbers below refer
to that supplied sequence.
The images remain conversation attachments; this document creates no repository screenshot copies.
ADAPT's user population is user-provided context, not independently verified usage evidence.
Screenshots establish presentation, not usability-study results, keyboard support, or runtime behavior.

[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) owns PLE requirements.
[UI_UX_USABILITY_AUDIT.md](UI_UX_USABILITY_AUDIT.md) owns concrete PLE findings.
ADAPT provides comparison patterns for Instructor laptop workflows. Recommendations below
preserve PLE's vocabulary, identity, authorization, and Question Backend boundaries.

## Supplied image inventory

Images are not stored in this repository. These descriptions make the numbered references
understandable independently of the conversation, but do not replace the original visual evidence.

| Image | Surface and visible evidence |
| --- | --- |
| 1 | My Courses: eight Course rows with linked names, Shown Yes/No toggles, Term, reorder handles, and action icons; New Course and Import Course actions. |
| 2 | My Questions, H5P do not use folder selected: folder counts at left; HLA genotyping and blood typing rows at right; content filter, search, bulk actions, and Show Descriptions. |
| 3 | My Questions, Biochemistry Questions folder selected: 91-item count, wrapped Question titles, Public column, compact actions, and an inner results scrollbar. |
| 4 | New Question, Properties tab: title, description, Question/Exposition class, Public, folder, authors, license, source URL, and contextual help icons. |
| 5 | New Question, Primary Content tab: introductory explanation, expandable HTML Block, submission-type selector, and Auto-Grade Tech Block with WeBWorK selected. |
| 6 | My Favorites: Main folder with four Questions; compact result table, content/search controls, Show Descriptions, bulk actions, and row action icons. |
| 7 | Bulk Import: H5P importer selected among five formats; Public, Question/Collection level, folder, comma-separated H5P IDs, and Import action. |
| 8 | Classification Manager: Courses and assignments scope selected; open Course selector lists the user's Courses. Introductory copy describes bulk metadata updates. |
| 9 | Classification Manager: Upper-Level Introductory Biochemistry selected; open Assignment selector includes All assignments and chapter-named Assignments. |
| 10 | Classification Manager, Chapter 07 Enzyme Kinetics: Apply to field, Public, tag changes, owner, author, license, source URL, Update meta-tags, pagination, and four visible metadata rows. |
| 11 | Assignment Templates: two named Templates with Description and compact actions; New Template action above the table. |
| 12 | Search Questions, broad search: 256,460 reported Questions, numbered pagination, Per page 100, vertically stacked filters, and results beginning below the form. |
| 13 | Search Questions, native auto-graded filter: 39,228 reported Questions; technology subdivisions, Type chooser, classification filters, and a compact result table below. |
| 14 | Open Subject menu, upper portion: Biochemistry selected; broad, introductory, and specialized labels intermixed, from Abstract algebra through Introductory Chemistry. |
| 15 | Open Subject menu, lower portion: Operations research highlighted; mixed course and academic labels continue through Webwork Subject Area Templates. |
| 16 | Open Chapter menu: Atomic Structure and Periodicity highlighted; repeated Acid-Base Equilibria labels, numbered textbook chapters, topical names, and lexical chapter-number ordering. |
| 17 | Open Discipline menu: Chemistry - General highlighted; broad Disciplines and prefixed subdivisions, including Biology and multiple Chemistry categories. |
| 18 | Public Courses, Biology filter: title/author/school filters; result table with Discipline, Title, Author, School, Actions, and visible sort indicators. |
| 19 | Commons, Chemistry filter: title and description filters; Introduction to Inorganic Chemistry row with a long full description and view/download icons. |
| 20 | Commons, Biology filter: Microbiology and General Biology OpenStax rows with long descriptions; Update/Reset controls and view/download icons. |
| 21 | Frameworks: explanatory introduction and New Framework action; title, type, description, author, and actions for learning objectives, outcomes, skills, and taxonomies. |

## Useful comparison patterns

| Pattern | ADAPT evidence | Comparison comments |
| --- | --- | --- |
| Aligned collection tables | Images 1, 11, 18: Course, Template, and public Course rows | Use consistent columns for recognition, state, and actions. Compare several long records at 1280 by 800. |
| Compact secondary actions | Images 1-3, 6, 11: repeated action icons at row ends | Compact actions can reduce noise. Provide accessible names, focus-visible explanations, and recognizable primary actions. |
| Optional descriptions | Images 2, 3, 6: Show Descriptions control | Let Instructors expand relevance information while retaining a compact default list. Preserve the choice during navigation. |
| Local organization with counts | Images 2, 3, 6: folders and visible counts | Counts and personal organization aid recognition. Evaluate PLE's existing My Questions, Starred, Watched, and classification paths before proposing folders. |
| Scoped bulk metadata editing | Images 8-10: Course/Assignment or My Questions scope, fields, result table | Make the affected set and proposed changes recognizable before applying them. Preserve PLE's actual permissions and revision rules. |
| Separate authoring concerns | Images 4, 5: Properties and Primary Content tabs | Keep educational content central and supporting metadata easy to reach. Match PLE's current Question and Properties boundaries. |
| Brief contextual help | Images 4, 5, 7, 10: help icons alongside labels | Tooltips suit supplementary explanations; essential requirements stay visible. Longer material uses accessible disclosure. |
| Pagination and page-size choice | Images 12, 13: number range, ellipsis, and Per page | Support deliberate traversal of large collections while preserving filters, sort, and position. |
| Comparable public records | Image 18: title, author, school, discipline | Show metadata that helps judge relevance and provenance, within PLE's existing data and privacy boundaries. |

These comments identify useful patterns and tradeoffs for comparison.

## Friction to account for

- Images 12 and 13 devote most of the first viewport to vertically stacked filters before results.
  PLE's simple initial Search and dense results guidance remains the better fit for its stated goal.
- Images 3, 12, and 13 show inner result scrollbars alongside page scrolling. Evaluate whether
  constrained scrolling makes navigation harder in the actual task before adopting it.
- Images 19-21 put full paragraphs in table rows, making a few records consume substantial height.
  Use brief summaries with expandable details when full descriptions impede comparison.
- Images 1-3 show compact icon-only actions, including consequential actions close to routine ones.
  Compactness still needs clear names, adequate targets, and explicit consequences.
- Images 4 and 5 use long introductory explanations and several boxed sections. Use supporting
  prose where it helps the task, with content-sized sections and compact common paths.
- Images 14-17 show very long classification menus. A hierarchy alone does not make selection easy;
  narrowing, recognizable context, and consistent names matter.

## Sorting shown in ADAPT

Image 18 shows sort indicators on public Course table headings. Images 12 and 13 show pagination
and page-size controls for Question search, but do not clearly establish Question sorting behavior.
Source inspection could clarify which columns and collections ADAPT actually sorts.

Comparison comment: collection order should be visible and useful for the task, with filtering and
pagination preserving that order.

## Classification shown in ADAPT

- Images 14 and 15 show a Subject menu mixing broad areas such as Biology with Genetics,
  General Biology, introductory course labels, and method-oriented labels.
- Image 16 shows a Chapter menu mixing textbook chapter numbers, topical names, near-duplicates,
  and repeated names. Numbered chapters also appear in lexical order, such as Chapter 10 before
  Chapter 2.
- Image 17 shows a separate Discipline menu containing Biology and Chemistry alongside prefixed
  Chemistry subdivisions. The screenshot shows the menu contents, not the management permissions
  or relationship between Discipline and Question Subject.
- Images 18-20 use Discipline to narrow public Course and Commons records.
- Image 21 presents Frameworks for learning outcomes and taxonomies, a separate organizing concept.

Comparison comments: distinguish broad academic context from textbook-specific organization;
consistent names and scoped selections would make these menus easier to navigate. Searchable or
progressively narrowed selection could reduce scanning effort for large vocabularies. Duplicate
labels and inconsistent levels warrant investigation before treating a long menu as a useful taxonomy.
PLE's own hierarchy, ownership, and validation decisions belong in HG.

## Public catalog comparison

The user favors the overall setup of Public Courses, Commons, and Frameworks as possible
references for Blueprint Course discovery. The supplied screenshots show three different
introductory approaches:

| Catalog | What ADAPT presents | Comparison comment |
| --- | --- | --- |
| Public Courses, Image 18 | Heading, Discipline/title/author/school filters, and comparable result rows; no explicit catalog definition in the captured page. | A brief purpose statement could explain what public availability means and what users can do with a Course. |
| Commons, Images 19-20 | A search hint explaining partial title/description matching with Spanish I and Spanning the Universe examples. | Explain what the catalog contains before supplementary search instructions. Keep the search hint concise or disclose examples through contextual help. |
| Frameworks, Image 21 | Defines a Framework as a hierarchical knowledge map aligned to Questions, lists possible organizing concepts, and describes coverage and mastery uses. | The purpose explanation is useful. A shorter summary could preserve the definition and move fuller explanation into supporting help. |

Comparison direction: give related catalogs a consistent introductory structure: recognizable
heading, brief definition, available user actions, and contextual search help. Consistent structure
can make their different purposes easier to understand. Public Courses, Commons, and Frameworks
remain distinct concepts; these screenshots do not establish their complete publication rules or
the claimed mastery-tracking behavior.

The user's observation that ADAPT has one developer working alone provides context for incremental
refinement rather than a requirement to reproduce its choices. The catalog layout is the useful
comparison pattern; each PLE catalog still needs its own purpose and contract.
