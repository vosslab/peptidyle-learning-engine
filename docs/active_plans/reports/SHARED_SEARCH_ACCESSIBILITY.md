# Shared search accessibility evidence

Source plan: [read-docs-active-plans-active-shared-sea-peaceful-mountain.md](../read-docs-active-plans-active-shared-sea-peaceful-mountain.md).

## M15 scope and result

The one-time M15 accessibility check passed on October 5, 2026. It mounted compiled
current source in an authenticated Instructor fixture and found zero serious or critical Axe
violations in these scopes:

- mixed, paged Question Library at 1280x800;
- Pool detail with Star Pool and Watch Pool controls;
- active Question Picker dialog;
- active Assessment content picker dialog;
- existing Library idle, loaded, and cleared states; and
- Public Blueprint Course search idle, loaded, and cleared states at 390x844.

The dialog scans target the open native dialog explicitly, including its controls and result rows,
instead of scanning the obscured page behind it. The fixture's authenticated `grass` appearance
covers Instructor controls. Desktop and narrow layouts are covered for Library and Blueprint; this
is not a complete personal-theme matrix.

The M16 Live Demo capture additionally exposed an Assessment dialog color-token defect that the
fixture did not expose. After the dialog adopted current surface tokens and a modal backdrop,
the complete browser launcher passed again, including all four accessibility cases
(`/tmp/ple_shared_search_m16_dialog_ui.log`). Independent review of the refreshed live desktop
and square captures confirmed an opaque, readable surface. Fixture scans remain separate from
that production-rendered evidence.

The shared RecordList Compact description repair was replayed with this focused acceptance before
the final fast-UI registration and captures below.

## Keyboard behavior

The one-time check used Tab to reach text fields, filters, result links, native controls, display
buttons, paging controls, Clear all, and the leave-dialog action. It verified Enter text submit,
Space selection and promoted filtering, Compact/List/Visual boxes activation, real Library and
Question Picker Next/Previous paging, no horizontal overflow for the narrow Library and Blueprint
views, and focus returning to the search field after Clear all. The Question Picker test preserves
its selected tray through Clear all, then loads the cleared query before paging; that is its
documented picker contract.

## Rendered evidence

The focused test captured these current-source images under `/tmp/ple_shared_search_m15/`:

- `library-compact-mixed-default.png`, `library-list-mixed-default.png`, and
  `library-visual-mixed-default.png` show the default-chip mixed Library modes;
- `results-library-list-mixed-default.png`,
  `results-library-visual-mixed-default.png`, and
  `results-library-compact-mixed-default-restored.png` show the loaded rows in those modes;
- `results-library-narrow-mixed.png` shows loaded rows at 390px;
- `results-library-narrow-default-chip-pool.png` is the narrow default-chip result scene with its
  Pool row;
- `pool-detail.png`, `question-picker.png`, and `assessment-content-picker.png` show the
  remaining M15 consumers.

## Disposition after permanent-test review

The plan-specific accessibility test and its separate harness were removed on October 5 at the
user's request. The six-item permanent-test checklist did not justify duplicating search behavior
across all consumers and maintaining a second application fixture. The results above remain
historical implementation evidence, not a current blocking gate. Future accessibility reviews
use temporary checks against the relevant current surface; a demonstrated regression can earn a
focused permanent case.
