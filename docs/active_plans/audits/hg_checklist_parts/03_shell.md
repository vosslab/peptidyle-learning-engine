## Interface design
### General interface design

- [ ] Design around what users need to find and do.
  - Mismatch: No repository-wide behavioral or usability evidence establishes this broad design outcome.
- [ ] Important information should stand out from supporting information.
  - Mismatch: Local visual hierarchy exists, but no system-wide implementation evidence verifies this outcome.
- [ ] Related information should be visually grouped and aligned.
  - Mismatch: No repository-wide visual audit verifies this across PLE pages.
- [ ] Similar pages should place similar controls in consistent locations.
  - Mismatch: No cross-page implementation evidence verifies the whole-product requirement.
- [ ] Use headings and action labels that reflect the current state and next useful step.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Match feedback wording and visual emphasis to the outcome: success, information, warning, or
  error. Make the result and any next action easy to recognize.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Primary actions should be easy to find and appear near the content or workflow they affect.
  - Mismatch: No whole-product browser or usability evidence verifies this broad requirement.
- [ ] Identify the object and relevant context before an action that changes membership or stored
  settings, so users can recognize what they are accepting or changing.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Use concise helper text near the control it explains. Present shared explanations once per
  relevant group and keep the main task information easy to scan.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Avoid scattering related actions across page headers, menus, navigation, and content areas.
  - Mismatch: Current top-bar Sign Out contradicts the specified Profile-menu location.
- [ ] Dream big on the UI. Choose one visual philosophy and carry it through the entire interface.
  - Mismatch: No repository evidence can verify this whole-product qualitative outcome.
- [ ] Students should have no upload capabilities. Instructor-created content should use text boxes.
  - Mismatch: Student upload denial is not sufficient to verify the universal Instructor text-box requirement.

### Information density and layout

- [x] Design Instructor and **Sysadmin** workflows for laptop browsers, using a 1280 by 800 viewport
  as the layout target.
  - Evidence (source): `tests/playwright/ui_corpus_manifest.ts` `RIBBON_RESPONSIVE_PROFILES` and `SYSADMIN_DESKTOP_CONTEXT_OPTIONS` declare 1280 by 800 desktop contexts for both staff roles.
  - Evidence (test): `tests/playwright/ribbon_m9_responsive_evidence.mjs` `assertResponsiveRows` verifies the Instructor desktop shell and `assertSysadminDesktopRibbon` verifies the Sysadmin Ribbon has no overflow with Instructor Accounts and Scoped Support visible.
  - Evidence (source): `src/pages/role_home_pages.tsx` `SysadminHomePage` presents the backed Instructor Accounts and Scoped Support operations reached by the checked Sysadmin desktop model.
  - Decision: retained viewport-target evidence supports this equivalent design-target rewrite, not whole-product usability or all staff workflows.
- [x] PLE often presents large collections where users need to find a few relevant items.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` renders the Question Library collection surface.
- [ ] Optimize large collections for scanning, searching, filtering, and comparison.
  - Mismatch: The broad all-collection outcome lacks complete implementation evidence.
- [ ] Show enough useful information at once to support comparison without excessive scrolling.
  - Mismatch: No viewport-based collection comparison evidence verifies this requirement.
- [ ] Search and filters should help users quickly narrow large collections.
  - Mismatch: Existing controls do not establish the stated quick-narrowing outcome across all large collections.
- [ ] Dense pages should remain easy to scan.
  - Mismatch: No usability or whole-product visual evidence establishes scanability.
- [ ] Treat screen space as a limited resource. Prefer useful information over decorative whitespace.
  - Verification pending: Current source includes compact Course rows in `src/pages/course_list_page.tsx` and grid/panel layout in `src/pages/assessment_templates_page.css`; this new or expanded requirement lacks a scoped rendered audit across the affected pages at 1280 x 800. Existing local layouts do not establish the whole requirement.
- [ ] Use spacing to separate meaningful groups rather than simply making pages spacious. Large gaps should communicate a meaningful change in section or task.
  - Verification pending: Current source includes compact Course rows in `src/pages/course_list_page.tsx` and grid/panel layout in `src/pages/assessment_templates_page.css`; this new or expanded requirement lacks a scoped rendered audit across the affected pages at 1280 x 800. Existing local layouts do not establish the whole requirement.
- [ ] Prefer alignment, typography, and dividers over unnecessary cards, boxes, borders, and nested containers.
  - Mismatch: Current pages include cards and borders; no audit establishes the preference is followed.
- [ ] Cards and rounded containers should earn their space by representing a distinct object or interaction, not merely grouping nearby content.
  - Verification pending: Current source includes compact Course rows in `src/pages/course_list_page.tsx` and grid/panel layout in `src/pages/assessment_templates_page.css`; this new or expanded requirement lacks a scoped rendered audit across the affected pages at 1280 x 800. Existing local layouts do not establish the whole requirement.
- [ ] Avoid the modern dashboard style of large rounded cards, generous padding, and isolated islands of content.
  - Verification pending: Current source includes compact Course rows in `src/pages/course_list_page.tsx` and grid/panel layout in `src/pages/assessment_templates_page.css`; this new or expanded requirement lacks a scoped rendered audit across the affected pages at 1280 x 800. Existing local layouts do not establish the whole requirement.
- [ ] Use horizontal and vertical space efficiently without crowding information together. Related information should form clearly readable rows, columns, or groups.
  - Verification pending: Current source includes compact Course rows in `src/pages/course_list_page.tsx` and grid/panel layout in `src/pages/assessment_templates_page.css`; this new or expanded requirement lacks a scoped rendered audit across the affected pages at 1280 x 800. Existing local layouts do not establish the whole requirement.
- [ ] Size controls and content regions for their contents and task. Avoid unnecessarily tall panels, empty states, Question previews, and other fixed-height regions.
  - Verification pending: Current source includes compact Course rows in `src/pages/course_list_page.tsx` and grid/panel layout in `src/pages/assessment_templates_page.css`; this new or expanded requirement lacks a scoped rendered audit across the affected pages at 1280 x 800. Existing local layouts do not establish the whole requirement.
- [ ] Keep the visual design compact, flat, information dense, and consistent across PLE.
  - Mismatch: No complete rendered-product audit verifies all four whole-product attributes.
- [ ] Use compact rows, restrained corner rounding, and controls sized to their task.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Present short labels and values in aligned rows or compact grids, adapting to stacked groups
  when the available width requires them.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Give each object one clear title within its list entry. Group its metadata and actions beneath
  or alongside that title.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Preserve readable text and reachable controls as users enlarge text or zoom the page.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.

### Interaction design

- [ ] Use progressive disclosure to keep common tasks compact while making supporting details easy
  to find.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Use tooltips for brief supplementary explanations, available on hover and keyboard focus.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Use clearly labeled expandable sections with chevrons for longer details and secondary settings,
  supporting keyboard, pointer, and touch interaction.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Keep essential information, primary actions, and current status visible in the main interface.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Use drag-and-drop where it makes reordering faster and more natural.
  - Mismatch: No implemented drag-and-drop reordering surface was found in the audited shell evidence.
- [x] Reordering must also have a precise keyboard-accessible method.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `moveEntry` and `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `move` back their labelled native-button Move earlier and Move later controls.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_choice_list.tsx` `onMoveChoice`, `src/features/ple_question_json_authoring/question_json_multiple_answer_editor.tsx` `onMoveChoice`, `src/features/ple_question_json_authoring/question_json_multi_fill_in_editor.tsx` `onMoveBlank`, `src/features/ple_question_json_authoring/question_json_matching_editor.tsx` `onMoveItem`, and `src/features/ple_question_json_authoring/question_json_ordering_editor.tsx` `onMoveItem` give every native JSON reorderer the same precise buttons.
  - Evidence (test): `tests/test_blueprint_course_model.mjs` `reusable entries preserve fixed and Question Pool interleaving`, `tests/test_ple_question_json_editor_model.mjs` `choice edits retain semantic IDs and enforce choices and correct-answer invariants`, `tests/test_ple_question_json_multiple_answer_editor.mjs` `multiple-answer text edits and reordering retain choice IDs and exact correct IDs`, and `tests/test_ple_question_json_multi_fill_ordering_authoring.mjs` `ORDER treats Ordering Items as the source of truth and derives correctOrder after movement` protect the stable reorder results.
  - Decision: The one-time seven-surface keyboard-control inventory passed and was removed rather than becoming a permanent implementation-inventory test. It does not select drag-and-drop surfaces, which remains the separate Human Guidance product question.
- [ ] UUIDs should never appear in visible content, navigation URLs, or copyable links.
  - Mismatch: `tests/test_public_navigation.mjs` `human route references are compact, typed, and bounded` checks route references only; it does not establish the absence of UUIDs from visible content or copyable links.

### Role colors and themes

- [x] **Sysadmin** uses tomato red as its role color.
  - Evidence (source): `src/styles/product_role.css` `[data-product-role="sysadmin"]` defines `--ple-role-accent: #ff6347`.
- [x] **Instructor** uses teal green as its role color.
  - Evidence (source): `src/styles/product_role.css` `[data-product-role="instructor"]` defines `--ple-role-accent: #168575`.
- [x] **Student** uses lavender purple as its role color.
  - Evidence (source): `src/styles/product_role.css` `[data-product-role="student"]` defines `--ple-role-accent: #8861b5`.
- [x] Role colors should be used consistently in role labels and other appropriate interface cues.
  - Evidence (source): `src/styles/product_role.css` `.live-demo-persona-action[data-product-role]` and `.ple-app-ribbon__product-role[data-product-role]` consume the shared role tokens.
- [x] Demo role selection should clearly state both the user's role and name.
  - Evidence (test): `tests/playwright/e2e_live_demo_authoring_browser.mjs` selects `Assume the role of Instructor Dr. Elena Rivera`.
- [ ] Courses use a fixed set of visually distinct biome and habitat themes.
  - Verification pending: `src/features/course_appearance/course_theme_registry.ts` `COURSE_THEME_REGISTRY` provides the existing 15-theme registry, not acceptance of the expanded palette specification. Current rendered distinctness and actual-use accessibility, coordinated light/dark behavior, and the specification cutover require separate proof.
- [ ] Course Themes should have coordinated light and dark appearances.
  - Mismatch: `src/features/course_appearance/course_theme_registry.ts` `theme` derives one appearance from three anchors using white surfaces; there is no coordinated light/dark mode registry or selector.
- [ ] Course Theme colors should remain accessible in their actual interface uses.
  - Verification pending: `src/features/course_appearance/course_theme_registry.ts` `COURSE_THEME_REGISTRY` provides the existing 15-theme registry, not acceptance of the expanded palette specification. Current rendered distinctness and actual-use accessibility, coordinated light/dark behavior, and the specification cutover require separate proof.
- [ ] Light themes should use clearly light page backgrounds; dark themes should use clearly dark page
  backgrounds. Use theme colors as accents on surfaces with readable contrast.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Check text, controls, borders, and interaction states against their actual rendered backgrounds.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Apply the contrast requirements for text, controls, and other semantic uses in
  [BIOME_THEME_PALETTES.md](/docs/BIOME_THEME_PALETTES.md)
  to rendered components in both light and dark themes, including gradients and state backgrounds.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Pair color cues with text, icons, or shapes so selection, focus, saved status, and results remain
  recognizable across themes and color-vision differences.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [x] Course Theme IDs are durable; changing a theme's display name or colors should not require a new ID.
  - Evidence (source): `crates/question_model/src/course_appearance.rs` `CourseTheme` and `as_str` own durable serialized IDs; `src/features/course_appearance/course_theme_registry.ts` `COURSE_THEME_REGISTRY` keys display names and colors separately by those IDs, including stored `grass` displayed as Grassland. Name/palette changes do not change the identity field.
- [ ] Follow `docs/BIOME_THEME_PALETTES.md` for Course Theme names, palettes, accessibility, and implementation.
  - Mismatch: `crates/question_model/src/course_appearance.rs` `CourseTheme` has 15 persisted themes; `src/features/course_appearance/course_theme_registry.ts` `theme` retains a three-anchor single appearance, rather than the proposed 25-theme coordinated light/dark fixed-palette specification.

### Typography

- [x] Use [Atkinson Hyperlegible Next](https://www.brailleinstitute.org/freefont/) as the main PLE font.
  - Evidence (source): `src/style.css` `:root` sets `Atkinson Hyperlegible Next` as the first font family.
- [x] Use [Atkinson Hyperlegible Mono](https://www.brailleinstitute.org/freefont/) for code and other monospace text.
  - Evidence (source): `src/styles/browser_fonts.css` `--ple-font-mono` applies the local `Atkinson Hyperlegible Mono` family to `code`, `kbd`, `pre`, and `samp` through normal and italic `@font-face` declarations.
  - Evidence (source): `pipeline/build.mjs` `BROWSER_FONT_BUNDLES` copies and verifies the Mono assets in the production `dist` output.
  - Decision: One-time normal-and-italic computed-style proof passed and was removed; the behavior does not retain a permanent implementation-coupled test.
- [x] Prefer the official Braille Institute font files and include the needed weights locally with PLE.
  - Evidence (source): `src/styles/browser_fonts.css` `@font-face` loads local Atkinson Hyperlegible Next variable font files.
- [x] When a narrow font is needed, use `IBM Plex Sans Condensed` for long unbreakable strings such as URLs.
  - Evidence (source): `src/styles/browser_fonts.css` `--ple-font-narrow` declares the local `IBM Plex Sans Condensed` face and applies it only to the PLE Question JSON editor's Citation URL input; `pipeline/build.mjs` `BROWSER_FONT_BUNDLES` copies and verifies that same-origin asset.
  - Evidence (runtime): `src/features/ple_question_json_authoring/question_json_editor_styles.ts` `PLE_QUESTION_JSON_EDITOR_STYLES`: a one-time Chromium fixture imported the actual injected editor CSS against production-built local font assets on 2026-09-15; at 360 and 1280 CSS pixels in light and dark OS preferences it requested the local font, computed the narrow family on the Citation URL input, and retained normal input value, horizontal-scroll, and overflow behavior.
- [x] With `IBM Plex Sans Condensed`, try `font-variant-numeric: slashed-zero` to better distinguish `0` from `O`.
  - Evidence (source): `src/styles/browser_fonts.css` `font-variant-numeric: slashed-zero` applies it to that narrow Citation URL input; `src/assets/fonts/ibm_plex_sans_condensed/provenance.txt` records the locally retained IBM Plex Sans Condensed Regular asset, OFL provenance, and its verified OpenType `zero` GSUB feature.
  - Evidence (runtime): `src/styles/browser_fonts.css` `font-variant-numeric: slashed-zero`: that same one-time Chromium fixture computed `slashed-zero` on the local IBM face without a fallback; it was removed rather than retained as a permanent implementation-coupled test.
- [ ] Question Backend-rendered content may use its own fonts when needed for correct display.
  - Mismatch: No Question Backend font-isolation implementation evidence was found in the shell audit.

### Ribbon and page layout

- [x] The top Ribbon is the persistent navigation area for signed-in PLE pages.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` renders `AppRibbon` inside the persistent shell.
- [x] The Ribbon should remain in the same location and use the same overall structure while navigating.
  - Evidence (test): `tests/playwright/ribbon_m9_responsive_evidence.mjs` `assertResponsiveRows` verifies declared Ribbon rows across route-model changes.
- [x] Navigation choices should remain in predictable locations as users move between related pages.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` and `RIBBON_TASK_CATALOG` own fixed navigation identities.
- [x] Changing a Ribbon selection changes the content below the Ribbon without moving the main content area up or down.
  - Evidence (test): `tests/playwright/ribbon_m9_responsive_evidence.mjs` `assertResponsiveRows` and the `application data does not move the content origin` assertion verify the content origin.
- [x] Ribbon rows should keep their space when needed so changing selections does not make the content area jump.
  - Evidence (test): `tests/playwright/ribbon_geometry_evidence.mjs` `chromeAboveContent` verifies reserved row tokens and shell track geometry.
- [x] Page actions should appear near the content they affect rather than changing the Ribbon layout.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` renders only catalog navigation and Sign Out in `AppRibbon`; task content stays in `ApplicationShell` content.
- [ ] On narrow Student screens, use a compact navigation arrangement that keeps the product identity,
  current location, navigation controls, and Profile readable and reachable.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `AppRibbon` retains the Student identity and Profile controls while `src/ribbon/app_ribbon.css` `@media (max-width: 40rem)` makes the Student identity content-sized and gives the navigation controls a full-width scrollport; `src/application_shell.tsx` `BreadcrumbPrelude` supplies the route-context trail where reserved.
  - Evidence (runtime): `src/ribbon/app_ribbon.tsx` `AppRibbon` and `src/application_shell.tsx` `BreadcrumbPrelude` were exercised in root's accepted bounded actual before/after receipt at `/private/tmp/ple-student-nav-proof/after-report.json`, with current `after-*.png` captures at 1280, 390, and 320 CSS pixels and 320 at 200% root font; the sampled overview, history, and Profile routes retained readable identity, reachable Profile and navigation, visible current route context where present, and no document overflow; an independent review accepted the bounded fix. See `/private/tmp/ple-student-nav-implementation.md`.
  - Verification pending: This bounded Student receipt does not establish every narrow Student route or the broader all-routes layout contract. Profile still has no breadcrumb reservation.
- [x] See **User top bar** and **Breadcrumbs** for the persistent elements that make up the top of the page.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` composes `AppRibbon` and `BreadcrumbPrelude`.

### User top bar interface

- [x] All signed-in users share the same top-left logo/account and top-right profile bar layout.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `AppRibbon` renders the shared leading `ple-app-ribbon__context-identity` logo/account block and shared trailing `ple-app-ribbon__profile-endcap` Profile control for every role model; `src/ribbon/app_ribbon.css` `ple-app-ribbon__profile-endcap` pins the Profile control at the inline end, while Student narrow rules adapt only the middle navigation arrangement.
- [x] The top bar remains in a consistent location as users navigate.
  - Evidence (test): `tests/playwright/ribbon_m9_responsive_evidence.mjs` `assertResponsiveRows` measures the persistent top row across model changes.
- [x] The PLE logo and product name appear at the upper left and link to the user's home dashboard.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__brand` is the leading `href="/"` Peptidyle home link.
- [x] Each Product Role has its own home dashboard and navigation.
  - Evidence (source): `src/route_contract.ts` `productRoleHomeRouteId` declares one role-scoped home route for Instructor, Student, and Sysadmin; `src/ribbon/ribbon_contract.ts` `hrefFor` directs each Product Role's selected Courses navigation to that route.
  - Evidence (test): `tests/test_ribbon_route_contract.mjs` `each Product Role has an explicit selected Courses home route` verifies the role-scoped route, selected Courses navigation, and matching link for every Product Role.
  - Generated evidence stale: `docs/screenshots/current_capture_manifest.json` marks the three new role-home routes `deferred`; it is not visual proof until a fresh Live Demo capture.
- [x] Product Role appears once next to the PLE name.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__product-role` occurs once in the shared identity block beside `ple-app-ribbon__brand`.
- [x] Role-specific navigation appears between the product identity and Profile.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` places `ple-app-ribbon__tabs` after identity and before account controls.
- [x] Profile appears at the far right as an icon-only avatar.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__profile-endcap` renders the shared icon-only Profile button after the navigation region; `src/ribbon/app_ribbon.css` `ple-app-ribbon__profile-endcap` anchors that endcap at the inline end.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `every signed-in Product Role has one accessible generic Profile end control` verifies the one accessible, text-free Profile control for Student, Instructor, and Sysadmin.
  - Decision: A one-time real-shell probe verified the isolated Profile control at 1280 and 320 CSS pixels with a coarse pointer, including thumbnail-request isolation; it was removed rather than retained as a permanent browser test.
- [x] Clicking the Profile avatar opens the Profile menu.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `openProfileMenu` controls the Profile trigger's `profileMenuOpen` state and renders the labelled `ple-profile-menu` menu.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Ribbon Profile menu contract: PASS` protects pointer and keyboard opening, focus, dismissal, and action dispatch.
- [ ] The Profile menu contains Profile settings, account settings, and Sign Out.
  - Mismatch: No Profile menu is implemented.
- [x] Sign Out belongs in the Profile menu rather than the main top bar.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `data-ribbon-action={props.model.context.signOutAction.id}` renders Sign Out as a Profile-menu item and closes that menu after dispatch.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Ribbon Profile menu contract: PASS` verifies no top-bar Sign Out button and one dispatched Profile-menu Sign Out action.
- [x] See **Ribbon and page layout** for the overall navigation and page-position rules.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` is the shared shell that composes the top bar and content region.

### Profile avatar interface

- [ ] Every Account is randomly assigned an avatar from the PLE avatar gallery when the Account is created.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Profile avatar interface boundary.
- [ ] The same avatar gallery collection is available to all Product Roles.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Profile avatar interface boundary.
- [ ] The current avatar or Profile image appears consistently anywhere PLE represents that user.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Profile avatar interface boundary.

#### Student avatars

- [ ] **Students** select avatars from the PLE-provided avatar gallery collection and cannot upload Profile images.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Student avatars boundary.
- [ ] Student avatar selection should be visual and playful.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Student avatars boundary.
- [ ] All avatars in the gallery are available for selection.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Student avatars boundary.
- [ ] Students may select another avatar at any time.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Student avatars boundary.

#### Instructor and Sysadmin Profile images

- [ ] **Instructors** and **Sysadmins** share the same Profile backend and functionality.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Instructor and Sysadmin Profile images boundary.
- [ ] **Instructors** and **Sysadmins** may select from the PLE avatar gallery or upload their own Profile image.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Instructor and Sysadmin Profile images boundary.
- [ ] Image upload accepts any aspect ratio with a minimum of 128 pixels in both dimensions.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Instructor and Sysadmin Profile images boundary.
- [ ] After upload, Instructors and Sysadmins can position and crop the image within a square Profile preview.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Instructor and Sysadmin Profile images boundary.
- [ ] Instructors and Sysadmins may replace their Profile image or select a provided avatar at any time.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Instructor and Sysadmin Profile images boundary.
- [ ] The current avatar or Profile image appears consistently anywhere PLE represents that user.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Profile avatar interface boundary.
  - Owner: 03_shell.md / Profile avatar interface (first occurrence; identical requirement and status).

### Breadcrumbs interface

- [ ] All signed-in users have a permanent breadcrumb row below the top Ribbon.
  - Mismatch: `src/application_shell.tsx` `BreadcrumbPrelude` renders only when `breadcrumbPreludeReserved` is true.
- [ ] The breadcrumb row remains in the same location and keeps the same space as users navigate.
  - Mismatch: Breadcrumb space is conditional on `breadcrumbPreludeReserved`, not permanent for every signed-in route.
- [ ] Breadcrumbs show the path from the user's home dashboard to the current page.
  - Mismatch: No audit evidence proves every route's breadcrumb path begins at the role home dashboard.
- [ ] Each breadcrumb level links back to its corresponding page.
  - Mismatch: `src/application_shell.tsx` allows a breadcrumb without `href` to render as a span.
- [x] Breadcrumbs use human-readable names rather than internal identifiers.
  - Evidence (source): `src/application_shell.tsx` `BreadcrumbPrelude` renders `RibbonBreadcrumbModel.label`, not an ID.
- [x] Course and Assessment breadcrumbs preserve the current Course context.
  - Evidence (source): `src/ribbon/route_scope_controller.ts` `createRouteScopeController` resolves route labels while retaining course scope.
- [x] Keeping the breadcrumb row in place prevents the main content from moving up or down as breadcrumb depth changes.
  - Evidence (test): `tests/playwright/ribbon_m10_shell_evidence.mjs` `label resolution preserves the reserved breadcrumb-prelude geometry` verifies stable shell geometry through deferred resolution.
- [x] See **Ribbon and page layout** for the overall page-position rules.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-shell__breadcrumb-prelude` documents and implements the shell-owned stable prelude.
