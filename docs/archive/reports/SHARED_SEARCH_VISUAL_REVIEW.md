# Shared search visual review

> Historical review evidence. The phone Library and square picker captures described below
> were removed from the published corpus after the user corrected their scope. Instructor
> and Sysadmin screenshots follow the Human Guidance laptop target of 1280x800.
> All results below describe the historical M15 review, not current-worktree acceptance.
> Narrow/square findings are superseded and are not requirements for the current corpus.


## Scope and method

Independent image review for M15 of
`read-docs-active-plans-active-shared-sea-peaceful-mountain.md`. I inspected the supplied PNG
captures directly at their native sizes. This is a rendered-capture review, not a claim of full
accessibility or browser-interaction coverage.

Criteria used during the historical M15 review (the narrow-screen criterion was later withdrawn):

- useful information density at 1280x800;
- Compact, List, and Visual boxes show the same useful result fields;
- Questions and Question Pools are easy to tell apart;
- the active Library default chip is visible; and
- the page fits a narrow screen.

## Evidence reviewed

| Capture | Native size | Surface visible | Evidence status |
| --- | ---: | --- | --- |
| `pool-detail.png` | 1280x800 | Pool detail page | adequate for high-level Pool identity and metadata review |
| `question-picker.png` | 1280x800 | Question picker dialog with a selected Question | adequate for dialog density and Question-row review |
| `assessment-content-picker.png` | 1280x800 | Assessment content picker with mixed Question and Pool cards | adequate for this picker's mixed-card review |
| `results-library-compact-mixed-default-restored.png` | 1280x800 | Library Compact with mixed result rows | adequate for Compact parity comparison |
| `results-library-list-mixed-default.png` | 1280x800 | Library List with mixed result rows | adequate for List parity comparison |
| `results-library-visual-mixed-default.png` | 1280x800 | Library Visual boxes with mixed result cards | adequate for Visual parity comparison |
| `results-library-narrow-default-chip-pool.png` | 390x844 | Library Compact Question and Pool rows at narrow width | adequate for narrow mixed-result fit |

These are the post-correction test/harness captures; the capture replay passed 4/4. They do not
replace the normal Live Demo evidence below.

## Historical M15 Live Demo evidence

I also inspected the normal focused screenshot corpus from the then-current client bundle
`78700f7e` at origin `8292`. Its focused capture passed 8/8. The supplied files are:

- `questions-search-shared_search_default.webp` (1280x800): default Library view with the
  visible no-Pool chip and 49 current Question results.
- `questions-search-shared_search_all_questions.webp` and `_pools_only.webp` (1280x800): the
  All Questions view and a current Pool row, respectively; the Pool-only scene makes the real
  Pool's identity, description, owner, member count, type, backend, calculated license, and link
  readable in the normal application shell.
- `questions-search-shared_search_compact.webp`, `_list.webp`, and `_visual.webp` (1280x800):
  each selected display mode is rendered by the normal Live Demo page.
- `questions-search-shared_search_default_phone.webp` (393x852): the normal narrow Library
  page, including the visible default chip and width-fitting controls.
- `questions-search-shared_search_pool_detail.webp` (1280x800): the normal Pool detail route
  with its identity, description, calculated license, member count, and Instructor support form.

No normal capture in that review has a mixed row set in each display mode, so the M15 harness captures
remain the evidence for three-mode mixed-row parity and narrow Pool layout. The two evidence sets
agree on the Library shell, default chip, controls, Pool row, and Pool-detail presentation.

## Historical M15 Live Demo picker evidence

I inspected the post-correction focused captures from then-current client bundle `c05d0446`; their
focused replay passed 4/4.

- `courses-assessments-content-picker-laptop.webp` (1280x800) and `_square.webp` (800x800)
  show an opaque Assessment content-picker surface, readable heading and explanatory text,
  controls, default membership chip, result rows, and a readable dimmed page backdrop.
- `courses-blueprint-question-picker-laptop.webp` (1280x800) and `_square.webp` (800x800) show
  the Question picker with the same opaque dialog treatment and loaded Question rows. The square
  capture has a wrapped `Close picker` label but it remains legible and does not overlap any
  control.

### Resolved: Assessment picker dialog token failure (medium)

The implementation team reported that the first actual Live Demo Assessment-picker capture
exposed undefined dialog CSS tokens, producing a transparent, overlapping dialog surface. The
historical `c05d0446` captures provide direct visual evidence of the correction: the dialog and
backdrop are opaque, controls and result rows are readable, and no overlap is visible at either
desktop or square size. **Resolved.**

## Observed facts

- The full Compact capture visibly retains the active `Question membership: Questions in no Pool`
  chip alongside the submitted `Search: protein` chip.
- The Compact, List, and Visual boxes captures show the same Question and Pool titles,
  descriptions, shared identity information, kind-specific metadata, and open links.
- Each display mode makes object kind immediately clear through the Pool's `Library object` /
  `Question Pool` labels, member count, owner, type, backend, and calculated license.
- The narrow mixed-result capture fits both kinds, all visible Pool metadata, links, and paging
  controls in a 390px viewport without clipping or horizontal scrolling.
- The Pool detail capture gives the Pool a clear page title and identifies it as a Published
  Question Pool. Discipline, type, backend, Pool ID, edit number, calculated license, member
  count, and tags are readable before the editor fields.
- The Question picker capture uses a contained dialog and keeps its selected-Question count,
  filters, view choices, row identity, revision, inspect action, and paging in one 1280x800
  viewport. Its visible result is deliberately Question-only, as expected for pool membership.
- The Assessment content picker uses the same default-membership chip and shows a selected
  Question plus an unselected Pool together. Its Visual boxes cards distinguish the kinds through
  both content and labels: the Pool card says `Library object` / `Question Pool` and gives owner,
  member count, type, backend, and calculated license.

## Criterion findings

| Criterion | Result | Basis |
| --- | --- | --- |
| 1280x800 density | Pass | All three modes show the mixed rows at readable density without visible horizontal crowding. |
| Same useful fields in all three modes | Pass after correction | Revised Compact renders the Question and Pool descriptions shown by List and Visual boxes. |
| Questions and Pools easy to tell apart | Pass | All three modes use titles, `Question Pool` identification, and Pool-specific metadata to differentiate kinds. |
| Active default chip visible | Pass in the Library's full Compact capture | The default no-Pool chip is visible. It is not in the result-position captures because they are vertically scrolled. |
| Narrow-width fit | Historical only; withdrawn from acceptance | The revised narrow capture includes both kinds with their metadata and links, without visible clipping or horizontal scroll. |
| Pool detail clarity | Pass | Pool identity and metadata are direct and readable. |
| Question picker dialog density | Pass | Historical desktop and square Live Demo captures show a readable opaque dialog and controls. |
| Assessment picker dialog and mixed-card distinction | Pass after correction | Historical desktop and square Live Demo captures show an opaque readable dialog; M15 mixed cards have distinct labels and kind-specific metadata. |

## Findings and recommendations

### Resolved: Compact omitted descriptions present in the other two modes (medium)

The first full-size Compact capture omitted the Question and Pool descriptions shown in List and
Visual boxes. The plan requires the three display modes to provide the same useful result fields.

The revised `results-library-compact-mixed-default-restored.png` renders both descriptions while
keeping the title/description group distinct from identity metadata. The corrected Compact layout
is readable at 1280x800. **Resolved.**

### Resolved: narrow Pool row was not captured (medium evidence gap)

The first narrow result capture included two Question rows only, so it did not establish that the
denser Pool metadata layout avoided clipping or horizontal scroll.

`results-library-narrow-default-chip-pool.png` now shows the Question and Pool rows at 390px,
including Pool owner, member count, type, backend, license, discipline, tags, and its open link.
All visible content stays inside the viewport. **Resolved.** The active default chip remains
established by the full Library capture; it cannot share this scrolled result viewport.

### Sparse Pool-detail page composition (low)

At 1280x800 the Pool detail metadata column occupies roughly the left quarter of the page, leaving
substantial unused space before the PLE-managed support form. The reading order remains clear and
the content is not crowded, so this does not block M15. A future presentation pass could place
metadata in a wider definition-list grid when that page gains more durable content.

## Limitations

- No Blueprint-search image was supplied for visual inspection.
- The normal corpus scenes for Compact, List, and Visual boxes show their selected controls,
  rather than result rows; the dedicated M15 captures establish mixed-row presentation.
- Raster captures cannot establish keyboard behavior, focus visibility, semantic labels, or
  request behavior; those belong to the accessibility and browser-test reviews.

## Historical M15 visual-review call

**Pass for the reviewed M15 visual criteria.** All three Library display modes show the same
useful mixed-row information after the Compact correction; Questions and Pools remain distinct;
the active default chip is visible in the full Library capture; and the 390px result layout fits
both kinds. The Pool detail and both picker captures also pass their reviewed visual criteria.

At the time of this review, M16 corpus verification remained pending. Its subsequent receipts
and the laptop-only scope correction are recorded in
[SHARED_SEARCH_IMPLEMENTATION.md](SHARED_SEARCH_IMPLEMENTATION.md).
