// Compiled-DOM responsive evidence: overflow affordances and touch reachability.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import {
  RIBBON_RESPONSIVE_PROFILES,
  SYSADMIN_DESKTOP_CONTEXT_OPTIONS,
} from "./ui_corpus_manifest.ts";
import { bundleRibbonResponsiveHarness } from "../support/ribbon_responsive_loader.ts";

const globalCss = [
  readFileSync(new URL("../../src/style.css", import.meta.url), "utf8"),
  readFileSync(new URL("../../src/style_responsive.css", import.meta.url), "utf8"),
].join("\n");
const accessibilityCss = readFileSync(
  new URL("../../src/styles/accessibility.css", import.meta.url),
  "utf8",
);
const bundle = await bundleRibbonResponsiveHarness();
const bundleUrl = `data:text/javascript;base64,${Buffer.from(bundle.javascript).toString(
  "base64",
)}`;
const markup = [
  "<!doctype html><html><head>",
  '<meta name="viewport" content="width=device-width, initial-scale=1.0">',
  `<style>${globalCss}\n${accessibilityCss}\n${bundle.stylesheet}`,
  "\nhtml,body{margin:0;min-inline-size:0}.m9-root{min-inline-size:0}</style>",
  '</head><body><div id="root" class="m9-root"></div>',
  '<script type="module">',
  `import { mountRibbonResponsiveHarness } from "${bundleUrl}";`,
  'window.ribbonResponsive = mountRibbonResponsiveHarness(document.querySelector("#root"));',
  "</script></body></html>",
].join("");

async function flush(page) {
  await page.evaluate(() => new Promise((resolve) => queueMicrotask(resolve)));
  await page.evaluate(() => new Promise((resolve) => queueMicrotask(resolve)));
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => resolve())));
}

async function ribbonEvidence(page, profileId) {
  return page.evaluate((currentProfileId) => {
    const ribbon = document.querySelector(".ple-app-ribbon");
    if (!(ribbon instanceof HTMLElement)) throw new Error("missing compiled AppRibbon");
    const rows = [...document.querySelectorAll("[data-ribbon-row]")];
    if (rows.length !== 2) throw new Error(`expected 2 Ribbon rows, found ${rows.length}`);
    const frames = [...ribbon.querySelectorAll(":scope > [data-ribbon-row-frame]")];
    if (frames.length !== 2)
      throw new Error(`expected 2 direct cue frames, found ${frames.length}`);
    const documentWidth = document.documentElement.scrollWidth;
    const viewportWidth = document.documentElement.clientWidth;
    const rowEvidence = rows.map((row) => {
      if (!(row instanceof HTMLElement)) throw new Error("invalid Ribbon row");
      const rowRect = row.getBoundingClientRect();
      const frame = row.parentElement;
      if (
        !(frame instanceof HTMLElement) ||
        frame.dataset.ribbonRowFrame !== row.dataset.ribbonRow
      ) {
        throw new Error("each labelled scrollport needs its own non-scrolling cue frame");
      }
      if (
        frame.parentElement !== ribbon ||
        frame.querySelectorAll(":scope > [data-ribbon-row]").length !== 1
      ) {
        throw new Error("each direct cue frame needs exactly one direct labelled scrollport");
      }
      const cues = [...frame.querySelectorAll(":scope > [data-ribbon-overflow-cue]")];
      const activeCues = cues.filter(
        (cue) => cue.getAttribute("data-ribbon-overflow-active") === "true",
      );
      const controls = [...row.querySelectorAll("a,button")].map((control) => {
        const rect = control.getBoundingClientRect();
        return {
          height: rect.height,
          label: control.textContent?.trim(),
          left: rect.left,
          right: rect.right,
          width: rect.width,
        };
      });
      return {
        id: row.dataset.ribbonRow,
        activeCueCount: activeCues.length,
        cues: cues.map((cue) => ({
          ariaHidden: cue.getAttribute("aria-hidden"),
          pointerEvents: getComputedStyle(cue).pointerEvents,
        })),
        controls,
        overflows: row.scrollWidth > row.clientWidth,
        rowBottom: rowRect.bottom,
        rowHeight: rowRect.height,
        rowLeft: rowRect.left,
        rowRight: rowRect.right,
        scrollLeft: row.scrollLeft,
        rowTop: rowRect.top,
        scrollHeight: row.scrollHeight,
        scrollWidth: row.scrollWidth,
        frameContainsCue: cues.every((cue) => cue.parentElement === frame),
        visibleWidth: row.clientWidth,
        whiteSpace: getComputedStyle(row).whiteSpace,
      };
    });
    const tokenSize = (name) => {
      const probe = document.createElement("div");
      probe.style.cssText = `position:absolute;visibility:hidden;block-size:var(${name});`;
      ribbon.append(probe);
      const value = probe.getBoundingClientRect().height;
      probe.remove();
      return value;
    };
    const computedBlockSize = getComputedStyle(ribbon).blockSize;
    return {
      computedBlockSize,
      documentWidth,
      innerWidth: window.innerWidth,
      profileId: currentProfileId,
      ribbonHeight: ribbon.getBoundingClientRect().height,
      rowEvidence,
      tokens: Object.fromEntries(
        [
          "--ple-ribbon-top-block-size",
          "--ple-ribbon-reserved-task-size",
          "--ple-ribbon-block-size",
        ].map((name) => [name, tokenSize(name)]),
      ),
      viewportWidth,
    };
  }, profileId);
}

/** The staff-role desktop Ribbon stays inside the declared desktop viewport. */
async function assertSysadminDesktopRibbon(browser) {
  const context = await browser.newContext(SYSADMIN_DESKTOP_CONTEXT_OPTIONS);
  try {
    const page = await context.newPage();
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));
    await page.setContent(markup);
    await page.waitForFunction(() => "ribbonResponsive" in window);
    await page.evaluate(() => window.ribbonResponsive.setRoleHome("sysadmin"));
    await flush(page);

    const evidence = await ribbonEvidence(page, "sysadmin_desktop");
    assertResponsiveRows(evidence, "sysadmin_desktop", 1280);
    const topRow = evidence.rowEvidence.find((row) => row.id === "top");
    assert.ok(topRow, "sysadmin_desktop: Sysadmin Ribbon has a top bar");
    assert.equal(topRow.overflows, false, "sysadmin_desktop: top bar fits without scrolling");
    for (const control of topRow.controls) {
      assert.equal(
        control.left >= topRow.rowLeft && control.right <= topRow.rowRight,
        true,
        `sysadmin_desktop: ${control.label} fits inside the top bar`,
      );
    }
    for (const label of ["Instructor Accounts"]) {
      assert.ok(
        topRow.controls.some((control) => control.label === label),
        `sysadmin_desktop: ${label} destination remains visible`,
      );
    }
    assert.deepEqual(pageErrors, [], "sysadmin_desktop: compiled Ribbon causes no browser error");
  } finally {
    await context.close();
  }
}

function assertResponsiveRows(evidence, profile, expectedWidth) {
  assert.equal(evidence.innerWidth, expectedWidth, `${profile}: true declared CSS viewport width`);
  assert.equal(
    evidence.viewportWidth,
    expectedWidth,
    `${profile}: document client width matches profile`,
  );
  assert.equal(
    evidence.documentWidth <= evidence.viewportWidth,
    true,
    `${profile}: no document overflow`,
  );
  assert.equal(evidence.rowEvidence.length, 2, `${profile}: reserves stable Ribbon rows`);
  assert.equal(
    evidence.tokens["--ple-ribbon-reserved-task-size"] > 0,
    true,
    `${profile}: reserves a stable tier-2 row`,
  );
  assert.equal(
    Math.abs(
      evidence.tokens["--ple-ribbon-block-size"] -
        (evidence.tokens["--ple-ribbon-top-block-size"] +
          evidence.tokens["--ple-ribbon-reserved-task-size"]),
    ) < 0.25,
    true,
    `${profile}: Ribbon token is the sum of stable rows`,
  );
  for (const row of evidence.rowEvidence) {
    assert.equal(row.whiteSpace, "nowrap", `${profile}: row remains one non-wrapping line`);
    assert.equal(
      row.scrollHeight <= row.rowHeight + 1,
      true,
      `${profile}: labels stay readable within the row rather than wrapping or clipping vertically`,
    );
    assert.equal(row.cues.length, 2, `${profile}: row retains start and end clipping-cue elements`);
    assert.equal(
      row.frameContainsCue,
      true,
      `${profile}: cues are paint siblings, not scrolling content`,
    );
    for (const cue of row.cues) {
      assert.equal(cue.ariaHidden, "true", `${profile}: clipping cue is not announced`);
      assert.equal(
        cue.pointerEvents,
        "none",
        `${profile}: clipping cue cannot intercept a control`,
      );
    }
    if (row.overflows) {
      assert.equal(row.activeCueCount > 0, true, `${profile}: overflow visibly activates a cue`);
    }
  }
  assert.equal(
    Math.abs(Number.parseFloat(evidence.computedBlockSize) - evidence.ribbonHeight) < 0.01,
    true,
    `${profile}: rendered Ribbon equals its computed block-size token`,
  );
}

function assertCanonicalDesktopTopBar(evidence) {
  const topRow = evidence.rowEvidence.find((row) => row.id === "top");
  assert.ok(topRow, "instructor_desktop: canonical Ribbon has a top bar");
  assert.equal(
    topRow.overflows,
    false,
    "instructor_desktop: canonical top bar fits without scrolling",
  );
  assert.equal(
    topRow.activeCueCount,
    0,
    "instructor_desktop: canonical top bar has no overflow cue",
  );
  for (const control of topRow.controls) {
    assert.equal(
      control.left >= topRow.rowLeft && control.right <= topRow.rowRight,
      true,
      `instructor_desktop: ${control.label} fits fully inside the canonical top bar`,
    );
  }
  assert.equal(
    topRow.controls.some((control) => control.label === "Sign out"),
    false,
    "instructor_desktop: Sign out is not scattered into the main top bar",
  );
}

async function assertPinnedOverflowCues(page, profile) {
  for (const position of ["start", "middle", "end"]) {
    const sample = await page.evaluate(async (requestedPosition) => {
      const rows = [...document.querySelectorAll("[data-ribbon-row]")];
      for (const row of rows) {
        if (!(row instanceof HTMLElement)) throw new Error("invalid Ribbon row");
        const maximum = row.scrollWidth - row.clientWidth;
        row.scrollLeft =
          requestedPosition === "start"
            ? 0
            : requestedPosition === "middle"
              ? maximum / 2
              : maximum;
      }
      await new Promise((resolve) => requestAnimationFrame(() => resolve()));
      return rows.map((row) => {
        if (!(row instanceof HTMLElement)) throw new Error("invalid Ribbon row");
        const rowRect = row.getBoundingClientRect();
        const frame = row.parentElement;
        const cues =
          frame instanceof HTMLElement
            ? [...frame.querySelectorAll(":scope > [data-ribbon-overflow-cue]")]
            : [];
        return {
          maximum: row.scrollWidth - row.clientWidth,
          rowRect,
          cues: cues.map((cue) => ({
            active: cue.getAttribute("data-ribbon-overflow-active") === "true",
            edge: cue.getAttribute("data-ribbon-overflow-cue"),
            rect: cue.getBoundingClientRect(),
          })),
        };
      });
    }, position);
    for (const row of sample) {
      if (row.maximum <= 0.5) continue;
      const activeEdges = row.cues
        .filter((cue) => cue.active)
        .map((cue) => cue.edge)
        .sort();
      const expectedEdges =
        position === "start" ? ["end"] : position === "middle" ? ["end", "start"] : ["start"];
      assert.deepEqual(
        activeEdges,
        expectedEdges,
        `${profile}:${position}: correct active overflow edge ${JSON.stringify({
          maximum: row.maximum,
          cues: row.cues,
        })}`,
      );
      for (const cue of row.cues.filter((candidate) => candidate.active)) {
        assert.equal(
          cue.rect.width > 1,
          true,
          `${profile}:${position}: ${cue.edge} cue has visible paint`,
        );
        assert.equal(
          cue.rect.left >= row.rowRect.left - 1 && cue.rect.right <= row.rowRect.right + 1,
          true,
          `${profile}:${position}: ${cue.edge} cue stays inside its row viewport`,
        );
        const edgeDistance =
          cue.edge === "start"
            ? Math.abs(cue.rect.left - row.rowRect.left)
            : Math.abs(cue.rect.right - row.rowRect.right);
        assert.equal(
          edgeDistance <= 1,
          true,
          `${profile}:${position}: ${cue.edge} cue is pinned to row edge`,
        );
      }
    }
  }
}

async function assertTierOneKeyboardNavigation(page, profile) {
  const focusable = await page.evaluate(() => {
    const row = document.querySelector('[data-ribbon-row="top"]');
    if (!(row instanceof HTMLElement)) throw new Error("missing top bar");
    const tabs = [...row.querySelectorAll(".ple-app-ribbon__tabs a")];
    return tabs.map((tab) => {
      if (!(tab instanceof HTMLElement)) throw new Error("invalid Tab link");
      tab.focus();
      return {
        id: tab.dataset.ribbonControl,
        focused: document.activeElement === tab,
      };
    });
  });
  for (const tab of focusable) {
    assert.equal(tab.focused, true, `${profile}: ${tab.id} can receive keyboard focus`);
  }
}

async function assertForcedColorsAffordances(page, profile) {
  const evidence = await page.evaluate(async () => {
    if (!matchMedia("(forced-colors: active)").matches) {
      throw new Error("forced-colors Ribbon evidence needs its declared browser context");
    }
    const rows = [...document.querySelectorAll("[data-ribbon-row]")];
    const cues = [];
    for (const row of rows) {
      if (!(row instanceof HTMLElement)) throw new Error("invalid Ribbon row");
      const maximum = row.scrollWidth - row.clientWidth;
      if (maximum <= 0.5) continue;
      for (const position of ["start", "end"]) {
        row.scrollLeft = position === "start" ? 0 : maximum;
        await new Promise((resolve) => requestAnimationFrame(() => resolve()));
        const edge = position === "start" ? "end" : "start";
        const frame = row.parentElement;
        const cue = frame?.querySelector(
          `[data-ribbon-overflow-cue="${edge}"][data-ribbon-overflow-active="true"]`,
        );
        if (!(cue instanceof HTMLElement)) {
          throw new Error(`overflow row needs its ${edge} marker at ${position}`);
        }
        const style = getComputedStyle(cue);
        const marker = getComputedStyle(cue, "::before");
        cues.push({
          backgroundColor: style.backgroundColor,
          borderInlineEndStyle: style.borderInlineEndStyle,
          borderInlineStartStyle: style.borderInlineStartStyle,
          edge,
          forcedColorAdjust: style.forcedColorAdjust,
          marker: marker.content,
        });
      }
    }
    const selected = document.querySelector(
      '[data-ribbon-row="tasks"] .ple-app-ribbon__link[aria-current="page"]',
    );
    if (!(selected instanceof HTMLElement)) throw new Error("missing selected task link");
    const label = selected.querySelector(".ple-app-ribbon__control-label");
    if (!(label instanceof HTMLElement)) throw new Error("selected task label is missing");
    const style = getComputedStyle(selected);
    return {
      cues,
      hasOverflowingRow: rows.some(
        (row) => row instanceof HTMLElement && row.scrollWidth > row.clientWidth,
      ),
      selected: {
        backgroundColor: style.backgroundColor,
        color: style.color,
        forcedColorAdjust: style.forcedColorAdjust,
        labelColor: getComputedStyle(label).color,
        labelWidth: label.getBoundingClientRect().width,
        textDecorationLine: style.textDecorationLine,
      },
    };
  });
  if (evidence.hasOverflowingRow) {
    assert.ok(
      evidence.cues.length > 0,
      `${profile}: an overflowing row exposes a forced-colors cue`,
    );
  }
  for (const cue of evidence.cues) {
    assert.equal(cue.forcedColorAdjust, "none", `${profile}: cue keeps its system-color marker`);
    assert.equal(cue.borderInlineStartStyle, "solid", `${profile}: cue has a start boundary`);
    assert.equal(cue.borderInlineEndStyle, "solid", `${profile}: cue has an end boundary`);
    assert.notEqual(cue.backgroundColor, "rgba(0, 0, 0, 0)", `${profile}: cue has a surface`);
    const expectedMarker = String.fromCodePoint(cue.edge === "end" ? 0x203a : 0x2039);
    assert.equal(
      cue.marker.includes(expectedMarker),
      true,
      `${profile}: ${cue.edge} cue has a directional marker`,
    );
  }
  assert.equal(
    evidence.selected.forcedColorAdjust,
    "none",
    `${profile}: selected task keeps the system-paired paint`,
  );
  assert.equal(
    evidence.selected.labelColor,
    evidence.selected.color,
    `${profile}: selected task label inherits the system foreground`,
  );
  assert.notEqual(
    evidence.selected.color,
    evidence.selected.backgroundColor,
    `${profile}: selected task has distinct readable foreground and background`,
  );
  assert.ok(evidence.selected.labelWidth > 1, `${profile}: selected task label remains visible`);
  assert.match(
    evidence.selected.textDecorationLine,
    /underline/,
    `${profile}: selected task retains non-color identity`,
  );
}

const browser = await chromium.launch({ headless: true });
try {
  for (const profile of RIBBON_RESPONSIVE_PROFILES) {
    const context = await browser.newContext(profile.contextOptions);
    const page = await context.newPage();
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));
    await page.setContent(markup);
    await page.waitForFunction(() => "ribbonResponsive" in window);
    await flush(page);

    const baseline = await ribbonEvidence(page, profile.id);
    const declaredWidth = profile.contextOptions.viewport?.width;
    assert.ok(declaredWidth, `${profile.id}: manifest declares a CSS viewport width`);
    assertResponsiveRows(baseline, profile.id, declaredWidth);
    if (profile.id === "instructor_desktop") {
      assertCanonicalDesktopTopBar(baseline);
    }
    await assertPinnedOverflowCues(page, profile.id);
    await assertForcedColorsAffordances(page, profile.id);
    await assertTierOneKeyboardNavigation(page, profile.id);
    const sameZoomRouteBaseline = baseline;
    await page.evaluate(() => window.ribbonResponsive.setFixture("courseInstructor"));
    await flush(page);

    await page.evaluate(() => window.ribbonResponsive.setFixture("longCourse"));
    await flush(page);
    const longTitle = await ribbonEvidence(page, profile.id);
    assertResponsiveRows(longTitle, `${profile.id}:long-title`, declaredWidth);
    assert.equal(
      longTitle.computedBlockSize,
      baseline.computedBlockSize,
      `${profile.id}: application data does not change the block-size token`,
    );
    assert.equal(
      longTitle.ribbonHeight,
      baseline.ribbonHeight,
      `${profile.id}: application data does not move the content origin`,
    );

    await page.evaluate(() => window.ribbonResponsive.setFixture("courseStudent"));
    await flush(page);
    const student = await ribbonEvidence(page, `${profile.id}:student`);
    assertResponsiveRows(student, `${profile.id}:student`, declaredWidth);
    assert.equal(
      student.computedBlockSize,
      sameZoomRouteBaseline.computedBlockSize,
      `${profile.id}: Student preserves the same-zoom Ribbon block-size token`,
    );
    assert.equal(
      student.ribbonHeight,
      sameZoomRouteBaseline.ribbonHeight,
      `${profile.id}: Student preserves the same-zoom content origin`,
    );
    await assertPinnedOverflowCues(page, `${profile.id}:student`);
    await assertTierOneKeyboardNavigation(page, `${profile.id}:student`);

    await page.evaluate(() => window.ribbonResponsive.setFixture("courseInstructor"));
    await flush(page);
    const instructorAgain = await ribbonEvidence(page, `${profile.id}:instructor-again`);
    assertResponsiveRows(instructorAgain, `${profile.id}:instructor-again`, declaredWidth);

    const disposal = await page.evaluate(async () => {
      const unhandled = [];
      const recordUnhandled = (event) => unhandled.push(String(event.reason));
      window.addEventListener("unhandledrejection", recordUnhandled);
      window.ribbonResponsive.setFixture("longCourse");
      window.ribbonResponsive.dispose();
      await new Promise((resolve) => queueMicrotask(resolve));
      await new Promise((resolve) => requestAnimationFrame(() => resolve()));
      window.removeEventListener("unhandledrejection", recordUnhandled);
      return { remainingRows: document.querySelectorAll("[data-ribbon-row]").length, unhandled };
    });
    assert.equal(
      disposal.remainingRows,
      0,
      `${profile.id}: disposing cleans up the Ribbon's row-local observers with its DOM`,
    );
    assert.deepEqual(
      disposal.unhandled,
      [],
      `${profile.id}: disposal during a content mutation causes no unhandled browser error`,
    );
    assert.deepEqual(pageErrors, [], `${profile.id}: compiled Ribbon causes no browser error`);
    await context.close();
  }
  await assertSysadminDesktopRibbon(browser);
  process.stdout.write("Ribbon responsive evidence: PASS\n");
} finally {
  await browser.close();
}
