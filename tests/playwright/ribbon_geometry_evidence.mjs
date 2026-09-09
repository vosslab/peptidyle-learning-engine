// Standalone computed-geometry evidence for the presentation-only Ribbon.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";
import { chromium } from "playwright";

import { bundledAppRibbonCss, loadAppRibbonForSsr } from "../support/ribbon_component_ssr.ts";
import { M6_RIBBON_FIXTURES } from "../support/ribbon_model_fixtures.ts";
import { RIBBON_RESPONSIVE_PROFILES } from "./ui_corpus_manifest.ts";

const globalCss = readFileSync(new URL("../../src/style.css", import.meta.url), "utf8");
const accessibilityCss = readFileSync(
  new URL("../../src/styles/accessibility.css", import.meta.url),
  "utf8",
);
const componentCss = await bundledAppRibbonCss();
const RealAppRibbon = await loadAppRibbonForSsr();

const fixtureMarkup = Object.entries(M6_RIBBON_FIXTURES)
  .map(([name, model]) => {
    const ribbon = renderToString(() => createComponent(RealAppRibbon, { model }));
    const taskRow = model.taskAreas.length > 0 ? "reserved" : "absent";
    return [
      `<div data-fixture="${name}" data-ribbon-task-row="${taskRow}" class="ple-ribbon-shell-grid">`,
      ribbon,
      '<main class="ribbon-proof-content">Proof content</main>',
      "</div>",
    ].join("");
  })
  .join("");
const proofCss = [
  "html,body{margin:0;inline-size:100%;}",
  "[data-fixture]{margin-block-end:1rem;}",
  ".ribbon-proof-content{min-inline-size:0;min-block-size:2rem;}",
].join("");
const documentMarkup = [
  "<!doctype html><html><head><style>",
  globalCss,
  accessibilityCss,
  componentCss,
  proofCss,
  "</style></head><body>",
  fixtureMarkup,
  "</body></html>",
].join("\n");

function paddingProofMarkup(name, model) {
  const taskRow = model.taskAreas.length > 0 ? "reserved" : "absent";
  const ribbon = renderToString(() => createComponent(RealAppRibbon, { model }));
  return [
    "<!doctype html><html><head><style>",
    globalCss,
    accessibilityCss,
    componentCss,
    "html,body{margin:0;inline-size:100%;}",
    "</style></head><body>",
    `<div class="ple-shell-frame ple-ribbon-shell-grid" data-padding-fixture="${name}" data-ribbon-task-row="${taskRow}">`,
    ribbon,
    '<main class="shell"><section id="main-content"><div data-content-probe>Proof content</div></section></main>',
    "</div></body></html>",
  ].join("\n");
}

function near(actual, expected, message) {
  assert.ok(Math.abs(actual - expected) < 0.25, `${message}: ${actual}px != ${expected}px`);
}

async function measuredAt(page, width, scale) {
  await page.setViewportSize({ width, height: 800 });
  await page.setContent(documentMarkup);
  await page.evaluate((fontScale) => {
    document.documentElement.style.fontSize = `${fontScale}%`;
  }, scale);
  return page.evaluate(() => {
    const tokenSize = (element, name) => {
      const probe = document.createElement("div");
      probe.style.cssText = `position:absolute;visibility:hidden;block-size:var(${name});`;
      element.append(probe);
      const value = probe.getBoundingClientRect().height;
      probe.remove();
      return value;
    };
    const entries = [...document.querySelectorAll("[data-fixture]")].map((fixture) => {
      const shell = fixture;
      const ribbon = fixture.querySelector(".ple-app-ribbon");
      const rows = [...fixture.querySelectorAll("[data-ribbon-row]")];
      if (ribbon === null) throw new Error("fixture is missing the Ribbon.");
      return {
        name: fixture.getAttribute("data-fixture"),
        ribbon: ribbon.getBoundingClientRect().height,
        rows: rows.map((row) => row.getBoundingClientRect().height),
        taskRow: ribbon.getAttribute("data-ribbon-task-row"),
        tokens: Object.fromEntries(
          [
            "--ple-ribbon-context-block-size",
            "--ple-ribbon-tab-block-size",
            "--ple-ribbon-reserved-task-size",
            "--ple-ribbon-block-size",
          ].map((name) => [name, tokenSize(ribbon, name)]),
        ),
        shellFirstTrack: Number.parseFloat(getComputedStyle(shell).gridTemplateRows),
      };
    });
    return {
      entries,
      documentOverflow: document.documentElement.scrollWidth > window.innerWidth,
    };
  });
}

async function shellPaddingMeasuredAt(page, viewport, name, model) {
  await page.setViewportSize(viewport);
  await page.setContent(paddingProofMarkup(name, model));
  return page.evaluate(() => {
    const frame = document.querySelector("[data-padding-fixture]");
    const ribbon = document.querySelector(".ple-app-ribbon");
    const mainContent = document.querySelector("#main-content");
    const contentProbe = document.querySelector("[data-content-probe]");
    if (
      !(frame instanceof HTMLElement) ||
      !(ribbon instanceof HTMLElement) ||
      !(mainContent instanceof HTMLElement) ||
      !(contentProbe instanceof HTMLElement)
    ) {
      throw new Error("shell-padding fixture is incomplete");
    }
    const tokenSize = (name) => {
      const probe = document.createElement("div");
      probe.style.cssText = `position:absolute;visibility:hidden;block-size:var(${name});`;
      ribbon.append(probe);
      const value = probe.getBoundingClientRect().height;
      probe.remove();
      return value;
    };
    return {
      paddingBlockStart: Number.parseFloat(getComputedStyle(mainContent).paddingBlockStart),
      chromeAboveContent:
        contentProbe.getBoundingClientRect().top - frame.getBoundingClientRect().top,
      reservedRows:
        tokenSize("--ple-ribbon-context-block-size") +
        tokenSize("--ple-ribbon-tab-block-size") +
        tokenSize("--ple-ribbon-reserved-task-size"),
    };
  });
}

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  for (const profile of [
    { width: 900, scale: 100 },
    { width: 320, scale: 100 },
    { width: 320, scale: 200 },
  ]) {
    const result = await measuredAt(page, profile.width, profile.scale);
    for (const entry of result.entries) {
      const context = entry.tokens["--ple-ribbon-context-block-size"];
      const tabs = entry.tokens["--ple-ribbon-tab-block-size"];
      const reservedTask = entry.tokens["--ple-ribbon-reserved-task-size"];
      const total = entry.tokens["--ple-ribbon-block-size"];
      const expectedTaskRow = entry.taskRow === "reserved";
      assert.ok(context > 0 && tabs > 0 && total > 0, `${entry.name} resolves named row tokens`);
      assert.equal(
        expectedTaskRow ? reservedTask > 0 : reservedTask === 0,
        true,
        `${entry.name} reserves the task token only for declared topology`,
      );
      near(total, context + tabs + reservedTask, `${entry.name} total reserved-row token`);
      assert.equal(
        entry.rows.length,
        expectedTaskRow ? 3 : 2,
        `${entry.name} renders declared rows`,
      );
      near(entry.rows[0], context, `${entry.name} context row token`);
      near(entry.rows[1], tabs, `${entry.name} tab row token`);
      if (expectedTaskRow) near(entry.rows[2], reservedTask, `${entry.name} task row token`);
      near(entry.ribbon, total, `${entry.name} Ribbon block token`);
      near(entry.shellFirstTrack, total, `${entry.name} shell first grid track`);
    }
    assert.equal(
      result.documentOverflow,
      false,
      `${profile.width}/${profile.scale} has no document overflow`,
    );
  }

  for (const profile of RIBBON_RESPONSIVE_PROFILES) {
    const viewport = profile.contextOptions.viewport;
    assert.ok(viewport, `${profile.id} declares a viewport`);
    for (const [name, model] of [
      ["taskful", M6_RIBBON_FIXTURES.courseInstructor],
      ["taskless", M6_RIBBON_FIXTURES.courseStudent],
    ]) {
      const result = await shellPaddingMeasuredAt(page, viewport, name, model);
      const expectedPadding = Math.min(12, Math.max(8, viewport.width * 0.009));
      near(result.paddingBlockStart, expectedPadding, `${profile.id}:${name} shell padding token`);
      near(
        result.chromeAboveContent,
        result.reservedRows + result.paddingBlockStart,
        `${profile.id}:${name} chrome above content derives from reserved rows plus shell padding`,
      );
    }
  }

  await page.setViewportSize({ width: 320, height: 800 });
  await page.setContent(documentMarkup);
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "200%";
  });
  const destinationEvidence = await page.evaluate(() => {
    const visibleControls = [...document.querySelectorAll("a, button")];
    return visibleControls.map((control) => {
      const row = control.closest("[data-ribbon-row]");
      if (row === null) return { control: control.textContent, passes: false, reason: "no row" };
      control.focus();
      const controlBox = control.getBoundingClientRect();
      const rowBox = row.getBoundingClientRect();
      const style = getComputedStyle(control);
      return {
        control: control.textContent,
        passes:
          document.activeElement === control &&
          controlBox.width > 0 &&
          controlBox.height > 0 &&
          controlBox.right > rowBox.left &&
          controlBox.left < rowBox.right &&
          controlBox.top >= rowBox.top - 0.25 &&
          controlBox.bottom <= rowBox.bottom + 0.25 &&
          style.overflowX === "visible" &&
          style.overflowY === "visible",
        box: [controlBox.left, controlBox.right, controlBox.top, controlBox.bottom],
        row: [rowBox.left, rowBox.right, rowBox.top, rowBox.bottom],
        overflow: [style.overflowX, style.overflowY],
      };
    });
  });
  assert.equal(
    destinationEvidence.every((evidence) => evidence.passes),
    true,
    "every visible destination focuses within its own row scrollport: " +
      JSON.stringify(destinationEvidence),
  );
  process.stdout.write(
    "Ribbon geometry evidence: production CSS, row tokens, shell grid, responsive shell padding, " +
      "and 320px/200% focus reachability passed.\n",
  );
} finally {
  await browser.close();
}
