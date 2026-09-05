// Standalone computed-geometry evidence for the presentation-only Ribbon.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";
import { chromium } from "playwright";

import { bundledAppRibbonCss, loadAppRibbonForSsr } from "../support/ribbon_component_ssr.ts";
import { M6_RIBBON_FIXTURES } from "../support/ribbon_model_fixtures.ts";

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
    return [
      `<div data-fixture="${name}" class="ple-ribbon-shell-grid">`,
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
    const names = [
      "--ple-ribbon-context-block-size",
      "--ple-ribbon-tab-block-size",
      "--ple-ribbon-task-block-size",
      "--ple-ribbon-block-size",
    ];
    const probes = Object.fromEntries(
      names.map((name) => {
        const probe = document.createElement("div");
        probe.style.cssText = `position:absolute;visibility:hidden;block-size:var(${name});`;
        document.body.append(probe);
        const value = probe.getBoundingClientRect().height;
        probe.remove();
        return [name, value];
      }),
    );
    const entries = [...document.querySelectorAll("[data-fixture]")].map((fixture) => {
      const shell = fixture;
      const ribbon = fixture.querySelector(".ple-app-ribbon");
      const rows = [...fixture.querySelectorAll("[data-ribbon-row]")];
      if (ribbon === null) throw new Error("fixture is missing the Ribbon.");
      return {
        name: fixture.getAttribute("data-fixture"),
        ribbon: ribbon.getBoundingClientRect().height,
        rows: rows.map((row) => row.getBoundingClientRect().height),
        shellFirstTrack: Number.parseFloat(getComputedStyle(shell).gridTemplateRows),
      };
    });
    return {
      probes,
      entries,
      documentOverflow: document.documentElement.scrollWidth > window.innerWidth,
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
    const context = result.probes["--ple-ribbon-context-block-size"];
    const tabs = result.probes["--ple-ribbon-tab-block-size"];
    const tasks = result.probes["--ple-ribbon-task-block-size"];
    const total = result.probes["--ple-ribbon-block-size"];
    assert.ok(context > 0 && tabs > 0 && tasks > 0 && total > 0, "resolved named row tokens");
    near(total, context + tabs + tasks, `${profile.width}/${profile.scale} total row token`);
    for (const entry of result.entries) {
      assert.equal(entry.rows.length, 3, `${entry.name} has exactly three permanent rows`);
      near(entry.rows[0], context, `${entry.name} context row token`);
      near(entry.rows[1], tabs, `${entry.name} tab row token`);
      near(entry.rows[2], tasks, `${entry.name} task row token`);
      near(entry.ribbon, total, `${entry.name} Ribbon block token`);
      near(entry.shellFirstTrack, total, `${entry.name} shell first grid track`);
    }
    assert.equal(
      result.documentOverflow,
      false,
      `${profile.width}/${profile.scale} has no document overflow`,
    );
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
    "Ribbon geometry evidence: production CSS, row tokens, shell grid, " +
      "and 320px/200% focus reachability passed.\n",
  );
} finally {
  await browser.close();
}
