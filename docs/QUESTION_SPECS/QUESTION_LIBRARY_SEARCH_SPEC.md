# Question Library search specification

Search queries match content in the Library using ordinary words, structured filters,
and optional precise syntax. Authority:
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#search-question-library-interface).
Library Objects use this search; shared results are defined in
[QUESTION_LIBRARY_SPEC.md](QUESTION_LIBRARY_SPEC.md).

## Query scope

Search and Browse query the same Library Objects with the same access rules, field meanings,
and result contract. Text and structured filters constrain one combined result set. A query
with no matches returns no objects; it does not silently drop criteria or substitute unrelated
content. [QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md) owns filter semantics,
cross-kind behavior, combinations, and count scope.

The Search page's initial state, query activation, controls, and layout belong to
[UI_DESIGN_GUIDE.md](../UI_DESIGN_GUIDE.md#question-library-discovery). Shared RecordList and
PageFrame behavior is defined there rather than in this domain specification.

## Browse

Classification-based discovery uses the same matching rules as text search. Hierarchy validation,
explicit cross-Discipline Subject matching, and counts follow
[QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md).
For example, Biology -> Genetics, Numeric, and the text `mapping` constrain one combined query;
Browse introduces neither a separate Pool catalog nor different matching rules.

The Browse navigator and its interaction with Search belong to
[UI_DESIGN_GUIDE.md](../UI_DESIGN_GUIDE.md#question-library-discovery).

## Text and field syntax

Ordinary words need no syntax. Apply the field meanings in
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

The current default request size is 50 results; the API accepts page sizes from 1 through 250.
These are established API bounds, not a reason to permanently save search
results. The opaque continuation cursor is bound to the normalized search terms and filters, sort, and page
size. A changed query starts at its first page. Each successful fetch replaces the current page.

Use the counts and scope defined in
[QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md#counts-and-pagination),
including when a result page is empty.

## Current search implementation evidence

`GET /api/library-objects/search` currently serves combined Library search. The table records the
implemented request names and values. The request, route, client, and filter field use the
Library Object and Questions in no Pool vocabulary. The default object-kind filter remains
tentative; see the filter specification and the
[M24 implementation report](../archive/reports/QUESTION_SPEC_M24_LIBRARY_RESULTS.md).

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
requests Questions in no Pool plus Pools matching meiosis. The object-kind combination is a
tentative default candidate in HG. Exact shared request types and limits live in
[question_search.rs](../../crates/question_model/src/question_search.rs); response kinds are in
[library_search.rs](../../crates/question_model/src/library_search.rs). A type's current default
does not settle the default object-kind filter. See
[QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md#object-kind-and-questions-in-no-pool).

## Navigation and search lifetime

Navigation behavior and temporary interface state belong to
[UI_DESIGN_GUIDE.md](../UI_DESIGN_GUIDE.md#question-library-discovery).
Search prompts and results are not permanently stored for restoration. The search contract
provides neither saved-result snapshots nor a guarantee that an earlier result remains current.

A result observed earlier is not continuing permission to act on that object. Opening or changing
it uses current server authorization and state checks. Retaining a result does not preserve a
database snapshot or authorize background synchronization.
