// ribbon_m10_shell_evidence.mjs - compiled-harness application-shell structural evidence.
// This exercises current source composition in a controlled browser fixture; it
// does not build dist/ or replace real-stack browser acceptance.

import assert from "node:assert/strict";
import { once } from "node:events";
import { readFileSync } from "node:fs";
import { createServer } from "node:http";

import { chromium } from "playwright";

import { bundleRibbonM10ShellHarness } from "../support/ribbon_m10_shell_loader.ts";

const globalCss = readFileSync(new URL("../../src/style.css", import.meta.url), "utf8");
const accessibilityCss = readFileSync(
  new URL("../../src/styles/accessibility.css", import.meta.url),
  "utf8",
);
const bundle = await bundleRibbonM10ShellHarness();
const markup = [
  "<!doctype html><html><head><style>",
  globalCss,
  accessibilityCss,
  bundle.stylesheet,
  'html,body{margin:0;min-inline-size:0}</style></head><body><div id="root"></div></body></html>',
].join("\n");

const server = createServer((_request, response) => {
  response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
  response.end(markup);
});
server.listen(0, "127.0.0.1");
await once(server, "listening");
const address = server.address();
if (address === null || typeof address === "string") {
  throw new Error("Application-shell evidence server did not receive a TCP address.");
}
const evidenceUrl = `http://127.0.0.1:${String(address.port)}/`;

function formatBootstrapDiagnostics(pageErrors, consoleErrors) {
  const details = [...pageErrors, ...consoleErrors];
  const summary = details.length === 0 ? "no page or console errors" : details.join(" | ");
  return `Application-shell harness bootstrap failed: ${summary}`;
}

async function mountHarness(page, pageErrors, consoleErrors) {
  try {
    await page.addScriptTag({ content: Buffer.from(bundle.javascript).toString("utf8") });
    await page.waitForFunction(
      () =>
        "PleRibbonM10Harness" in window &&
        typeof window.PleRibbonM10Harness.mountRibbonM10ShellHarness === "function",
      undefined,
      { timeout: 5_000 },
    );
    await page.evaluate(() => {
      const target = document.querySelector("#root");
      if (!(target instanceof HTMLElement))
        throw new Error("Application-shell harness root is missing.");
      window.ribbonM10 = window.PleRibbonM10Harness.mountRibbonM10ShellHarness(target);
    });
    await page.waitForFunction(() => "ribbonM10" in window, undefined, { timeout: 5_000 });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`${formatBootstrapDiagnostics(pageErrors, consoleErrors)}; ${message}`, {
      cause: error,
    });
  }
}

async function flush(page) {
  await page.evaluate(() => new Promise((resolve) => queueMicrotask(resolve)));
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(resolve)));
}

function caseLocator(page, evidenceCase) {
  return page.locator(`[data-m10-case="${evidenceCase}"]`);
}

async function waitForPath(page, evidenceCase, pathname) {
  if (evidenceCase === "current-production") {
    await page.waitForFunction(
      (expectedPath) => window.ribbonM10.currentPathname() === expectedPath,
      pathname,
      { timeout: 3_000 },
    );
    await flush(page);
    return;
  }
  const evidenceCaseLocator = caseLocator(page, evidenceCase);
  try {
    await evidenceCaseLocator
      .locator("[data-current-path]")
      .waitFor({ state: "attached", timeout: 3_000 });
  } catch (error) {
    const visibleText = await evidenceCaseLocator.innerText().catch(() => "case root missing");
    throw new Error(
      `${evidenceCase} did not mount its compiled-harness content boundary at ${pathname}: ` +
        visibleText,
      { cause: error },
    );
  }
  await page.waitForFunction(
    ({ evidenceCase: expectedCase, pathname: expectedPath }) =>
      document
        .querySelector(`[data-m10-case="${expectedCase}"] [data-current-path]`)
        ?.getAttribute("data-current-path") === expectedPath,
    { evidenceCase, pathname },
    { timeout: 3_000 },
  );
  await flush(page);
}

async function assertOneStableRibbon(page, evidenceCase, initialRibbon) {
  const evidence = await caseLocator(page, evidenceCase).evaluate(
    (root, initial) => ({
      sameRibbon: root.querySelector(".ple-app-ribbon") === initial,
      roots: root.querySelectorAll(".ple-app-ribbon").length,
      shellGrids: root.querySelectorAll(".ple-ribbon-shell-grid").length,
      landmarks: root.querySelectorAll('[aria-label="PLE application Ribbon"]').length,
      tabs: root.querySelectorAll('nav[aria-label="Ribbon tabs"]').length,
      tasks: root.querySelectorAll('nav[aria-label="Ribbon tasks"]').length,
      legacyCourse: root.querySelectorAll('nav[aria-label="Course management"]').length,
      legacyPrimary: root.querySelectorAll('nav[aria-label="Primary navigation"]').length,
      legacyWorkspace: root.querySelectorAll('nav[aria-label="Assignment workspace"]').length,
    }),
    initialRibbon,
  );
  assert.deepEqual(evidence, {
    sameRibbon: true,
    roots: 1,
    shellGrids: 1,
    landmarks: 1,
    tabs: 1,
    tasks: 1,
    legacyCourse: 0,
    legacyPrimary: 0,
    legacyWorkspace: 0,
  });
}

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const pageErrors = [];
  const consoleErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });
  await page.goto(evidenceUrl);
  await mountHarness(page, pageErrors, consoleErrors);

  // Case A: current-source App composition with its router, routes, and providers.
  await page.evaluate(() => window.ribbonM10.currentNavigate("/"));
  await waitForPath(page, "current-production", "/");
  const currentCase = caseLocator(page, "current-production");
  await currentCase.locator(".ple-app-ribbon").waitFor({ state: "visible" });
  let currentRibbon = await currentCase.locator(".ple-app-ribbon").elementHandle();
  assert.notEqual(currentRibbon, null, "current App mounts one Ribbon on an authenticated route");
  await assertOneStableRibbon(page, "current-production", currentRibbon);
  assert.equal(
    await currentCase
      .locator('nav[aria-label="Ribbon tabs"] a, nav[aria-label="Ribbon tasks"] a')
      .count(),
    0,
    "current all-unbacked registry truthfully omits destination controls",
  );
  await currentCase.screenshot({
    path: "/private/tmp/ple_ribbon_m10_current_production_empty.png",
    fullPage: true,
  });

  await page.evaluate(() => window.ribbonM10.currentNavigate("/courses/C-1"));
  await waitForPath(page, "current-production", "/courses/C-1");
  await assertOneStableRibbon(page, "current-production", currentRibbon);
  assert.equal(
    await currentCase.locator('.ple-app-ribbon[data-ribbon-scope="courseInstance"]').count(),
    1,
    "the current-source App harness admits the fixed course Ribbon immediately " +
      "while its scope label is deferred",
  );
  assert.equal(
    await currentCase.locator(".ple-app-ribbon__course-scope-label").count(),
    0,
    "unresolved current-source harness course scope has no fabricated course label",
  );
  assert.equal(
    await currentCase.locator("[data-course-reference]").count(),
    0,
    "unresolved current-source harness course scope has no fabricated course theme reference",
  );
  assert.equal(
    await page.evaluate(() => window.ribbonM10.scopeRequestCount("C-1")),
    1,
    "the current-source scope controller starts exactly one deferred C-1 request",
  );
  const deferredAssignments = currentCase.locator('[role="status"].loading-state');
  await deferredAssignments.waitFor({ state: "visible" });
  assert.equal(
    await deferredAssignments.innerText(),
    "Loading assignments...",
    "the current-source course Assignments page owns its deferred content status",
  );
  assert.equal(
    await page.evaluate(() => window.ribbonM10.assignmentQueryCount()),
    0,
    "the deferred course boundary starts no course-assignments query before scope release",
  );
  await assertOneStableRibbon(page, "current-production", currentRibbon);
  await page.evaluate(() => window.ribbonM10.releaseCourseScope("C-1"));
  await currentCase.locator(".ple-app-ribbon__course-scope-label").waitFor({ state: "visible" });
  await page.waitForFunction(
    () =>
      document
        .querySelector('[data-m10-case="current-production"] [data-course-reference]')
        ?.getAttribute("data-course-reference") === "C-1",
    undefined,
    { timeout: 3_000 },
  );
  await assertOneStableRibbon(page, "current-production", currentRibbon);
  assert.equal(
    await currentCase.locator(".ple-app-ribbon__course-scope-label").innerText(),
    "Course C-1",
    "released current-source harness scope supplies the truthful C-1 label " +
      "without remounting Ribbon",
  );
  const courseAssignments = currentCase.locator('[data-route-surface="courseAssignments"]');
  await courseAssignments.waitFor({ state: "visible" });
  assert.equal(
    await page.evaluate(() => window.ribbonM10.assignmentQueryCount()),
    1,
    "released current-source harness course scope starts the typed course-assignments query once",
  );
  assert.equal(
    await courseAssignments.locator(".course-assignments-identity .eyebrow").textContent(),
    "Instructor course",
    "the instructor course Assignments surface owns its concise course eyebrow",
  );
  assert.equal(
    await courseAssignments.getByRole("heading", { name: "Assignments", level: 1 }).count(),
    1,
    "the instructor course Assignments surface owns exactly one page h1",
  );
  const newAssignment = courseAssignments.getByRole("link", { name: "New assignment" });
  assert.equal(
    await newAssignment.count(),
    1,
    "the instructor course Assignments surface has exactly one New assignment page action",
  );
  assert.equal(
    await newAssignment.getAttribute("href"),
    "/instructor/courses/C-1/assignments/new",
    "New assignment uses the canonical instructor course route",
  );
  assert.equal(
    await courseAssignments.locator("[data-course-title], .course-entry-identity").count(),
    0,
    "the instructor course Assignments surface does not retain student Course home identity",
  );
  assert.equal(
    await courseAssignments.getByText("Course home", { exact: true }).count(),
    0,
    "the instructor course Assignments surface has no student Course home eyebrow",
  );
  assert.equal(
    await courseAssignments.locator("h1").count(),
    1,
    "the instructor course Assignments surface has no duplicate course-title h1",
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
    path: "/private/tmp/ple_ribbon_m11_course_assignments.png",
    // A fixed, offscreen skip link would otherwise be painted into this
    // element capture. The assertions immediately above prove it is neither
    // focused nor visible; this screenshot-only stylesheet never changes the
    // source stylesheet cascade or hides an interactive visible control.
    style: ".skip-link:not(:focus) { visibility: hidden !important; }",
  });

  await page.evaluate(() => window.ribbonM10.currentNavigate("/courses/C-0"));
  await waitForPath(page, "current-production", "/courses/C-0");
  await assertOneStableRibbon(page, "current-production", currentRibbon);
  assert.deepEqual(
    await currentRibbon.evaluate((ribbon) => ({
      scope: ribbon.getAttribute("data-ribbon-scope"),
      label: ribbon.querySelector(".ple-app-ribbon__course-scope-label")?.textContent,
      controls: [...ribbon.querySelectorAll("[data-ribbon-control]")].map((control) => ({
        unavailable: control.getAttribute("data-ribbon-availability"),
        href: control.getAttribute("href"),
      })),
    })),
    {
      scope: "courseInstance",
      label: undefined,
      controls: [],
    },
    "a malformed Course reference keeps the declared data-free Course Instance Ribbon",
  );

  await page.evaluate(() => window.ribbonM10.currentNavigate("/assignment-attempts/R-0"));
  await waitForPath(page, "current-production", "/assignment-attempts/R-0");
  await assertOneStableRibbon(page, "current-production", currentRibbon);
  assert.deepEqual(
    await currentRibbon.evaluate((ribbon) => ({
      scope: ribbon.getAttribute("data-ribbon-scope"),
      label: ribbon.querySelector(".ple-app-ribbon__course-scope-label")?.textContent,
      controls: [...ribbon.querySelectorAll("[data-ribbon-control]")].map((control) => ({
        unavailable: control.getAttribute("data-ribbon-availability"),
        href: control.getAttribute("href"),
      })),
    })),
    {
      scope: "assignmentAttempt",
      label: undefined,
      controls: [],
    },
    "a malformed Assignment Attempt reference keeps the declared data-free Attempt Ribbon",
  );

  await page.evaluate(() => window.ribbonM10.currentNavigate("/instructor/courses/C-1/students"));
  await waitForPath(page, "current-production", "/instructor/courses/C-1/students");
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
    "/courses/C-2",
    "/assignment-attempts/R-1",
    "/courses/C-1",
  ]) {
    await page.evaluate((nextPathname) => window.ribbonM10.currentNavigate(nextPathname), pathname);
    await waitForPath(page, "current-production", pathname);
    await assertOneStableRibbon(page, "current-production", currentRibbon);
  }

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

  await page.evaluate(() => window.ribbonM10.currentNavigate("/sign-in"));
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
  await page.evaluate(() => window.ribbonM10.currentNavigate("/not-a-route"));
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

  await page.evaluate(() => window.ribbonM10.currentNavigate("/"));
  await waitForPath(page, "current-production", "/");
  const currentSignOut = currentCase.getByRole("button", { name: "Sign out" });
  await currentSignOut.click();
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
  await page.evaluate(() => window.ribbonM10.currentNavigate("/courses/C-1"));
  await waitForPath(page, "current-production", "/courses/C-1");
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
  await page.evaluate(() => window.ribbonM10.disposeCurrent());
  const fixtureCase = caseLocator(page, "fixture-shell");
  await waitForPath(page, "fixture-shell", "/");
  await fixtureCase.locator(".ple-app-ribbon").waitFor({ state: "visible" });
  const fixtureRibbon = await fixtureCase.locator(".ple-app-ribbon").elementHandle();
  assert.notEqual(fixtureRibbon, null, "fixture mounts the ApplicationShell Ribbon");

  const fixtureTransitions = [
    {
      pathname: "/courses/C-1",
      scope: "courseInstance",
      selected: "Assignments",
      context: "Course C-1",
    },
    {
      pathname: "/instructor/courses/C-1/students",
      scope: "courseInstance",
      selected: "Students",
      context: "Course C-1",
    },
    {
      pathname: "/instructor/courses/C-1/gradebook",
      scope: "courseInstance",
      selected: "Gradebook",
      context: "Course C-1",
    },
    { pathname: "/library", scope: "product", selected: "Question Library", context: undefined },
    {
      pathname: "/blueprint-courses",
      scope: "product",
      selected: "Blueprint Courses",
      context: undefined,
    },
    {
      pathname: "/courses/C-2",
      scope: "courseInstance",
      selected: "Assignments",
      context: "Course C-2",
    },
    {
      pathname: "/assignment-attempts/R-1",
      scope: "assignmentAttempt",
      selected: undefined,
      context: undefined,
    },
    {
      pathname: "/courses/C-1",
      scope: "courseInstance",
      selected: "Assignments",
      context: "Course C-1",
    },
  ];
  for (const transition of fixtureTransitions) {
    await page.evaluate(
      (nextPathname) => window.ribbonM10.fixtureNavigate(nextPathname),
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
    const contextLabels = fixtureCase.locator(".ple-app-ribbon__course-scope-label");
    if (transition.context === undefined) {
      assert.equal(
        await contextLabels.count(),
        0,
        "product and attempt fixture scopes omit a course label",
      );
    } else {
      assert.equal(
        await contextLabels.innerText(),
        transition.context,
        `fixture projection exposes its truthful course context at ${transition.pathname}`,
      );
    }
  }

  await page.evaluate(() => window.ribbonM10.fixtureNavigate("/courses/C-1"));
  await waitForPath(page, "fixture-shell", "/courses/C-1");
  await fixtureCase.locator('[data-course-reference="C-1"]').waitFor({ state: "attached" });
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

  await page.evaluate(() => window.ribbonM10.throwFixtureContent(true));
  await page.evaluate(() => window.ribbonM10.fixtureNavigate("/courses/C-2"));
  await fixtureCase.getByRole("alert").waitFor({ state: "visible" });
  await assertOneStableRibbon(page, "fixture-shell", fixtureRibbon);

  const tab = fixtureCase.getByRole("link", { name: "Assignments" });
  const task = fixtureCase.getByRole("link", { name: "Overview" });
  await tab.waitFor({ state: "visible" });
  await task.waitFor({ state: "visible" });
  await tab.click();
  assert.equal(
    await page.evaluate(() => window.ribbonM10.fixturePathname()),
    "/courses/C-1",
    "visible Tab activation changes the controlled content route while its error remains contained",
  );
  await assertOneStableRibbon(page, "fixture-shell", fixtureRibbon);
  await task.focus();
  await page.keyboard.press("Enter");
  assert.equal(
    await page.evaluate(() => window.ribbonM10.fixturePathname()),
    "/instructor/courses/C-1/assignments/A-1",
    "keyboard Task activation changes the controlled content route while its " +
      "error remains contained",
  );
  await assertOneStableRibbon(page, "fixture-shell", fixtureRibbon);
  await fixtureCase.screenshot({
    path: "/private/tmp/ple_ribbon_m10_fixture_shell.png",
    fullPage: true,
  });

  await page.evaluate(() => window.ribbonM10.throwFixtureContent(false));
  await fixtureCase.getByRole("button", { name: "Try this page again" }).click();
  await fixtureCase.locator("[data-m10-fixture-content]").waitFor({ state: "visible" });
  await assertOneStableRibbon(page, "fixture-shell", fixtureRibbon);

  const fixtureSignOut = fixtureCase.getByRole("button", { name: "Sign out" });
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
    await page.evaluate(() => window.ribbonM10.signOutActions()),
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
    await page.evaluate(() => window.ribbonM10.signOutActions()),
    1,
    "valid closed action is handled once",
  );
  await fixtureSignOut.click();
  await flush(page);
  assert.equal(
    await page.evaluate(() => window.ribbonM10.signOutActions()),
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
    "Compiled-harness application-shell evidence passed: current-source empty admission " +
      "and populated structural fixture shell; not dist or real-stack browser acceptance.\n",
  );
} finally {
  await browser.close();
  server.close();
  await once(server, "close");
}
