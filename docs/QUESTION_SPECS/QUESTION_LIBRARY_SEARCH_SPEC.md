# Question Library search specification

Search helps Instructors find content in a large Library using ordinary words, visible filters,
and optional precise syntax. Authority:
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#search-question-library-interface).
Library Objects use this search; shared results are defined in
[QUESTION_LIBRARY_SPEC.md](QUESTION_LIBRARY_SPEC.md).

## Starting and refining

An Instructor may start directly in the spreadsheet-style results interface without a mandatory
simple-search landing page. Simple search submits into the same advanced interface, retaining
the entered terms and filters for refinement. Browse can enter it with classification choices.

Keep active terms, filters, and sort visible. Changing or clearing a filter updates this
search. An empty result retains those controls and explains that nothing matches; it does not
silently drop filters or show unrelated content. Changing display mode changes presentation,
not which Library Objects match.

[QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md) owns filter semantics,
cross-kind behavior, filter combinations, and count scope for both Search and Browse.

## Browse

Browse lets Instructors explore without knowing a search term. It enters and refines the same
Library discovery workflow, using the same Library Objects, access rules, filters, and result
display. Authority: [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#browse-question-library-interface).

1. Begin with Discipline and show useful available groupings and counts.
2. Offer Subjects associated with the selected Discipline.
3. Narrow to Topic and then Subtopic.
4. Offer Tags, Question Types, Bloom dimensions, and other useful existing filters.
5. Show matching Library Objects in the shared spreadsheet-style results.
6. Allow text search within those results, retaining the selected filters.

Use [QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md) for hierarchy validation,
explicit cross-Discipline Subject search, per-kind matching, and all counts. Present the
cross-Discipline choice visibly. Show when a selected group has no matches and keep a direct way
to loosen or clear its filters.

Use [QUESTION_LIBRARY_SPEC.md](QUESTION_LIBRARY_SPEC.md) for Compact, List, and Visual box modes,
shared fields, and item opening. Moving between Browse and text Search retains terms and filters
within the open workflow; the navigation and temporary search-lifetime rules below apply equally.

For example, an Instructor browses Biology -> Genetics, chooses Numeric, then searches `mapping`.
The same combined discovery query applies those choices using the filter specification; Browse
introduces neither a separate Pool catalog nor different matching rules.

## Text and field syntax

Ordinary words need no syntax. Make useful field syntax discoverable when needed, and present
options by priority so new users are not overwhelmed. Apply the field meanings in
[QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md#per-field-meanings).
Private source and Answer Keys are not exposed as search results.

| Input | Meaning |
| --- | --- |
| `meiosis` | Search the object's own searchable text/metadata |
| `"chromosomal inheritance"` | Require the exact phrase |
| `meiosis -mitosis` | Require the positive term and exclude objects matching the negative term |
| `discipline:biology` | Restrict by Discipline text |
| `subject:genetics` | Restrict by Subject text |
| `topic:"chromosomal inheritance"` | Restrict by Topic phrase |
| `subtopic:"x-linked recessive crosses"` | Restrict by Subtopic phrase |
| `tags:review` | Restrict by Tag |
| `type:numeric` | Question Type term |
| `author:"Elena Rivera"` | Author term |

Apply [QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md#combining-filters)
for positive terms, exclusions, structured filter combinations, and exact vocabulary selections.
The parser's established prefixes are in [library_search_terms.rs](../../crates/server/src/library_search_terms.rs). HG does not
choose Boolean-expression syntax, stemming, fuzzy matching, or a relevance-ranking algorithm;
those are not implied by the examples.

## Ordering and paging

Both object kinds belong to one result set with consistent sorting and paging. The concrete
orders, bounds, and cursor rules below describe current implementation choices; HG does not
prescribe those constants or limit future ordering choices to them.

Filter both kinds in one server result set before global sorting and paging. Do not merge two
independently paged searches in the browser. Current supported orders are `titleAscending`
(Title, then stable ID) and `publishedNewest` (publication/creation time, then stable ID). There
is no extra Pool-promotion tie-break rule.

The discovery page defaults to 50 results; requested size is 1-250, with ordinary controls for
50, 100, and 250. These are established API bounds, not a reason to permanently save search
results. The opaque continuation cursor is bound to the normalized search terms and filters, sort, and page
size. A changed query starts at its first page. Each successful fetch replaces the current page.

Use the counts and scope defined in
[QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md#counts-and-pagination),
including when a result page is empty.

## Current search implementation evidence

`GET /api/library-objects/search` currently serves combined Library search. The table records the
implemented request names and values. The request, route, client, and filter field use the
Library Object and Questions in no Pool vocabulary. They do not settle the tentative initial
filter choice; see the [M24 implementation report](../active_plans/reports/QUESTION_SPEC_M24_LIBRARY_RESULTS.md).

| Parameters | Values |
| --- | --- |
| `kind` | `both`, `questions`, `pools` |
| `questions` | `inNoPool`, `all`; applies to Question rows |
| `owner_account_id` | Canonical Account ID, when supplied |
| `text` | Current search text, at most 256 Unicode scalar values |
| `author_names`, `backends`, `tags`, `subjects`, `topics`, `question_types`, `question_licenses`, `capabilities` | Repeated values from the corresponding allowed set or normalized text |
| `discipline_uuid`, `subject_uuid`, `topic_uuid`, `subtopic_uuid` | Internal vocabulary selectors; never user-facing IDs or copyable content links |
| `cross_discipline` | Explicit inclusion of the selected Subject across Disciplines |
| `bloom_cognitive_process`, `bloom_knowledge_dimension` | Independent exact Bloom values |
| `authorship` | `any` or `authoredByCurrentAccount` |
| `sort` | `titleAscending` or `publishedNewest` |
| `cursor`, `page_size` | Opaque cursor and bounded page size |

For example, `GET /api/library-objects/search?kind=both&questions=inNoPool&text=meiosis&sort=titleAscending&page_size=50`
requests Questions in no Pool plus Pools matching meiosis. This is the tentative default candidate
in HG; the final default remains open. Exact shared request types and limits live in
[question_search.rs](../../crates/question_model/src/question_search.rs); response kinds are in
[library_search.rs](../../crates/question_model/src/library_search.rs). A type's current default
does not settle the initial interface default. See
[QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md#object-kind-and-questions-in-no-pool).

## Navigation and search lifetime

Opening a result list item opens a new tab or window, leaving the current page and position
available. Ribbon navigation remains in the current tab; confirm before discarding an existing
search and its results. Leave an empty search page directly. Buttons perform their labeled action.

Search prompts and results are never permanently stored for restoration. Keeping the original
open page is temporary working state, not a saved search service. Do not add database snapshots,
browser-persistent archives, or a restore-old-results promise. The user's reason is database size,
server cost, and concern that results can become stale. No WASM search-memory design was approved.

A result observed earlier is not continuing permission to act on that object. Opening or changing
it uses current server authorization and state checks. A page left open for months is not a
four-month-old database snapshot. The exact choice of freshness indicators or refresh timing is
not settled; do not introduce background synchronization to fill that gap.
