// ribbon_shell_evidence.mjs - compiled-harness application-shell structural evidence.
// This exercises current source composition in a controlled browser fixture; it
// does not build dist/ or replace real-stack browser acceptance.

import assert from "node:assert/strict";

import { assertDenseTopBarKeyboardTraversal } from "./ribbon_dense_top_bar_keyboard.mjs";
import { assertNarrowRoutedShell } from "./ribbon_narrow_routed_shell.mjs";
import {
  assertBreadcrumbPreludeStyle,
  assertOneStableRibbon,
  caseLocator,
  flush,
  openRibbonShellEvidencePage,
  waitForPath,
} from "./ribbon_shell_helpers.mjs";

const { browser, page, harnessServer, pageErrors, consoleErrors } =
  await openRibbonShellEvidencePage();
try {
  // Case A: current-source App composition with its router, routes, and providers.
  await page.evaluate(() => window.ribbonShell.currentNavigate("/"));
  await waitForPath(page, "current-production", "/");
  const currentCase = caseLocator(page, "current-production");
  await currentCase.locator(".site-header").waitFor({ state: "visible" });
  assert.equal(
    await currentCase.locator(".ple-app-ribbon").count(),
    0,
    "the synthetic loading surface uses the no-Ribbon shell shape",
  );
  assert.equal(
    await currentCase.locator(".site-header").count(),
    1,
    "the synthetic loading surface retains its identity header",
  );
  assert.equal(
    await currentCase.locator('.sr-only[role="status"][aria-live="polite"]').count(),
    1,
    "the synthetic loading surface retains the sign-out live region",
  );
  await page.evaluate(() => window.ribbonShell.releaseSession());
  await currentCase.locator(".ple-app-ribbon").waitFor({ state: "visible" });
  assert.equal(
    await currentCase.locator(".ple-shell-frame.ple-ribbon-shell-grid").count(),
    1,
    "an authenticated route uses one Ribbon shell frame",
  );
  assert.equal(
    await currentCase.locator(".site-header").count(),
    0,
    "an authenticated Ribbon route has no duplicate identity header",
  );
  assert.equal(
    await currentCase.locator('.sr-only[role="status"][aria-live="polite"]').count(),
    1,
    "the sign-out live region remains mounted on an authenticated Ribbon route",
  );
  assert.equal(
    await currentCase.locator('.ple-shell-frame .sr-only[role="status"]').count(),
    0,
    "the sign-out live region remains outside the geometry-owning shell frame",
  );
  const authenticatedFrame = await currentCase.locator(".ple-shell-frame").evaluate((frame) => ({
    clientHeight: frame.clientHeight,
    scrollHeight: frame.scrollHeight,
  }));
  assert.ok(
    authenticatedFrame.scrollHeight <= authenticatedFrame.clientHeight,
    "the resolved authenticated shell has stable short-content geometry without internal overflow",
  );
  let currentRibbon = await currentCase.locator(".ple-app-ribbon").elementHandle();
  assert.notEqual(currentRibbon, null, "current App mounts one Ribbon on an authenticated route");
  await assertOneStableRibbon(page, "current-production", currentRibbon);
  assert.deepEqual(
    await currentCase
      .locator("[data-ribbon-control]")
      .evaluateAll((controls) =>
        controls.map((control) => control.getAttribute("data-ribbon-control")),
      ),
    [
      "courses",
      "questions",
      "productAssessments",
      "myBlueprintCourses",
      "myActiveCourses",
      "myInactiveCourses",
      "searchPublicBlueprintCourses",
    ],
    "the current Instructor Product route retains every required Courses choice",
  );
  assert.deepEqual(
    await currentCase
      .locator('.ple-app-ribbon__task-area[data-ribbon-task-area="instructorCourses"]')
      .locator("[data-ribbon-control]")
      .evaluateAll((controls) =>
        controls.map((control) => ({
          id: control.getAttribute("data-ribbon-control"),
          availability: control.getAttribute("data-ribbon-availability"),
          ariaDisabled: control.getAttribute("aria-disabled"),
          href: control.getAttribute("href"),
        })),
      ),
    [
      {
        id: "myBlueprintCourses",
        availability: null,
        ariaDisabled: null,
        href: "/blueprint-courses",
      },
      {
        id: "myActiveCourses",
        availability: null,
        ariaDisabled: null,
        href: "/instructor",
      },
      {
        id: "myInactiveCourses",
        availability: null,
        ariaDisabled: null,
        href: "/instructor/courses/inactive",
      },
      {
        id: "searchPublicBlueprintCourses",
        availability: null,
        ariaDisabled: null,
        href: "/blueprint-courses/search/public",
      },
    ],
    "implemented Course choices remain links while future Instructor choices stay visible and unusable",
  );
  assert.deepEqual(
    await currentCase
      .locator(".ple-app-ribbon__top-bar [data-ribbon-control]")
      .evaluateAll((controls) =>
        controls.map((control) => control.getAttribute("data-ribbon-control")),
      ),
    ["courses", "questions", "productAssessments"],
    "only backed Product Tabs participate in the dense top-bar keyboard order",
  );
  await page.evaluate(() => {
    document.body.tabIndex = -1;
    document.body.focus();
    document.body.removeAttribute("tabindex");
  });
  const productKeyboardOrder = [
    [currentCase.getByRole("link", { name: "Skip to learning content" }), "the skip link"],
    [currentCase.locator(".ple-app-ribbon__brand"), "the Peptidyle home control"],
    [currentCase.locator('[data-ribbon-control="courses"]'), "the Courses Tab"],
    [currentCase.locator('[data-ribbon-control="questions"]'), "the Questions Tab"],
    [currentCase.locator('[data-ribbon-control="productAssessments"]'), "the Assessments Tab"],
    [currentCase.getByRole("button", { name: "Profile" }), "the Profile control"],
    [
      currentCase.locator('[data-ribbon-control="myBlueprintCourses"]'),
      "the first backed task after the dense top bar",
    ],
  ];
  await assertDenseTopBarKeyboardTraversal(page, productKeyboardOrder);
  await currentCase.screenshot({
    path: "/private/tmp/ple_ribbon_shell_current_production_empty.png",
    fullPage: true,
  });

  await page.evaluate(() => window.ribbonShell.currentNavigate("/courses/CI7K3M2QAZ"));
  await waitForPath(page, "current-production", "/courses/CI7K3M2QAZ");
  await assertOneStableRibbon(page, "current-production", currentRibbon);
  const deferredBreadcrumb = currentCase.locator(".ple-shell__breadcrumb-prelude");
  assert.equal(
    await deferredBreadcrumb.count(),
    1,
    "a declared deep route reserves its shell-owned breadcrumb prelude while the title resolves",
  );
  assert.equal(
    await deferredBreadcrumb.locator('nav[aria-label="Breadcrumb"]').count(),
    1,
    "a deferred course scope retains the known role-home breadcrumb landmark",
  );
  assert.deepEqual(
    await deferredBreadcrumb.locator('nav[aria-label="Breadcrumb"] li').allTextContents(),
    ["Home", "Course"],
    "a deferred course scope exposes a stable human-readable current label",
  );
  assert.equal(
    await deferredBreadcrumb
      .locator('nav[aria-label="Breadcrumb"] [aria-current="page"]')
      .getAttribute("href"),
    "/courses/CI7K3M2QAZ",
    "the deferred current breadcrumb links to the canonical current Course URL",
  );
  const deferredBreadcrumbBox = await deferredBreadcrumb.boundingBox();
  const deferredRibbonBox = await currentCase.locator(".ple-app-ribbon").boundingBox();
  assert.equal(
    await currentCase.locator('.ple-app-ribbon[data-ribbon-scope="courseInstance"]').count(),
    1,
    "the current-source App harness admits the fixed course Ribbon immediately",
  );
  assert.equal(
    await currentCase.locator("[data-course-instance-id]").count(),
    0,
    "unresolved current-source harness course scope has no fabricated course theme ID",
  );
  assert.equal(
    await page.evaluate(() => window.ribbonShell.scopeRequestCount("CI7K3M2QAZ")),
    1,
    "the current-source scope controller starts exactly one deferred CI7K3M2QAZ request",
  );
  const courseInstance = currentCase.locator('[data-route-surface="courseInstance"]');
  await courseInstance.waitFor({ state: "visible" });
  assert.equal(
    await page.evaluate(() => window.ribbonShell.courseInstanceQueryCount()),
    1,
    "the current Course Instance surface resolves its exact direct-route identity once",
  );
  assert.equal(
    await page.evaluate(() => window.ribbonShell.assessmentQueryCount()),
    1,
    "the current Course Instance surface requests its direct-route Assessment list once",
  );
  await assertOneStableRibbon(page, "current-production", currentRibbon);
  await page.evaluate(() => window.ribbonShell.releaseCourseScope("CI7K3M2QAZ"));
  await page.waitForFunction(
    () =>
      document
        .querySelector('[data-m10-case="current-production"] [data-course-instance-id]')
        ?.getAttribute("data-course-instance-id") === "CI7K3M2QAZ",
    undefined,
    { timeout: 3_000 },
  );
  await page.waitForFunction(
    () =>
      document.querySelector(
        '[data-m10-case="current-production"] nav[aria-label="Breadcrumb"] [aria-current="page"]',
      )?.textContent === "BCHM 355/455 Section 20 Biochemistry (Roosevelt U; Spring 2026)",
    undefined,
    { timeout: 3_000 },
  );
  await assertOneStableRibbon(page, "current-production", currentRibbon);
  assert.equal(
    await currentCase.locator('nav[aria-label="Breadcrumb"]').count(),
    1,
    "the resolved deep route renders one shell-owned breadcrumb landmark",
  );
  assert.deepEqual(
    await currentCase.locator('nav[aria-label="Breadcrumb"] li').allInnerTexts(),
    ["Home", "BCHM 355/455 Section 20 Biochemistry (Roosevelt U; Spring 2026)"],
    "the resolved Course root omits its public ID from the human-readable current title",
  );
  assert.equal(
    await currentCase.locator('nav[aria-label="Breadcrumb"] [aria-current="page"]').count(),
    1,
    "the breadcrumb terminal is the only current item",
  );
  assert.equal(
    await currentCase
      .locator('nav[aria-label="Breadcrumb"] [aria-current="page"]')
      .getAttribute("href"),
    "/courses/CI7K3M2QAZ",
    "the resolved breadcrumb terminal points to the Course route",
  );
  assert.equal(
    await currentCase.locator('nav[aria-label="Breadcrumb"] a').first().getAttribute("href"),
    "/instructor",
    "the breadcrumb ancestor uses the canonical role-home path",
  );
  await assertBreadcrumbPreludeStyle(deferredBreadcrumb, "desktop");
  await page.setViewportSize({ width: 320, height: 640 });
  await assertBreadcrumbPreludeStyle(deferredBreadcrumb, "narrow");
  await page.setViewportSize({ width: 1280, height: 800 });
  assert.deepEqual(
    await deferredBreadcrumb.boundingBox(),
    deferredBreadcrumbBox,
    "label resolution preserves the reserved breadcrumb-prelude geometry",
  );
  assert.deepEqual(
    await currentCase.locator(".ple-app-ribbon").boundingBox(),
    deferredRibbonBox,
    "label resolution does not move any Ribbon control geometry",
  );
  assert.equal(
    await page.evaluate(() => window.ribbonShell.assessmentQueryCount()),
    1,
    "route-scope release does not restart the direct Course Assessment query",
  );
  assert.equal(
    await page.evaluate(() => window.ribbonShell.courseInstanceQueryCount()),
    1,
    "route-scope release does not restart the direct Course Instance query",
  );
  assert.equal(
    await courseInstance.getByText("Course Instance · CI7K3M2QAZ", { exact: true }).isVisible(),
    true,
    "the Course Instance surface owns its exact public-ID eyebrow",
  );
  assert.equal(
    await courseInstance
      .getByRole("heading", {
        name: "BCHM 355/455 Section 20 Biochemistry (Roosevelt U; Spring 2026)",
        level: 1,
      })
      .count(),
    1,
    "the Course Instance surface uses the descriptive Course name as its identity h1",
  );
  const newAssessment = courseInstance.getByRole("link", { name: "Create Assessment" });
  assert.equal(
    await newAssessment.count(),
    1,
    "the Course Instance surface has exactly one Create Assessment action",
  );
  assert.equal(
    await newAssessment.getAttribute("href"),
    "/instructor/courses/CI7K3M2QAZ/assessments/new",
    "Create Assessment uses the canonical instructor Course route",
  );
  assert.equal(
    await courseInstance.locator("[data-course-title], .course-entry-banner-container").count(),
    0,
    "the Course Instance surface does not include the Student-only Course entry banner",
  );
  assert.equal(
    await courseInstance.getByText("Course home", { exact: true }).count(),
    0,
    "the Course Instance surface has no student Course-home eyebrow",
  );
  assert.equal(
    await courseInstance.locator("h1").count(),
    1,
    "the Course Instance surface has no duplicate course-title h1",
  );
  await assertOneStableRibbon(page, "current-production", currentRibbon);
  const currentMainFocus = await currentCase.evaluate((root) => {
    const mainContent = root.querySelector("#main-content");
    if (!(mainContent instanceof HTMLElement)) {
      throw new Error("current-source App harness has no post-Ribbon main content target");
    }
    const style = getComputedStyle(mainContent);
    return {
      isFocused: document.activeElement === mainContent,
      outlineStyle: style.outlineStyle,
      outlineWidth: style.outlineWidth,
    };
  });
  assert.equal(
    currentMainFocus.isFocused,
    true,
    "a current-source harness course route transition focuses the one post-Ribbon content target",
  );
  assert.ok(
    currentMainFocus.outlineStyle === "none" || currentMainFocus.outlineWidth === "0px",
    "programmatic main-content focus suppresses the viewport-sized outline",
  );

  const currentSkip = currentCase.getByRole("link", { name: "Skip to learning content" });
  await currentSkip.focus();
  const skipFocus = await currentSkip.evaluate((skip) => {
    const style = getComputedStyle(skip);
    const bounds = skip.getBoundingClientRect();
    return {
      isFocused: document.activeElement === skip,
      isInViewport:
        bounds.top >= 0 &&
        bounds.left >= 0 &&
        bounds.bottom <= window.innerHeight &&
        bounds.right <= window.innerWidth,
      outlineStyle: style.outlineStyle,
      outlineWidth: style.outlineWidth,
      transform: style.transform,
    };
  });
  assert.equal(skipFocus.isFocused, true, "intentional skip-link focus reaches the link");
  assert.equal(skipFocus.isInViewport, true, "focused skip link is revealed within the viewport");
  assert.ok(
    skipFocus.outlineStyle !== "none" && skipFocus.outlineWidth !== "0px",
    "focused skip link retains a visible focus outline",
  );
  assert.equal(skipFocus.transform, "matrix(1, 0, 0, 1, 0, 0)", "focused skip link is revealed");
  await page.keyboard.press("Escape");
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  });
  const neutralSkip = await currentSkip.evaluate((skip) => {
    const bounds = skip.getBoundingClientRect();
    return {
      isFocused: document.activeElement === skip,
      isOffscreen: bounds.bottom < 0 || bounds.top > window.innerHeight,
    };
  });
  assert.deepEqual(
    neutralSkip,
    { isFocused: false, isOffscreen: true },
    "neutral evidence capture leaves the skip link unfocused and offscreen",
  );
  const courseCaptureReadiness = await page.evaluate(() => {
    const currentCaseRoot = document.querySelector('[data-m10-case="current-production"]');
    const fixtureCaseRoot = document.querySelector('[data-m10-case="fixture-shell"]');
    const mainContent = currentCaseRoot?.querySelector("#main-content");
    const skipLink = currentCaseRoot?.querySelector(".skip-link");
    if (
      !(currentCaseRoot instanceof HTMLElement) ||
      !(fixtureCaseRoot instanceof HTMLElement) ||
      !(mainContent instanceof HTMLElement) ||
      !(skipLink instanceof HTMLElement)
    ) {
      throw new Error("Course capture requires both evidence roots, main content, and skip link.");
    }
    const skipBounds = skipLink.getBoundingClientRect();
    const mainStyle = getComputedStyle(mainContent);
    return {
      mainIsNotOutlined: mainStyle.outlineStyle === "none" || mainStyle.outlineWidth === "0px",
      skipIsUnfocused: document.activeElement !== skipLink,
      skipIsOutsideViewport: skipBounds.bottom < 0 || skipBounds.top > window.innerHeight,
      fixtureIsNotInCaptureRoot: !currentCaseRoot.contains(fixtureCaseRoot),
    };
  });
  assert.deepEqual(
    courseCaptureReadiness,
    {
      mainIsNotOutlined: true,
      skipIsUnfocused: true,
      skipIsOutsideViewport: true,
      fixtureIsNotInCaptureRoot: true,
    },
    "isolated course capture has neutral focus and excludes the structural fixture",
  );
  await currentCase.screenshot({
    path: "/private/tmp/ple_ribbon_m11_course_assessments.png",
    // A fixed, offscreen skip link would otherwise be painted into this
    // element capture. The assertions immediately above prove it is neither
    // focused nor visible; this screenshot-only stylesheet never changes the
    // source stylesheet cascade or hides an interactive visible control.
    style: ".skip-link:not(:focus) { visibility: hidden !important; }",
  });

  // The routed ApplicationShell is the M2 visual evidence surface. At phone
  // width and a 200% root-text setting it must keep the dense Ribbon, its
  // shell-owned breadcrumb prelude, and the beginning of route content in one
  // readable document flow without horizontal overflow.
  await page.setViewportSize({ width: 320, height: 640 });
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "200%";
  });
  await flush(page);
  await assertNarrowRoutedShell(currentCase);
  await page.screenshot({
    path: "/private/tmp/ple_ribbon_m2_routed_shell_320x640_text200.png",
    // Preserve the neutral capture conditions proved above while presenting the
    // actual Ribbon, breadcrumb, and route content at this accessibility size.
    style: ".skip-link:not(:focus) { visibility: hidden !important; }",
  });
  await page.setViewportSize({ width: 393, height: 852 });
  await flush(page);
  await page.screenshot({
    path: "/private/tmp/ple_course_breadcrumb_393x852_text200.png",
    style: ".skip-link:not(:focus) { visibility: hidden !important; }",
  });
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "";
  });
  await page.setViewportSize({ width: 1280, height: 800 });
  await flush(page);

  await page.evaluate(() => window.ribbonShell.currentNavigate("/courses/CI7K3M2"));
  await waitForPath(page, "current-production", "/courses/CI7K3M2");
  await assertOneStableRibbon(page, "current-production", currentRibbon);
  assert.deepEqual(
    await currentRibbon.evaluate((ribbon) => ({
      scope: ribbon.getAttribute("data-ribbon-scope"),
      tierOneControlIds: [
        ...ribbon.querySelectorAll('nav[aria-label="Ribbon tabs"] [data-ribbon-control-id]'),
      ].map((control) => control.getAttribute("data-ribbon-control-id")),
      taskControls: [
        ...ribbon.querySelectorAll('nav[aria-label="Ribbon tasks"] [data-ribbon-control]'),
      ].map((control) => control.getAttribute("data-ribbon-control")),
    })),
    {
      scope: "courseInstance",
      tierOneControlIds: ["courses", "questions", "productAssessments"],
      taskControls: [],
    },
    "a malformed Course Instance ID keeps the declared data-free Course Instance Ribbon",
  );

  await page.evaluate(() => window.ribbonShell.currentNavigate("/assessment-attempts/R-0"));
  await waitForPath(page, "current-production", "/assessment-attempts/R-0");
  await assertOneStableRibbon(page, "current-production", currentRibbon);
  assert.deepEqual(
    await currentRibbon.evaluate((ribbon) => ({
      scope: ribbon.getAttribute("data-ribbon-scope"),
      tierOneControlIds: [
        ...ribbon.querySelectorAll('nav[aria-label="Ribbon tabs"] [data-ribbon-control-id]'),
      ].map((control) => control.getAttribute("data-ribbon-control-id")),
      taskControls: [
        ...ribbon.querySelectorAll('nav[aria-label="Ribbon tasks"] [data-ribbon-control]'),
      ].map((control) => control.getAttribute("data-ribbon-control")),
    })),
    {
      scope: "assessmentAttempt",
      tierOneControlIds: ["courses", "questions", "productAssessments"],
      taskControls: [],
    },
    "a malformed Assessment Attempt ID keeps the declared data-free Attempt Ribbon",
  );

  await page.evaluate(() =>
    window.ribbonShell.currentNavigate("/instructor/courses/CI7K3M2QAZ/students"),
  );
  await waitForPath(page, "current-production", "/instructor/courses/CI7K3M2QAZ/students");
  currentRibbon = await currentCase.locator(".ple-app-ribbon").elementHandle();
  assert.notEqual(
    currentRibbon,
    null,
    "a valid scoped route restores its declared Ribbon schema after malformed scope handling",
  );
  await assertOneStableRibbon(page, "current-production", currentRibbon);

  for (const pathname of [
    "/library",
    "/blueprint-courses",
    "/courses/CI4W8QF9AD",
    "/assessment-attempts/00000000-0000-0000-0000-000000000001",
    "/courses/CI7K3M2QAZ",
  ]) {
    await page.evaluate(
      (nextPathname) => window.ribbonShell.currentNavigate(nextPathname),
      pathname,
    );
    await waitForPath(page, "current-production", pathname);
    try {
      await assertOneStableRibbon(page, "current-production", currentRibbon);
    } catch (error) {
      throw new Error(`current-source Ribbon topology disagreed at ${pathname}`, { cause: error });
    }
  }
  assert.deepEqual(
    pageErrors,
    [],
    "current-source route transitions, including the active Attempt scope, raise no page errors",
  );

  await currentSkip.focus();
  await page.keyboard.press("Enter");
  await flush(page);
  assert.equal(
    await currentCase.evaluate(
      (root) => root.querySelector("#main-content") === document.activeElement,
    ),
    true,
    "current-source App harness skip link focuses the one post-Ribbon content target",
  );

  await page.evaluate(() => window.ribbonShell.currentNavigate("/sign-in"));
  await waitForPath(page, "current-production", "/sign-in");
  assert.equal(
    await currentCase.locator(".ple-app-ribbon").count(),
    0,
    "public sign-in does not fabricate a Ribbon",
  );
  assert.equal(
    await currentCase.locator(".ple-ribbon-shell-grid").count(),
    0,
    "public sign-in reserves no phantom Ribbon row",
  );
  assert.equal(
    await currentCase.locator(".ple-shell-frame").count(),
    1,
    "the no-Ribbon sign-in surface keeps the same viewport-owning shell frame",
  );
  assert.equal(
    await currentCase.locator(".site-header").count(),
    1,
    "the no-Ribbon sign-in surface retains its identity header",
  );
  assert.equal(
    await currentCase.locator('.sr-only[role="status"][aria-live="polite"]').count(),
    1,
    "the no-Ribbon sign-in surface retains the sign-out live region",
  );
  await page.evaluate(() => window.ribbonShell.currentNavigate("/not-a-route"));
  await waitForPath(page, "current-production", "/not-a-route");
  assert.equal(
    await currentCase.locator(".ple-app-ribbon").count(),
    0,
    "unknown route does not fabricate a Ribbon",
  );
  assert.equal(
    await currentCase.locator(".ple-ribbon-shell-grid").count(),
    0,
    "unknown routes reserve no phantom Ribbon row",
  );

  await page.evaluate(() => window.ribbonShell.currentNavigate("/instructor"));
  await waitForPath(page, "current-production", "/instructor");
  await currentCase.getByRole("button", { name: "Profile" }).click();
  await currentCase.getByRole("menuitem", { name: "Sign out" }).click();
  await waitForPath(page, "current-production", "/sign-in");
  assert.equal(
    await currentCase.locator(".ple-app-ribbon").count(),
    0,
    "current-source harness sign-out removes the Ribbon before public sign-in",
  );
  assert.equal(
    await currentCase.locator(".ple-ribbon-shell-grid").count(),
    0,
    "sign-out removes the Ribbon grid before public sign-in",
  );
  await page.evaluate(() => window.ribbonShell.currentNavigate("/courses/CI7K3M2QAZ"));
  await waitForPath(page, "current-production", "/courses/CI7K3M2QAZ");
  await currentCase.getByRole("heading", { name: "You are signed out" }).waitFor({
    state: "visible",
  });
  assert.equal(
    await currentCase.locator('[data-session-state="signedOut"]').count(),
    1,
    "a protected route after harness sign-out presents session recovery",
  );
  assert.equal(
    await currentCase.locator(".ple-app-ribbon").count(),
    0,
    "session recovery after harness sign-out does not fabricate a Ribbon",
  );
  assert.equal(
    await currentCase.locator(".ple-ribbon-shell-grid").count(),
    0,
    "session recovery reserves no phantom Ribbon row",
  );

  // Case B: ApplicationShell with an explicit structural model fixture.
  // Only one MemoryRouter may own document-level anchor interception at a
  // time. The current-source case is complete before fixture clicks begin.
  await page.evaluate(() => window.ribbonShell.disposeCurrent());
  const fixtureCase = caseLocator(page, "fixture-shell");
  await waitForPath(page, "fixture-shell", "/");
  await fixtureCase.locator(".ple-app-ribbon").waitFor({ state: "visible" });
  const fixtureRibbon = await fixtureCase.locator(".ple-app-ribbon").elementHandle();
  assert.notEqual(fixtureRibbon, null, "fixture mounts the ApplicationShell Ribbon");

  const fixtureTransitions = [
    {
      pathname: "/courses/CI7K3M2QAZ",
      scope: "courseInstance",
      selected: "Assessments",
    },
    {
      pathname: "/instructor/courses/CI7K3M2QAZ/students",
      scope: "courseInstance",
      selected: "Assessments",
    },
    {
      pathname: "/instructor/courses/CI7K3M2QAZ/gradebook",
      scope: "courseInstance",
      selected: "Assessments",
    },
    {
      pathname: "/library",
      scope: "product",
      selected: "Questions",
    },
    {
      pathname: "/blueprint-courses",
      scope: "product",
      selected: "Courses",
    },
    {
      pathname: "/courses/CI4W8QF9AD",
      scope: "courseInstance",
      selected: "Assessments",
    },
    {
      pathname: "/assessment-attempts/00000000-0000-0000-0000-000000000001",
      scope: "assessmentAttempt",
      selected: undefined,
    },
    {
      pathname: "/courses/CI7K3M2QAZ",
      scope: "courseInstance",
      selected: "Assessments",
    },
  ];
  for (const transition of fixtureTransitions) {
    await page.evaluate(
      (nextPathname) => window.ribbonShell.fixtureNavigate(nextPathname),
      transition.pathname,
    );
    await waitForPath(page, "fixture-shell", transition.pathname);
    await assertOneStableRibbon(page, "fixture-shell", fixtureRibbon);
    assert.equal(
      await fixtureCase.locator(`.ple-app-ribbon[data-ribbon-scope="${transition.scope}"]`).count(),
      1,
      `fixture projection exposes ${transition.scope} at ${transition.pathname}`,
    );
    const selected = fixtureCase.locator('nav[aria-label="Ribbon tabs"] a[aria-current="page"]');
    if (transition.selected === undefined) {
      assert.equal(
        await selected.count(),
        0,
        "attempt fixture has no fabricated selected destination",
      );
    } else {
      assert.equal(
        await selected.innerText(),
        transition.selected,
        `fixture projection updates the selected Tab at ${transition.pathname}`,
      );
    }
  }

  await page.evaluate(() => window.ribbonShell.fixtureNavigate("/courses/CI7K3M2QAZ"));
  await waitForPath(page, "fixture-shell", "/courses/CI7K3M2QAZ");
  await fixtureCase
    .locator('[data-course-instance-id="CI7K3M2QAZ"]')
    .waitFor({ state: "attached" });
  const fixtureThemeScope = fixtureCase.locator(".course-theme-scope");
  assert.equal(
    await fixtureThemeScope.getAttribute("data-course-theme"),
    "grass",
    "fixture begins from its route-supplied course appearance",
  );
  await fixtureCase.getByRole("button", { name: "Present Ocean course theme" }).click();
  await fixtureThemeScope.evaluate((scope) => {
    if (scope.getAttribute("data-course-theme") !== "ocean") {
      throw new Error("presentation setter did not update data-course-theme");
    }
    if (getComputedStyle(scope).getPropertyValue("--ple-theme-canvas").trim() !== "#ddeff5") {
      throw new Error("presentation setter did not update the course theme variables");
    }
  });
  await assertOneStableRibbon(page, "fixture-shell", fixtureRibbon);

  await page.evaluate(() => window.ribbonShell.throwFixtureContent(true));
  await page.evaluate(() => window.ribbonShell.fixtureNavigate("/courses/CI4W8QF9AD"));
  await fixtureCase.getByRole("alert").waitFor({ state: "visible" });
  await assertOneStableRibbon(page, "fixture-shell", fixtureRibbon);

  const tab = fixtureCase.getByRole("link", { name: "Assessments" });
  await tab.waitFor({ state: "visible" });
  await tab.click();
  assert.equal(
    await page.evaluate(() => window.ribbonShell.fixturePathname()),
    "/assessments/due-soon",
    "visible Tab activation changes the controlled content route while its error remains contained",
  );
  await assertOneStableRibbon(page, "fixture-shell", fixtureRibbon);
  await page.evaluate(() =>
    window.ribbonShell.fixtureNavigate("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF"),
  );
  await page.waitForFunction(
    () =>
      window.ribbonShell.fixturePathname() ===
        "/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF" &&
      document.querySelector(
        '[data-m10-case="fixture-shell"] section[data-ribbon-row-frame="tasks"]',
      ) !== null,
  );
  await flush(page);
  await assertOneStableRibbon(page, "fixture-shell", fixtureRibbon);
  const task = fixtureCase
    .getByRole("navigation", { name: "Ribbon tasks" })
    .getByRole("link", { name: "Questions" });
  await task.waitFor({ state: "visible" });
  await task.focus();
  await page.keyboard.press("Enter");
  assert.equal(
    await page.evaluate(() => window.ribbonShell.fixturePathname()),
    "/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/questions",
    "keyboard Task activation changes the controlled content route while its " +
      "error remains contained",
  );
  await assertOneStableRibbon(page, "fixture-shell", fixtureRibbon);
  await fixtureCase.screenshot({
    path: "/private/tmp/ple_ribbon_shell_fixture_shell.png",
    fullPage: true,
  });

  await page.evaluate(() => window.ribbonShell.throwFixtureContent(false));
  await fixtureCase.getByRole("button", { name: "Try this page again" }).click();
  await fixtureCase
    .getByRole("heading", { name: "Fixture page", exact: true })
    .waitFor({ state: "visible" });
  await assertOneStableRibbon(page, "fixture-shell", fixtureRibbon);

  await fixtureCase.evaluate((root) => {
    root.querySelector(".ple-app-ribbon")?.dispatchEvent(
      new CustomEvent("ple-ribbon-action", {
        bubbles: true,
        detail: { id: "not-sign-out", kind: "action" },
      }),
    );
  });
  await flush(page);
  assert.equal(
    await page.evaluate(() => window.ribbonShell.signOutActions()),
    0,
    "invalid custom action is ignored",
  );
  await fixtureCase.evaluate((root) => {
    root.querySelector(".ple-app-ribbon")?.dispatchEvent(
      new CustomEvent("ple-ribbon-action", {
        bubbles: true,
        detail: { id: "signOut", kind: "action" },
      }),
    );
  });
  await flush(page);
  assert.equal(
    await page.evaluate(() => window.ribbonShell.signOutActions()),
    1,
    "valid closed action is handled once",
  );
  const fixtureProfile = fixtureCase.getByRole("button", { name: "Profile" });
  await fixtureProfile.focus();
  await page.keyboard.press("ArrowDown");
  const fixtureMenu = fixtureCase.getByRole("menu", { name: "Profile menu" });
  await fixtureMenu.getByRole("menuitem", { name: "Sign out" }).click();
  await flush(page);
  assert.equal(
    await page.evaluate(() => window.ribbonShell.signOutActions()),
    2,
    "fixture click reaches the controlled sign-out callback once",
  );

  assert.deepEqual(
    pageErrors,
    [],
    "compiled-harness App and structural-shell transitions raise no page errors",
  );
  assert.deepEqual(
    consoleErrors,
    [],
    "compiled-harness App and structural-shell transitions emit no console errors",
  );
  process.stdout.write(
    "Compiled-harness application-shell evidence passed: current-source routed application " +
      "and structural fixture shell; not dist or real-stack browser acceptance.\n",
  );
} finally {
  await browser.close();
  await harnessServer.close();
}
