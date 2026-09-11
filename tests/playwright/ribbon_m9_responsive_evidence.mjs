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
    const taskRow = ribbon.dataset.ribbonTaskRow;
    const expectedRows = taskRow === "reserved" ? 2 : 1;
    if (rows.length !== expectedRows)
      throw new Error(`expected ${expectedRows} Ribbon rows, found ${rows.length}`);
    const frames = [...ribbon.querySelectorAll(":scope > [data-ribbon-row-frame]")];
    if (frames.length !== expectedRows)
      throw new Error(`expected ${expectedRows} direct cue frames, found ${frames.length}`);
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
        selectedRect,
        activeCueRects,
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
      taskRow,
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

async function assertBrandProjection(page, profileId) {
  const accessibleBrand = page.getByRole("link", { name: "Peptidyle home", exact: true });
  assert.equal(await accessibleBrand.count(), 1, `${profileId}: brand remains one named home link`);
  const projection = await accessibleBrand.evaluate((brand) => {
    const word = brand.querySelector(".ple-app-ribbon__brand-word");
    if (!(word instanceof HTMLElement)) throw new Error("brand word is missing");
    const style = getComputedStyle(word);
    const bounds = word.getBoundingClientRect();
    return {
      brandHeight: brand.getBoundingClientRect().height,
      clipPath: style.clipPath,
      display: style.display,
      height: bounds.height,
      visibility: style.visibility,
      width: bounds.width,
    };
  });
  assert.ok(projection.width > 1, `${profileId}: brand word remains visibly readable`);
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
  const reservesTaskRow = evidence.taskRow === "reserved";
  assert.equal(
    evidence.rowEvidence.length,
    reservesTaskRow ? 2 : 1,
    `${profile}: renders only declared Ribbon rows`,
  );
  assert.equal(
    reservesTaskRow
      ? evidence.tokens["--ple-ribbon-reserved-task-size"] > 0
      : evidence.tokens["--ple-ribbon-reserved-task-size"] === 0,
    true,
    `${profile}: reserved task token follows route topology`,
  );
  assert.equal(
    Math.abs(
      evidence.tokens["--ple-ribbon-block-size"] -
        (evidence.tokens["--ple-ribbon-top-block-size"] +
          evidence.tokens["--ple-ribbon-reserved-task-size"]),
    ) < 0.25,
    true,
    `${profile}: Ribbon token is the sum of currently reserved rows`,
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
    if ((row.id === "top" || row.id === "tasks") && row.selectedRect !== undefined) {
      assert.equal(
        row.selectedRect.left >= row.rowLeft && row.selectedRect.right <= row.rowRight,
        true,
        `${profile}: selected ${row.id === "top" ? "Tab" : "Task"}` +
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
            `${row.id === "top" ? "Tab" : "Task"} ` +
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
  const signOut = topRow.controls.find((control) => control.label === "Sign out");
  assert.ok(signOut, "instructor_desktop: canonical top bar includes Sign out");
  assert.equal(
    signOut.right <= topRow.rowRight,
    true,
    "instructor_desktop: Sign out remains clear of the top-bar end boundary",
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
    const frame = document.querySelector('[data-ribbon-row-frame="top"]');
    const row = document.querySelector('[data-ribbon-row="top"]');
    if (!(frame instanceof HTMLElement) || !(row instanceof HTMLElement)) {
      throw new Error("missing top-bar cue frame");
    }
    const maximum = row.scrollWidth - row.clientWidth;
    if (maximum <= 0.5) throw new Error("narrow top-bar regression needs real overflow");
    const rowStyle = getComputedStyle(row);
    const readActiveCue = () => {
      const cue = [...frame.querySelectorAll("[data-ribbon-overflow-cue]")].find(
        (candidate) => candidate.getAttribute("data-ribbon-overflow-active") === "true",
      );
      if (!(cue instanceof HTMLElement)) throw new Error("top-bar overflow needs an active cue");
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
    `${profile}: top bar retains its tinted surface`,
  );
  assert.equal(samples.start.edge, "end", `${profile}: top-bar start activates its end cue`);
  assert.equal(samples.end.edge, "start", `${profile}: top-bar end activates its start cue`);
  for (const sample of [samples.start, samples.end]) {
    assert.match(
      sample.backgroundImage,
      /color\(srgb|rgb\(/,
      `${profile}: ${sample.edge} top-bar cue resolves a painted surface`,
    );
    assert.doesNotMatch(
      sample.backgroundImage,
      /rgb\(255, 255, 255\)/,
      `${profile}: ${sample.edge} top-bar cue does not fall back to the white card fade`,
    );
  }
}

async function assertEveryTabReachable(page, profile) {
  const reachable = await page.evaluate(() => {
    const row = document.querySelector('[data-ribbon-row="top"]');
    if (!(row instanceof HTMLElement)) throw new Error("missing top bar");
    const tabs = [...row.querySelectorAll(".ple-app-ribbon__tabs a")];
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
    const selected = document.querySelector(
      '[data-ribbon-row="top"] .ple-app-ribbon__tabs [aria-current="page"]',
    );
    return selected instanceof HTMLElement ? selected.dataset.ribbonControl : undefined;
  });
  assert.ok(selectedId, "responsive evidence requires a selected Tab to restore");
  await page.evaluate((currentSelectedId) => {
    const alternatives = [
      ...document.querySelectorAll('[data-ribbon-row="top"] .ple-app-ribbon__tabs a'),
    ];
    const alternative = alternatives.find(
      (tab) => tab instanceof HTMLElement && tab.dataset.ribbonControl !== currentSelectedId,
    );
    if (!(alternative instanceof HTMLElement) || alternative.dataset.ribbonControl === undefined)
      return;
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

async function assertLateSelectedTaskAutoReveal(page, profile) {
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "200%";
    window.ribbonM9.setFixture("productInstructor");
    window.ribbonM9.selectTask("searchQuestionLibrary");
  });
  await flush(page);

  const evidence = await page.evaluate(() => {
    const row = document.querySelector('[data-ribbon-row="tasks"]');
    const selected = row?.querySelector('[aria-current="page"]');
    if (!(row instanceof HTMLElement) || !(selected instanceof HTMLElement)) {
      throw new Error("late selected task needs a task scrollport and selected control");
    }
    const rowRect = row.getBoundingClientRect();
    const selectedRect = selected.getBoundingClientRect();
    const style = getComputedStyle(row);
    const startInset = Number.parseFloat(style.scrollPaddingInlineStart) || 0;
    const endInset = Number.parseFloat(style.scrollPaddingInlineEnd) || 0;
    const cues = [...(row.parentElement?.querySelectorAll("[data-ribbon-overflow-cue]") ?? [])]
      .filter((cue) => cue.getAttribute("data-ribbon-overflow-active") === "true")
      .map((cue) => cue.getBoundingClientRect());
    return {
      cueSafeLeft: rowRect.left + startInset,
      cueSafeRight: rowRect.right - endInset,
      cues,
      id: selected.dataset.ribbonControl,
      selectedRect,
    };
  });
  assert.equal(
    evidence.id,
    "searchQuestionLibrary",
    `${profile}: fixture route selects the late Question task`,
  );
  assert.equal(
    evidence.selectedRect.left >= evidence.cueSafeLeft - 0.25 &&
      evidence.selectedRect.right <= evidence.cueSafeRight + 0.25,
    true,
    `${profile}: 320px/200% late selected Task is fully inside the cue-safe scrollport: ` +
      JSON.stringify(evidence),
  );
  for (const cue of evidence.cues) {
    assert.equal(
      evidence.selectedRect.right <= cue.left || evidence.selectedRect.left >= cue.right,
      true,
      `${profile}: active clipping paint clears the late selected Task`,
    );
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
    if (profile.id === "instructor_desktop") {
      assertCanonicalDesktopTopBar(baseline);
    }
    await assertBrandProjection(page, profile.id);
    await assertPinnedOverflowCues(page, profile.id);
    await assertForcedColorsAffordances(page, profile.id);
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
      await assertLateSelectedTaskAutoReveal(page, "narrow_phone:200-percent-text");
    }

    await page.evaluate(() => window.ribbonM9.setFixture("courseStudent"));
    await flush(page);
    const taskless = await ribbonEvidence(page, `${profile.id}:taskless`);
    assert.equal(taskless.taskRow, "absent", `${profile.id}: taskless route declares no Task Row`);
    assertResponsiveRows(taskless, `${profile.id}:taskless`, declaredWidth);
    await assertPinnedOverflowCues(page, `${profile.id}:taskless`);
    await assertEveryTabReachable(page, `${profile.id}:taskless`);

    await page.evaluate(() => window.ribbonM9.setFixture("courseInstructor"));
    await flush(page);
    const taskfulAgain = await ribbonEvidence(page, `${profile.id}:taskful-again`);
    assert.equal(
      taskfulAgain.taskRow,
      "reserved",
      `${profile.id}: taskless-to-taskful model revision restores the declared Task Row`,
    );
    assertResponsiveRows(taskfulAgain, `${profile.id}:taskful-again`, declaredWidth);
    await restoreSelectedTaskVisibility(page);

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
