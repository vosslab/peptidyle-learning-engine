// Compiled-DOM responsive evidence: overflow affordances and touch reachability.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { RIBBON_RESPONSIVE_PROFILES } from "./ui_corpus_manifest.ts";
import { bundleRibbonM9ResponsiveHarness } from "../support/ribbon_m9_responsive_loader.ts";

const globalCss = readFileSync(new URL("../../src/style.css", import.meta.url), "utf8");
const accessibilityCss = readFileSync(
  new URL("../../src/styles/accessibility.css", import.meta.url),
  "utf8",
);
const bundle = await bundleRibbonM9ResponsiveHarness();
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
  `import { mountRibbonM9ResponsiveHarness } from "${bundleUrl}";`,
  'window.ribbonM9 = mountRibbonM9ResponsiveHarness(document.querySelector("#root"));',
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
    if (rows.length !== 3) throw new Error(`expected three Ribbon rows, found ${rows.length}`);
    const frames = [...ribbon.querySelectorAll(":scope > [data-ribbon-row-frame]")];
    if (frames.length !== 3)
      throw new Error(`expected three direct cue frames, found ${frames.length}`);
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
      const selected = row.querySelector('[aria-current="page"]');
      const selectedRect =
        selected instanceof HTMLElement ? selected.getBoundingClientRect() : undefined;
      const activeCueRects = activeCues.map((cue) => ({
        edge: cue.getAttribute("data-ribbon-overflow-cue"),
        rect: cue.getBoundingClientRect(),
      }));
      const controls = [...row.querySelectorAll("a,button")].map((control) => {
        const rect = control.getBoundingClientRect();
        return { height: rect.height, width: rect.width, label: control.textContent?.trim() };
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
        selectedRect,
        activeCueRects,
        frameContainsCue: cues.every((cue) => cue.parentElement === frame),
        visibleWidth: row.clientWidth,
        whiteSpace: getComputedStyle(row).whiteSpace,
      };
    });
    const computedBlockSize = getComputedStyle(ribbon).blockSize;
    return {
      computedBlockSize,
      documentWidth,
      innerWidth: window.innerWidth,
      profileId: currentProfileId,
      ribbonHeight: ribbon.getBoundingClientRect().height,
      rowEvidence,
      viewportWidth,
    };
  }, profileId);
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
  assert.equal(evidence.rowEvidence.length, 3, `${profile}: exactly three permanent rows`);
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
    if ((row.id === "tabs" || row.id === "tasks") && row.selectedRect !== undefined) {
      assert.equal(
        row.selectedRect.left >= row.rowLeft && row.selectedRect.right <= row.rowRight,
        true,
        `${profile}: selected ${row.id === "tabs" ? "Tab" : "Task"}` +
          " is fully visible after automatic reveal",
      );
      for (const cue of row.activeCueRects) {
        const cueRect = cue.rect;
        const overlapsSelected =
          row.selectedRect.left < cueRect.right && row.selectedRect.right > cueRect.left;
        assert.equal(
          overlapsSelected,
          false,
          `${profile}: active clipping paint clears the selected ` +
            `${row.id === "tabs" ? "Tab" : "Task"} ` +
            `(${JSON.stringify({
              selected: row.selectedRect,
              cue: cueRect,
              scrollLeft: row.scrollLeft,
            })})`,
        );
      }
    }
  }
  assert.equal(
    Math.abs(Number.parseFloat(evidence.computedBlockSize) - evidence.ribbonHeight) < 0.01,
    true,
    `${profile}: rendered Ribbon equals its computed block-size token`,
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
            ? [...frame.querySelectorAll("[data-ribbon-overflow-cue]")]
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

async function assertContextCueBlend(page, profile) {
  const samples = await page.evaluate(async () => {
    const frame = document.querySelector('[data-ribbon-row-frame="context"]');
    const row = document.querySelector('[data-ribbon-row="context"]');
    if (!(frame instanceof HTMLElement) || !(row instanceof HTMLElement)) {
      throw new Error("missing Context cue frame");
    }
    const maximum = row.scrollWidth - row.clientWidth;
    if (maximum <= 0.5) throw new Error("narrow Context regression needs real overflow");
    const rowStyle = getComputedStyle(row);
    const readActiveCue = () => {
      const cue = [...frame.querySelectorAll("[data-ribbon-overflow-cue]")].find(
        (candidate) => candidate.getAttribute("data-ribbon-overflow-active") === "true",
      );
      if (!(cue instanceof HTMLElement)) throw new Error("Context overflow needs an active cue");
      return {
        backgroundImage: getComputedStyle(cue).backgroundImage,
        edge: cue.dataset.ribbonOverflowCue,
      };
    };

    row.scrollLeft = 0;
    await new Promise((resolve) => requestAnimationFrame(() => resolve()));
    const start = readActiveCue();
    row.scrollLeft = maximum;
    await new Promise((resolve) => requestAnimationFrame(() => resolve()));
    const end = readActiveCue();
    return {
      rowBackground: rowStyle.backgroundColor,
      start,
      end,
    };
  });
  assert.notEqual(
    samples.rowBackground,
    "rgb(255, 255, 255)",
    `${profile}: Context row retains its tinted surface`,
  );
  assert.equal(samples.start.edge, "end", `${profile}: Context start activates its end cue`);
  assert.equal(samples.end.edge, "start", `${profile}: Context end activates its start cue`);
  for (const sample of [samples.start, samples.end]) {
    assert.match(
      sample.backgroundImage,
      /color\(srgb|rgb\(/,
      `${profile}: ${sample.edge} Context cue resolves a painted surface`,
    );
    assert.doesNotMatch(
      sample.backgroundImage,
      /rgb\(255, 255, 255\)/,
      `${profile}: ${sample.edge} Context cue does not fall back to the white card fade`,
    );
  }
}

async function assertEveryTabReachable(page, profile) {
  const reachable = await page.evaluate(() => {
    const row = document.querySelector('[data-ribbon-row="tabs"]');
    if (!(row instanceof HTMLElement)) throw new Error("missing Tab row");
    const tabs = [...row.querySelectorAll("a")];
    return tabs.map((tab) => {
      if (!(tab instanceof HTMLElement)) throw new Error("invalid Tab link");
      tab.scrollIntoView({ block: "nearest", inline: "nearest" });
      const tabRect = tab.getBoundingClientRect();
      const rowRect = row.getBoundingClientRect();
      return {
        id: tab.dataset.ribbonControl,
        reachable: tabRect.left >= rowRect.left && tabRect.right <= rowRect.right,
      };
    });
  });
  for (const tab of reachable) {
    assert.equal(tab.reachable, true, `${profile}: ${tab.id} is reachable by horizontal scrolling`);
  }
}

async function restoreSelectedTabVisibility(page) {
  const selectedId = await page.evaluate(() => {
    const selected = document.querySelector('[data-ribbon-row="tabs"] [aria-current="page"]');
    return selected instanceof HTMLElement ? selected.dataset.ribbonControl : undefined;
  });
  assert.ok(selectedId, "responsive evidence requires a selected Tab to restore");
  await page.evaluate((currentSelectedId) => {
    const alternatives = [...document.querySelectorAll('[data-ribbon-row="tabs"] a')];
    const alternative = alternatives.find(
      (tab) => tab instanceof HTMLElement && tab.dataset.ribbonControl !== currentSelectedId,
    );
    if (!(alternative instanceof HTMLElement) || alternative.dataset.ribbonControl === undefined) {
      throw new Error("responsive evidence needs a second Tab to exercise automatic reveal");
    }
    window.ribbonM9.selectTab(alternative.dataset.ribbonControl);
  }, selectedId);
  await flush(page);
  await page.evaluate(
    (currentSelectedId) => window.ribbonM9.selectTab(currentSelectedId),
    selectedId,
  );
  await flush(page);
}

async function restoreSelectedTaskVisibility(page) {
  const selectedId = await page.evaluate(() => {
    const selected = document.querySelector('[data-ribbon-row="tasks"] [aria-current="page"]');
    return selected instanceof HTMLElement ? selected.dataset.ribbonControl : undefined;
  });
  if (selectedId === undefined) return;
  await page.evaluate((currentSelectedId) => {
    const alternatives = [...document.querySelectorAll('[data-ribbon-row="tasks"] a')];
    const alternative = alternatives.find(
      (task) => task instanceof HTMLElement && task.dataset.ribbonControl !== currentSelectedId,
    );
    if (!(alternative instanceof HTMLElement) || alternative.dataset.ribbonControl === undefined)
      return;
    window.ribbonM9.selectTask(alternative.dataset.ribbonControl);
  }, selectedId);
  await flush(page);
  await page.evaluate(
    (currentSelectedId) => window.ribbonM9.selectTask(currentSelectedId),
    selectedId,
  );
  await flush(page);
}

const browser = await chromium.launch({ headless: true });
try {
  // The required corpus profiles intentionally use forced colors. Exercise the
  // surface blend separately in an ordinary, true-320px browser context: a
  // forced-colors context correctly replaces author paints with system colors.
  const contextCueContext = await browser.newContext({
    reducedMotion: "reduce",
    viewport: { width: 320, height: 640 },
  });
  const contextCuePage = await contextCueContext.newPage();
  await contextCuePage.setContent(markup);
  await contextCuePage.waitForFunction(() => "ribbonM9" in window);
  await flush(contextCuePage);
  await assertContextCueBlend(contextCuePage, "narrow_phone:author-colors");
  await contextCueContext.close();

  for (const profile of RIBBON_RESPONSIVE_PROFILES) {
    const context = await browser.newContext(profile.contextOptions);
    const page = await context.newPage();
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));
    await page.setContent(markup);
    await page.waitForFunction(() => "ribbonM9" in window);
    await flush(page);

    if (profile.id === "narrow_phone") {
      await page.evaluate(() => window.ribbonM9.selectTab("teachingOperations"));
      await flush(page);
    }
    const baseline = await ribbonEvidence(page, profile.id);
    const declaredWidth = profile.contextOptions.viewport?.width;
    assert.ok(declaredWidth, `${profile.id}: manifest declares a CSS viewport width`);
    assertResponsiveRows(baseline, profile.id, declaredWidth);
    await assertPinnedOverflowCues(page, profile.id);
    await assertEveryTabReachable(page, profile.id);
    await restoreSelectedTabVisibility(page);
    await restoreSelectedTaskVisibility(page);

    await page.evaluate(() => window.ribbonM9.setFixture("longCourse"));
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

    if (profile.id !== "instructor_desktop") {
      const touchControls = longTitle.rowEvidence.flatMap((row) => row.controls);
      for (const control of touchControls) {
        assert.equal(
          control.height >= 44,
          true,
          `${profile.id}: ${control.label} meets the 44 CSS px primary target minimum`,
        );
      }
    }

    if (profile.id === "narrow_phone") {
      await page.evaluate(() => {
        document.documentElement.style.fontSize = "200%";
      });
      await flush(page);
      const enlargedText = await ribbonEvidence(page, "narrow_phone:200-percent-text");
      assertResponsiveRows(enlargedText, "narrow_phone:200-percent-text", declaredWidth);
      await assertEveryTabReachable(page, "narrow_phone:200-percent-text");
      await restoreSelectedTaskVisibility(page);
    }

    const disposal = await page.evaluate(async () => {
      const unhandled = [];
      const recordUnhandled = (event) => unhandled.push(String(event.reason));
      window.addEventListener("unhandledrejection", recordUnhandled);
      window.ribbonM9.setFixture("longCourse");
      window.ribbonM9.dispose();
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
  process.stdout.write("Ribbon responsive evidence: PASS\n");
} finally {
  await browser.close();
}
