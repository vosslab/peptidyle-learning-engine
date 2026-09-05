// Compiled-DOM acceptance evidence for pending ownership and selected Tab visibility.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { bundleRibbonM8IntegrationHarness } from "../support/ribbon_m8_integration_loader.ts";

const globalCss = readFileSync(new URL("../../src/style.css", import.meta.url), "utf8");
const accessibilityCss = readFileSync(
  new URL("../../src/styles/accessibility.css", import.meta.url),
  "utf8",
);
const bundle = await bundleRibbonM8IntegrationHarness();
const bundleBase64 = Buffer.from(bundle.javascript).toString("base64");
const bundleUrl = `data:text/javascript;base64,${bundleBase64}`;
const markup = [
  "<!doctype html><html><head><style>",
  globalCss,
  "\n",
  accessibilityCss,
  "\n",
  bundle.stylesheet,
  "\nhtml,body{margin:0}.m8-root{inline-size:6rem}</style></head><body>",
  '<div id="root" class="m8-root"></div><script type="module">',
  `import { mountRibbonM8IntegrationHarness } from "${bundleUrl}";`,
  'window.ribbonM8 = mountRibbonM8IntegrationHarness(document.querySelector("#root"));',
  'document.addEventListener("click", event => event.preventDefault());',
  "window.scrollCalls = [];",
  "HTMLElement.prototype.scrollIntoView = function(options) {",
  "  window.scrollCalls.push({ id: this.dataset.ribbonControl, options });",
  "};",
  "</script></body></html>",
].join("");

async function flush(page) {
  await page.evaluate(() => new Promise((resolve) => queueMicrotask(resolve)));
}

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 640, height: 300 } });
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.setContent(markup);
  await page.waitForFunction(() => "ribbonM8" in window);
  await flush(page);

  const evidence = await page.evaluate(async () => {
    const harness = window.ribbonM8;
    const selected = () => document.querySelector('[aria-current="page"]');
    const link = (id) => {
      const element = document.querySelector(`[data-ribbon-control="${id}"]`);
      if (!(element instanceof HTMLAnchorElement)) throw new Error(`missing Ribbon link ${id}`);
      return element;
    };
    const click = (element, init = {}) =>
      element.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, ...init }));
    const flushMicrotasks = () => new Promise((resolve) => queueMicrotask(resolve));
    const busyIds = () =>
      [...document.querySelectorAll('[data-ribbon-pending="true"]')].map(
        (element) => element.dataset.ribbonControl,
      );

    const externalBefore = busyIds();
    harness.setRoutingInFlight(true);
    const externalInFlight = busyIds();
    harness.setRoutingInFlight(false);

    const students = link("students");
    const before = students.getBoundingClientRect().toJSON();
    click(students);
    harness.setRoutingInFlight(true);
    const pending = {
      busyIds: busyIds(),
      ariaBusy: students.getAttribute("aria-busy"),
      before,
      style: {
        backgroundImage: getComputedStyle(students).backgroundImage,
        boxShadow: getComputedStyle(students).boxShadow,
      },
      after: students.getBoundingClientRect().toJSON(),
    };
    harness.setRoutingInFlight(false);
    await flushMicrotasks();
    const settled = busyIds();

    click(students);
    harness.setRoutingInFlight(true);
    const assignments = link("assignments");
    click(assignments);
    const replacement = busyIds();
    harness.selectTab("gradebook");
    harness.setRoutingInFlight(false);
    await flushMicrotasks();
    const redirected = busyIds();

    harness.setRoutingInFlight(true);
    click(link("teachingOperations"), { ctrlKey: true });
    click(link("teachingOperations"), { button: 1 });
    const prevented = link("teachingOperations");
    prevented.addEventListener("click", (event) => event.preventDefault(), { once: true });
    click(prevented);
    const nonPrimaryOrPrevented = busyIds();
    harness.setRoutingInFlight(false);

    window.scrollCalls.length = 0;
    document.querySelector("#root").style.inlineSize = "40rem";
    harness.selectTab("assignments");
    await flushMicrotasks();
    const alreadyVisible = [...window.scrollCalls];
    document.querySelector("#root").style.inlineSize = "6rem";
    harness.selectTab("teachingOperations");
    await flushMicrotasks();
    const clippedReveal = [...window.scrollCalls];
    const clippedBounds = {
      row: document.querySelector('[data-ribbon-row="tabs"]').getBoundingClientRect().toJSON(),
      tab: link("teachingOperations").getBoundingClientRect().toJSON(),
    };
    window.scrollCalls.length = 0;
    harness.selectTab("students");
    harness.selectTab("gradebook");
    await flushMicrotasks();
    const rapidSelection = [...window.scrollCalls];
    window.scrollCalls.length = 0;
    harness.setReducedMotion(true);
    harness.selectTab("teachingOperations");
    await flushMicrotasks();
    const reducedMotion = [...window.scrollCalls];

    return {
      externalBefore,
      externalInFlight,
      pending,
      settled,
      replacement,
      redirected,
      nonPrimaryOrPrevented,
      selectedId: selected()?.getAttribute("data-ribbon-control"),
      alreadyVisible,
      clippedReveal,
      clippedBounds,
      rapidSelection,
      reducedMotion,
    };
  });

  assert.deepEqual(
    evidence.externalBefore,
    [],
    "external navigation starts with no Ribbon pending link",
  );
  assert.deepEqual(
    evidence.externalInFlight,
    [],
    "external in-flight navigation marks no Ribbon link",
  );
  assert.deepEqual(evidence.pending.busyIds, ["students"], "only the activated link is pending");
  assert.equal(evidence.pending.ariaBusy, "true", "activated link exposes aria-busy");
  assert.notEqual(
    evidence.pending.style.backgroundImage,
    "none",
    "pending treatment is visibly painted",
  );
  assert.notEqual(
    evidence.pending.style.boxShadow,
    "none",
    "pending treatment has an inset paint cue",
  );
  assert.deepEqual(
    evidence.pending.after,
    evidence.pending.before,
    "pending feedback changes no visible control geometry",
  );
  assert.deepEqual(evidence.settled, [], "settling clears the recorded destination");
  assert.deepEqual(
    evidence.replacement,
    ["assignments"],
    "new Ribbon activation replaces pending identity",
  );
  assert.deepEqual(evidence.redirected, [], "redirect settlement clears pending identity");
  assert.deepEqual(
    evidence.nonPrimaryOrPrevented,
    [],
    "modified, middle, and already-prevented activations never arm pending feedback",
  );
  assert.deepEqual(evidence.alreadyVisible, [], "an already-visible selected Tab does not scroll");
  assert.deepEqual(
    evidence.clippedReveal,
    [
      {
        id: "teachingOperations",
        options: { behavior: "smooth", block: "nearest", inline: "nearest" },
      },
    ],
    `a newly selected clipped Tab reveals itself: ${JSON.stringify(evidence.clippedBounds)}`,
  );
  assert.deepEqual(evidence.rapidSelection, [
    {
      id: "gradebook",
      options: { behavior: "smooth", block: "nearest", inline: "nearest" },
    },
  ]);
  assert.deepEqual(evidence.reducedMotion.at(-1), {
    id: "teachingOperations",
    options: { behavior: "auto", block: "nearest", inline: "nearest" },
  });
  assert.equal(
    evidence.selectedId,
    "teachingOperations",
    "only Tabs own selected-tab visibility behavior",
  );
  await page.evaluate(() => {
    const students = document.querySelector('[data-ribbon-control="students"]');
    if (!(students instanceof HTMLAnchorElement))
      throw new Error("missing pending-hover evidence link");
    students.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
    window.ribbonM8.setRoutingInFlight(true);
  });
  await page.hover('[data-ribbon-control="students"]');
  const pendingHoverAndFocus = await page.evaluate(() => {
    const students = document.querySelector('[data-ribbon-control="students"]');
    if (!(students instanceof HTMLAnchorElement))
      throw new Error("missing pending-hover evidence link");
    students.focus();
    const style = getComputedStyle(students);
    return {
      ariaBusy: students.getAttribute("aria-busy"),
      focused: document.activeElement === students,
      pending: students.dataset.ribbonPending,
      backgroundImage: style.backgroundImage,
      boxShadow: style.boxShadow,
    };
  });
  assert.equal(pendingHoverAndFocus.ariaBusy, "true");
  assert.equal(pendingHoverAndFocus.focused, true);
  assert.equal(pendingHoverAndFocus.pending, "true");
  assert.notEqual(
    pendingHoverAndFocus.backgroundImage,
    "none",
    "pending paint persists while hovered and focused",
  );
  assert.notEqual(
    pendingHoverAndFocus.boxShadow,
    "none",
    "pending inset cue persists while hovered and focused",
  );
  await page.evaluate(() => window.ribbonM8.setRoutingInFlight(false));
  const disposal = await page.evaluate(async () => {
    const unhandled = [];
    const recordUnhandled = (event) => unhandled.push(String(event.reason));
    window.addEventListener("unhandledrejection", recordUnhandled);
    window.scrollCalls.length = 0;
    document.querySelector("#root").style.inlineSize = "6rem";
    window.ribbonM8.selectTab("teachingOperations");
    window.ribbonM8.dispose();
    await new Promise((resolve) => queueMicrotask(resolve));
    await new Promise((resolve) => queueMicrotask(resolve));
    window.removeEventListener("unhandledrejection", recordUnhandled);
    return { scrollCalls: [...window.scrollCalls], unhandled };
  });
  assert.deepEqual(
    disposal.scrollCalls,
    [],
    "disposing before the queued selected-Tab observation prevents post-unmount scroll",
  );
  assert.deepEqual(
    disposal.unhandled,
    [],
    "disposing before the queued selected-Tab observation causes no unhandled browser error",
  );
  assert.deepEqual(
    pageErrors,
    [],
    "disposing before the queued selected-Tab observation causes no page error",
  );
  process.stdout.write(
    "AppRibbon DOM evidence passed: exact pending ownership and selected-tab reveal.\n",
  );
} finally {
  await browser.close();
}
