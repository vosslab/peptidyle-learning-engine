# Top bar cleanup and browser typography

## Scope and evidence

The current screenshot corpus in `docs/screenshots/` shows the Product Role twice in each
authenticated top bar: once in the boxed plate and again as `Instructor account`, `Student account`,
or `System administrator`. The current session boundary (`src/api/contracts.ts`) has a Product Role,
but no account name that would make the second label useful.

The same screenshots show an unstyled, underlined `Profile` link. The requested behavior is a
rightmost Instructor-only Profile icon: use the established generic user silhouette until the
existing uploaded Profile thumbnail is available. The screen-reader name remains `Profile`.

This work is limited to the top menu bar, the existing Instructor Profile thumbnail placeholder and
the wiring needed to update it, plus the separately owned browser typography work package below.
It does not add navigation destinations, account-name data, a search surface, Student uploads, or
new Profile storage/API behavior.

## Work package A: one role plate and an accessible Profile avatar

### A1. Remove redundant account context

- Remove `accountLabel` from `RibbonContextLabels` and `RibbonContextModel` in
  `src/ribbon/ribbon_contract.ts`; remove its construction in `contextFor`.
- Remove `accountLabelFor()` and the resulting label entries from `src/app.tsx`.
- Remove the account-label markup in `src/ribbon/app_ribbon.tsx`, its CSS rule in
  `src/ribbon/app_ribbon.css`, and the now-unused `account` context glyph in
  `src/ribbon/ribbon_icons.ts`.
- Update the existing Ribbon models and contract/icon tests that construct or assert the deleted
  field. Keep role-specific availability and the Product Role plate unchanged.

**Outcome:** each authenticated bar presents Product Role exactly once, in the boxed plate.

### A2. Keep the pure Ribbon boundary and inject the Instructor avatar

- Keep `RibbonModel` synchronous and resource-free. `AppRibbon` accepts an optional
  `renderProfileAvatar` presentation slot; the application shell supplies it only for the existing
  Instructor Profile control.
- Add the API-aware component as
  `src/features/instructor_profile/ribbon_profile_avatar.tsx`, rather than under `src/ribbon/`.
  It receives the narrow Profile-thumbnail client capability and is injected by
  `src/application_shell.tsx`. The generic Ribbon continues to own layout, links, and semantics.
- Move the reusable `RibbonIcon` to `src/ribbon/ribbon_icon.tsx`, so the feature component and the
  Profile-page placeholder use the same local `circle-user` glyph without importing an API-aware
  feature into the Ribbon.
- In `AppRibbon`, render Sign out before the Profile link. The Profile link has
  `aria-label={control.label}` and `title={control.label}`, retains
  `data-ribbon-context-control="profile"`, and contains only the avatar/glyph; it has no visible
  `Profile` text. Sign out's auto margin keeps both account-end controls at the right edge.
- Preserve the existing Instructor-only `profile` context control. Student and Sysadmin bars have
  no Profile control and receive no new avatar behavior.

### A3. Use one reactive, cleanup-safe thumbnail projection

- Extract the current reference-to-Blob-to-object-URL logic from
  `src/features/instructor_profile/profile_thumbnail.tsx` into one small feature-level helper. It
  accepts a *reactive* thumbnail-reference accessor and the narrow thumbnail-delivery client,
  returns the current primitive URL accessor, and owns object-URL revocation whenever the
  reference changes, delivery fails, or the consumer unmounts.
- Reuse that helper in `ProfileThumbnail` and `RibbonProfileAvatar`. A missing reference, loading
  delivery, failed delivery, or unavailable thumbnail renders the generic `circle-user` glyph.
- After `replaceInstructorProfileThumbnail()` succeeds in
  `src/pages/instructor_profile_page.tsx`, dispatch a narrowly typed
  `ple-profile-thumbnail-replaced` event containing the saved thumbnail metadata. The ribbon
  avatar updates its resource from the event and therefore fetches the replacement without a page
  reload. Register the listener inside the avatar component and remove it with `onCleanup`.
- Keep the Profile-page container's `aria-label="Profile image"`; replace only its visible
  `Profile image` fallback words with the decorative shared glyph. Continue to expose the existing
  unavailable-status message when delivery fails.

### A4. Style the actual control, including touch and forced-color modes

- Add a dedicated `.ple-app-ribbon__profile` rule beside the existing end controls in
  `src/ribbon/app_ribbon.css`. Use the established rounded-square silhouette
  (`var(--ple-radius-inset)`), a subtle Ribbon-rule boundary, hidden image overflow,
  `object-fit: cover`, and the local generic glyph. Do not introduce a circular avatar treatment.
- Preserve dense desktop sizing with a 2rem visual control. At coarse pointers, make the actual
  hit target at least 2.75rem in both dimensions while preserving the existing responsive Ribbon
  topology.
- Add the Profile selector to the established hover, active, focus-visible, and forced-colors
  selector groups. The link must not inherit the global underlined anchor presentation.

### A5. Reconcile durable interaction documentation

- In `docs/HUMAN_GUIDANCE.md`, add only the owner's direct guidance or close paraphrase: Product
  Role is shown once in the role plate; Instructor Profile is a rightmost icon-only generic user
  image until an uploaded Profile image replaces it. Do not add a search-bar statement.
- Update `docs/DESIGN_DECISIONS.md`, `docs/UI_DESIGN_GUIDE.md`, and
  `docs/ux/RIBBON_TASK_MODEL.md`. In the task model, replace the Instructor and Sysadmin
  `Account label` information need with the single Product Role plate and identify the Instructor
  Profile control as an accessible icon-only end control.
- Add concise behavior/interface and verification entries under the current date in
  `docs/CHANGELOG.md`.

## Work package B: browser-wide Atkinson Hyperlegible Next

This package is independent of Work package A's Ribbon contract and can proceed in parallel after
the current browser build is known healthy.

### B1. Bundle the official browser font locally

- Obtain the official Atkinson Hyperlegible Next web-font distribution from its publisher, retain
  only the weights/styles the browser application actually uses, and store the files plus the
  applicable license/provenance notice in a clear browser-asset location under `src/`.
- Define explicit local `@font-face` rules in `src/styles/browser_fonts.css` with correct family,
  style, weight range, `font-display`, and local asset URLs. Set the browser application's root
  sans-serif stack in `src/style.css` to Atkinson Hyperlegible Next before the existing system
  fallbacks.
- Keep explicit `ui-monospace`/monospace rules for code, machine identifiers, and math-like
  alignment. Preserve backend/export/native-renderer font choices: this package changes browser
  presentation only.
- Record the source URL, version or release identity, license, retained files, and build-copy owner
  in the appropriate durable design/build documentation. The application must not depend on a
  remote font host at runtime.

### B2. Prove local delivery and rendered use without creating a fragile visual gate

- Extend the existing build-asset evidence or add one narrowly scoped build check to prove that the
  declared font files are copied to `dist/`, CSS refers only to local font assets, and built browser
  artifacts contain no remote font request. Its failure means the application would lose its
  selected readable typeface or violate the local-delivery decision; correct the asset/build rule.
- Use a one-time browser check in `tests/_temp/` (or an equivalent untracked operator check) to
  confirm a normal browser text element resolves to Atkinson Hyperlegible Next, an intentional
  monospace element remains monospace, and no font delivery request leaves the local stack. Remove
  this proof after implementation unless it earns durable contract coverage.
- Capture a fresh screenshot corpus and review typography at ordinary laptop and phone sizes. This
  is rendered evidence, not a pixel-equivalence requirement.

## Verification and acceptance

1. Run TypeScript and lint checks required by the changed source, then the focused existing Ribbon
   contract/icon tests. Retain only permanent tests that protect the stable public contract: one
   Product Role plate and the accessible Profile link name/order. Use
   `getByRole("link", { name: "Profile", exact: true })` in browser evidence; avoid a data-attribute
   selector when the accessible link can express the user behavior.
2. Run the existing build-copy proof for locally bundled fonts and its focused failure recovery.
   Run the one-time thumbnail upload/update/persistence proof: upload on `/profile`, observe the
   bar avatar update without reload, navigate away and back, and observe the saved image. Keep the
   check outside the permanent suite unless review finds a distinct stable regression it protects.
3. Run `./devel/capture_screenshots.sh`, then
   `./devel/capture_screenshots.sh --verify`. The latter proves semantic replay, current routes,
   privacy, and corpus closure; it does not require matching PNG bytes.
4. Have an independent reviewer inspect the authenticated Instructor, Student, Sysadmin, Profile,
   laptop, and phone captures. Accept when role appears only in its plate, the Instructor's right
   end has Sign out then a rounded-square generic/avatar Profile control, other roles have no
   Profile control, top-bar text remains dense and readable, and ordinary browser text uses the
   bundled Atkinson Hyperlegible Next family.
5. Run `git diff --check` and reread this plan against the actual changed source. Correct failures
   at their owning boundary: Ribbon model/presentation, Profile feature delivery, local font build
   wiring, or evidence workflow.

## 2026-09-11 implementation receipt

Focused TypeScript, lint, 22 Ribbon tests, and build passed. One connected Instructor proof and
the 49-path screenshot publication/semantic-privacy replay passed; five byte differences remain
human-review evidence, and independent visual acceptance passed.

The simultaneous Course Instance short/long-name contract belongs to the completed Interface Cleanup
M2 Ribbon work in [interface_cleanup_2026_09.md](../archive/interface_cleanup_2026_09.md), not this focused cleanup.
