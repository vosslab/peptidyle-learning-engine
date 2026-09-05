// Static browser evidence for the Ribbon design laboratory.

import assert from "node:assert/strict";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";
import { chromium } from "playwright";

import {
  RIBBON_DESIGN_SCHEMAS,
  RIBBON_DESIGN_STATE_SPECIMENS,
  RIBBON_DESIGN_TREATMENTS,
} from "../support/ribbon_design_models.ts";
import { RIBBON_GLYPH_IDS, RIBBON_ICON_ASSET_PATH } from "../../src/ribbon/ribbon_icons.ts";
import {
  bundledRibbonDesignFixtureCss,
  loadRibbonDesignFixtureForSsr,
} from "../support/ribbon_design_fixture_loader.ts";

const { COURSE_THEME_OPTIONS } =
  await import("../../src/features/course_appearance/course_theme_registry.ts");

const globalCss = readFileSync(new URL("../../src/style.css", import.meta.url), "utf8");
const accessibilityCss = readFileSync(
  new URL("../../src/styles/accessibility.css", import.meta.url),
  "utf8",
);
const fixtureCss = await bundledRibbonDesignFixtureCss();
const RibbonDesignFixture = await loadRibbonDesignFixtureForSsr();
const markup = renderToString(() => createComponent(RibbonDesignFixture, {}));
const documentMarkup = [
  "<!doctype html><html><head><style>",
  globalCss,
  accessibilityCss,
  fixtureCss,
  "html,body{margin:0;inline-size:100%;}</style></head><body>",
  markup,
  "</body></html>",
].join("\n");
const ribbonSprite = readFileSync(
  new URL("../../src/ribbon/assets/ribbon-icons.svg", import.meta.url),
  "utf8",
);
const outputDirectory = mkdtempSync(join(tmpdir(), "ple_ribbon_m7_harness_"));

async function inspect(page, profile) {
  await page.setViewportSize({ width: profile.width, height: profile.height });
  await page.goto("https://ple.test/");
  await page.evaluate((fontScale) => {
    document.documentElement.style.fontSize = `${fontScale}%`;
  }, profile.fontScale ?? 100);
  await page.waitForFunction(() =>
    [...document.querySelectorAll("[data-ribbon-glyph-atlas-entry] use")].every((use) => {
      try {
        const box = use.getBBox();
        return box.width > 0 && box.height > 0;
      } catch {
        return false;
      }
    }),
  );
  return page.evaluate(() => {
    const panel = (selector) => [...document.querySelectorAll(selector)];
    const ribbons = panel("[data-ribbon-design-panel] .ple-app-ribbon");
    const rows = panel("[data-ribbon-design-panel] [data-ribbon-row]");
    const textFailures = panel("[data-ribbon-design-panel]").filter((item) => {
      const heading = item.querySelector("h3");
      return heading === null || heading.textContent?.trim() === "";
    });
    const treatments = panel("[data-ribbon-treatment]").map((treatment) => ({
      name: treatment.getAttribute("data-ribbon-treatment"),
      schemas: [...treatment.querySelectorAll('[data-ribbon-design-panel="schema"]')].map((item) =>
        item.getAttribute("data-ribbon-design-name"),
      ),
      specimens: [...treatment.querySelectorAll('[data-ribbon-design-panel="specimen"]')].map(
        (item) => item.getAttribute("data-ribbon-design-name"),
      ),
      themes: [...treatment.querySelectorAll('[data-ribbon-design-panel="theme"]')].map((item) =>
        item.getAttribute("data-ribbon-design-name"),
      ),
    }));
    const canonicalInstructorPanels = panel(
      '[data-ribbon-design-panel="schema"][data-ribbon-design-name="courseInstructor"]',
    ).map((panel) => {
      const ribbon = panel.querySelector(".ple-app-ribbon");
      if (ribbon === null) return { width: 0, controlsVisible: false };
      const controlRows = [...ribbon.querySelectorAll("[data-ribbon-row]")];
      const controlsVisible = controlRows.every((row) => {
        const rowBox = row.getBoundingClientRect();
        return [...row.querySelectorAll("a, button")].every((control) => {
          const box = control.getBoundingClientRect();
          return box.left >= rowBox.left - 0.25 && box.right <= rowBox.right + 0.25;
        });
      });
      return { width: ribbon.getBoundingClientRect().width, controlsVisible };
    });
    const overflowCandidates = [...document.querySelectorAll("body *")]
      .filter((element) => {
        const box = element.getBoundingClientRect();
        const row = element.closest("[data-ribbon-row]");
        return (
          row === null &&
          (box.right > window.innerWidth + 0.25 || element.scrollWidth > element.clientWidth)
        );
      })
      .slice(0, 12)
      .map((element) => ({
        element: element.tagName.toLowerCase(),
        className: element.getAttribute("class"),
        treatment: element
          .closest("[data-ribbon-treatment]")
          ?.getAttribute("data-ribbon-treatment"),
        right: element.getBoundingClientRect().right,
        scrollWidth: element.scrollWidth,
        clientWidth: element.clientWidth,
      }));
    const glyphAtlas = [...document.querySelectorAll("[data-ribbon-glyph-atlas-entry]")].map(
      (entry) => {
        const glyph = entry.getAttribute("data-ribbon-glyph-atlas-entry");
        const icon = entry.querySelector("svg[data-ribbon-glyph]");
        const use = icon?.querySelector("use");
        const box = icon?.getBoundingClientRect();
        let renderedBounds;
        try {
          renderedBounds = use?.getBBox();
        } catch {
          renderedBounds = undefined;
        }
        return {
          glyph,
          href: use?.getAttribute("href"),
          iconCount: entry.querySelectorAll("svg[data-ribbon-glyph]").length,
          paintedBox: box !== undefined && box.width > 0 && box.height > 0,
          renderedGraphics:
            renderedBounds !== undefined && renderedBounds.width > 0 && renderedBounds.height > 0,
          fill: icon === null ? undefined : getComputedStyle(icon).fill,
          text: entry.textContent?.trim() ?? "",
        };
      },
    );
    const renderedControls = [
      ...document.querySelectorAll("[data-ribbon-control], [data-ribbon-action]"),
    ].map((control) => {
      const icon = control.querySelector("svg[data-ribbon-glyph]");
      const use = icon?.querySelector("use");
      const label = control.querySelector(".ple-app-ribbon__control-label");
      const labelStyle = label === null ? undefined : getComputedStyle(label);
      return {
        id:
          control.getAttribute("data-ribbon-control") ?? control.getAttribute("data-ribbon-action"),
        iconCount: control.querySelectorAll("svg[data-ribbon-glyph]").length,
        href: use?.getAttribute("href"),
        text: label?.textContent?.trim() ?? "",
        labelVisible:
          labelStyle === undefined ||
          (labelStyle.position !== "absolute" && labelStyle.clipPath === "none"),
        ariaLabel: control.getAttribute("aria-label"),
        title: control.getAttribute("title"),
      };
    });
    const iconOnlySpecimens = [
      ...document.querySelectorAll("[data-ribbon-icon-only-specimen]"),
    ].map((control) => {
      const label = control.querySelector(".ple-app-ribbon__control-label");
      const style = label === null ? undefined : getComputedStyle(label);
      const icon = control.querySelector("svg[data-ribbon-glyph]");
      const use = icon?.querySelector("use");
      return {
        glyph: control.getAttribute("data-ribbon-icon-only-specimen"),
        href: use?.getAttribute("href"),
        iconCount: control.querySelectorAll("svg[data-ribbon-glyph]").length,
        label: label?.textContent?.trim() ?? "",
        labelVisible:
          style === undefined || (style.position !== "absolute" && style.clipPath === "none"),
        ariaLabel: control.getAttribute("aria-label"),
        title: control.getAttribute("title"),
      };
    });
    return {
      documentOverflow: document.documentElement.scrollWidth > window.innerWidth,
      documentWidths: {
        documentClient: document.documentElement.clientWidth,
        documentScroll: document.documentElement.scrollWidth,
        bodyClient: document.body.clientWidth,
        bodyScroll: document.body.scrollWidth,
      },
      treatments,
      canonicalInstructorPanels,
      overflowCandidates,
      withheld: panel("[data-ribbon-withheld]").map((item) =>
        item.getAttribute("data-ribbon-withheld"),
      ),
      exactThreeRows: ribbons.every((ribbon) => {
        const frames = [...ribbon.querySelectorAll(":scope > [data-ribbon-row-frame]")];
        return (
          frames.length === 3 &&
          frames.every(
            (frame) =>
              frame.querySelectorAll(":scope > [data-ribbon-row]").length === 1 &&
              frame.querySelectorAll(":scope > [data-ribbon-overflow-cue]").length === 2,
          )
        );
      }),
      rowGeometry: rows.every((row) => row.getBoundingClientRect().height > 0),
      visibleControlFailures: rows.flatMap((row) =>
        [...row.querySelectorAll("a, button")].flatMap((control) => {
          const controlBox = control.getBoundingClientRect();
          const rowBox = row.getBoundingClientRect();
          const style = getComputedStyle(control);
          const text = control.textContent?.trim() ?? "";
          const controlIsVisible =
            controlBox.width > 0 &&
            controlBox.height > 0 &&
            controlBox.right > rowBox.left &&
            controlBox.left < rowBox.right;
          if (!controlIsVisible) return [];
          const isPartiallyOutsideHorizontalScrollport =
            controlBox.left < rowBox.left - 0.25 || controlBox.right > rowBox.right + 0.25;
          // A row may intentionally offer horizontal scrolling; only a control
          // fully inside its current scrollport is claimed as visibly readable.
          if (isPartiallyOutsideHorizontalScrollport) return [];
          const readable =
            text.length > 0 &&
            Number.parseFloat(style.fontSize) > 0 &&
            style.visibility !== "hidden" &&
            style.display !== "none" &&
            control.scrollWidth <= control.clientWidth &&
            controlBox.top >= rowBox.top - 0.25 &&
            controlBox.bottom <= rowBox.bottom + 0.25;
          return readable
            ? []
            : [
                {
                  text: text || "unnamed control",
                  treatment: control
                    .closest("[data-ribbon-treatment]")
                    ?.getAttribute("data-ribbon-treatment"),
                  fontSize: style.fontSize,
                  scrollWidth: control.scrollWidth,
                  clientWidth: control.clientWidth,
                  vertical: [controlBox.top, controlBox.bottom, rowBox.top, rowBox.bottom],
                },
              ];
        }),
      ),
      // A selected Task is a stronger promise than an arbitrary offscreen
      // item. Intentionally include partial intersections so an ``Ov``-style
      // endpoint clipping regression cannot be silently skipped.
      selectedTaskVisibilityFailures: rows.flatMap((row) => {
        if (row.getAttribute("data-ribbon-row") !== "tasks") return [];
        const rowBox = row.getBoundingClientRect();
        return [...row.querySelectorAll('a[aria-current="page"]')].flatMap((control) => {
          const box = control.getBoundingClientRect();
          const intersects = box.right > rowBox.left && box.left < rowBox.right;
          const fullyVisible =
            box.left >= rowBox.left - 0.25 &&
            box.right <= rowBox.right + 0.25 &&
            control.scrollWidth <= control.clientWidth;
          return intersects && !fullyVisible
            ? [
                {
                  id: control.getAttribute("data-ribbon-control"),
                  box,
                  fontSize: getComputedStyle(control).fontSize,
                  gridRow: getComputedStyle(control).gridRow,
                  rowBox,
                },
              ]
            : [];
        });
      }),
      horizontalScrollableRows: rows.filter((row) => row.scrollWidth > row.clientWidth).length,
      textFailures: textFailures.length,
      glyphAtlas,
      renderedControls,
      iconOnlySpecimens,
    };
  });
}

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  const requests = [];
  page.on("request", (request) => requests.push(request.url()));
  await page.route("https://ple.test/", (route) =>
    route.fulfill({ body: documentMarkup, contentType: "text/html" }),
  );
  await page.route("https://ple.test/assets/ribbon-icons.svg", (route) =>
    route.fulfill({ body: ribbonSprite, contentType: "image/svg+xml" }),
  );
  const desktop = await inspect(page, { width: 1280, height: 800 });
  assert.equal(
    desktop.documentOverflow,
    false,
    "design laboratory has no desktop document overflow",
  );
  assert.deepEqual(
    desktop.treatments.map((treatment) => treatment.name).sort(),
    [...RIBBON_DESIGN_TREATMENTS].sort(),
  );
  for (const treatment of desktop.treatments) {
    assert.deepEqual(treatment.schemas.sort(), [...Object.keys(RIBBON_DESIGN_SCHEMAS)].sort());
    assert.deepEqual(
      treatment.specimens.sort(),
      [...Object.keys(RIBBON_DESIGN_STATE_SPECIMENS)].sort(),
    );
    assert.deepEqual(
      treatment.themes.sort(),
      COURSE_THEME_OPTIONS.map((option) => option.id).sort(),
    );
  }
  assert.equal(desktop.canonicalInstructorPanels.length, RIBBON_DESIGN_TREATMENTS.length);
  assert.equal(
    desktop.canonicalInstructorPanels.every(
      (panel) => panel.width >= 1200 && panel.controlsVisible,
    ),
    true,
    [
      "the canonical Instructor Course Instance Ribbon is full-width with all",
      "available controls direct",
    ].join(" "),
  );
  assert.ok(desktop.withheld.includes("Unavailable"));
  assert.ok(desktop.withheld.includes("Checking"));
  assert.equal(
    desktop.exactThreeRows,
    true,
    [
      "every real Ribbon has three direct cue frames with one labelled scrollport",
      "and two inert cues each",
    ].join(" "),
  );
  assert.equal(desktop.rowGeometry, true, "each real Ribbon has three measurable permanent rows");
  assert.deepEqual(
    desktop.visibleControlFailures,
    [],
    "visible Ribbon controls keep readable, unclipped labels within their rows",
  );
  assert.equal(desktop.textFailures, 0, "every review panel retains a visible label");
  assert.equal(
    desktop.renderedControls
      .filter((control) => control.iconCount > 0)
      .every(
        (control) =>
          control.iconCount === 1 &&
          control.href?.startsWith(`${RIBBON_ICON_ASSET_PATH}#`) &&
          control.text.length > 0 &&
          control.labelVisible,
      ),
    true,
    "desktop icon-bearing controls have one production sprite glyph and a visible real text label",
  );
  assert.equal(
    desktop.renderedControls
      .filter((control) =>
        ["teachingOperations", "assignmentOverview", "assignmentPolicies"].includes(control.id),
      )
      .every((control) => control.iconCount === 0),
    true,
    "rendered text-only destinations do not receive decorative SVGs",
  );
  assert.equal(
    desktop.iconOnlySpecimens.every(
      (specimen) =>
        specimen.iconCount === 1 &&
        specimen.href === `${RIBBON_ICON_ASSET_PATH}#${specimen.glyph}` &&
        specimen.label === specimen.ariaLabel &&
        specimen.label === specimen.title &&
        specimen.labelVisible,
    ),
    true,
    "desktop keeps every safe visual specimen labelled with an exact tooltip and accessible name",
  );
  assert.deepEqual(
    desktop.glyphAtlas.map((entry) => entry.glyph).sort(),
    [...RIBBON_GLYPH_IDS].sort(),
    "the review laboratory exposes the complete closed glyph vocabulary",
  );
  assert.equal(
    desktop.glyphAtlas.every(
      (entry) =>
        entry.iconCount === 1 &&
        entry.href === `${RIBBON_ICON_ASSET_PATH}#${entry.glyph}` &&
        entry.paintedBox &&
        entry.renderedGraphics &&
        entry.fill !== "none" &&
        entry.text.length > 0,
    ),
    true,
    "each atlas entry has one sized real production sprite use and a semantic label",
  );
  await page.screenshot({ path: join(outputDirectory, "desktop.png"), fullPage: true });

  const tablet = await inspect(page, { width: 768, height: 1024 });
  assert.equal(
    tablet.documentOverflow,
    false,
    "portrait tablet design laboratory has no document overflow",
  );
  assert.deepEqual(
    tablet.glyphAtlas.map((entry) => entry.glyph).sort(),
    [...RIBBON_GLYPH_IDS].sort(),
    "portrait tablet review retains the complete glyph atlas",
  );
  assert.deepEqual(
    tablet.iconOnlySpecimens.map((specimen) => specimen.glyph).sort(),
    ["arrow-left", "eye", "star"],
    "tablet reviews every catalog-safe icon-only specimen before the narrow-phone collapse",
  );
  assert.equal(
    tablet.iconOnlySpecimens.every(
      (specimen) =>
        specimen.iconCount === 1 &&
        specimen.href === `${RIBBON_ICON_ASSET_PATH}#${specimen.glyph}` &&
        specimen.labelVisible &&
        specimen.label === specimen.ariaLabel &&
        specimen.label === specimen.title,
    ),
    true,
    [
      "tablet keeps all safe visual specimens visibly labelled with their exact",
      "accessible names and tooltips",
    ].join(" "),
  );
  const tabletSignOutControls = tablet.renderedControls.filter(
    (control) => control.id === "signOut",
  );
  assert.ok(tabletSignOutControls.length > 0, "tablet renders the real Context Sign out control");
  assert.equal(
    tabletSignOutControls.every(
      (control) =>
        control.iconCount === 1 &&
        control.href === `${RIBBON_ICON_ASSET_PATH}#right-from-bracket` &&
        control.text === "Sign out" &&
        control.labelVisible &&
        control.ariaLabel === "Sign out" &&
        control.title === "Sign out",
    ),
    true,
    "tablet keeps every actual Sign out label, exact name, tooltip, and conventional glyph visible",
  );
  assert.equal(
    tablet.renderedControls
      .filter((control) => control.id !== "signOut" && control.iconCount > 0)
      .every((control) => control.iconCount === 1 && control.labelVisible),
    true,
    "tablet retains ordinary icon-bearing controls as labelled paired controls",
  );
  assert.equal(
    tablet.renderedControls
      .filter((control) =>
        ["teachingOperations", "assignmentOverview", "assignmentPolicies"].includes(control.id),
      )
      .every((control) => control.iconCount === 0 && control.labelVisible),
    true,
    "tablet retains ordinary text-only destinations as visibly labelled controls",
  );
  await page.screenshot({ path: join(outputDirectory, "tablet.png"), fullPage: true });

  const phone = await inspect(page, { width: 320, height: 800 });
  assert.equal(
    phone.documentOverflow,
    false,
    "design laboratory reflows without phone document overflow",
  );
  assert.equal(phone.exactThreeRows, true, "phone profile retains exactly three Ribbon rows");
  assert.equal(phone.rowGeometry, true, "phone profile retains real three-row Ribbon geometry");
  assert.deepEqual(
    phone.visibleControlFailures,
    [],
    "visible phone Ribbon controls keep readable, unclipped labels within their rows",
  );
  assert.deepEqual(
    phone.selectedTaskVisibilityFailures,
    [],
    [
      "phone keeps every intersecting selected Task fully readable rather than",
      "partially painting it beneath an edge",
    ].join(" "),
  );
  const phoneSignOut = phone.renderedControls.find((control) => control.id === "signOut");
  assert.deepEqual(
    {
      ariaLabel: phoneSignOut?.ariaLabel,
      title: phoneSignOut?.title,
      visible: phoneSignOut?.labelVisible,
    },
    { ariaLabel: "Sign out", title: "Sign out", visible: false },
    [
      "narrow-phone Sign out retains its exact name and tooltip while its",
      "conventional glyph takes the compact slot",
    ].join(" "),
  );
  assert.equal(
    phone.renderedControls
      .filter((control) => control.iconCount > 0 && control.id !== "signOut")
      .every((control) => control.labelVisible),
    true,
    "narrow phone does not hide labels for ordinary icon-bearing destinations",
  );
  assert.deepEqual(
    phone.iconOnlySpecimens.map((specimen) => specimen.glyph).sort(),
    ["arrow-left", "eye", "star"],
    "the design fixture honestly reviews all three catalog-safe destination glyphs without routes",
  );
  assert.equal(
    phone.iconOnlySpecimens.every(
      (specimen) =>
        specimen.iconCount === 1 &&
        specimen.href === `${RIBBON_ICON_ASSET_PATH}#${specimen.glyph}` &&
        !specimen.labelVisible &&
        specimen.label === specimen.ariaLabel &&
        specimen.label === specimen.title,
    ),
    true,
    "true 320 phone safely compacts only the clearly labelled non-route visual specimens",
  );
  assert.deepEqual(
    phone.glyphAtlas.map((entry) => entry.glyph).sort(),
    [...RIBBON_GLYPH_IDS].sort(),
    "phone review retains the complete glyph atlas",
  );
  await page.screenshot({ path: join(outputDirectory, "phone.png"), fullPage: true });
  const enlargedPhone = await inspect(page, { width: 320, height: 800, fontScale: 200 });
  assert.equal(
    enlargedPhone.documentOverflow,
    false,
    [
      "200% phone design laboratory reflows without document overflow:",
      JSON.stringify({
        widths: enlargedPhone.documentWidths,
        candidates: enlargedPhone.overflowCandidates,
      }),
    ].join(" "),
  );
  assert.equal(
    enlargedPhone.exactThreeRows,
    true,
    "200% phone profile retains exactly three Ribbon rows",
  );
  assert.equal(
    enlargedPhone.rowGeometry,
    true,
    "200% phone profile retains real three-row Ribbon geometry",
  );
  assert.deepEqual(
    enlargedPhone.visibleControlFailures,
    [],
    "visible 200% phone Ribbon controls keep readable, unclipped labels within their rows",
  );
  assert.deepEqual(
    enlargedPhone.selectedTaskVisibilityFailures,
    [],
    [
      "true 320px/200% text keeps every selected Task fully visible and clear of",
      "the task-row endpoint",
    ].join(" "),
  );
  assert.deepEqual(
    enlargedPhone.glyphAtlas.map((entry) => entry.glyph).sort(),
    [...RIBBON_GLYPH_IDS].sort(),
    "200% phone review retains the complete glyph atlas",
  );
  await page.screenshot({ path: join(outputDirectory, "phone_200_percent.png"), fullPage: true });
  const ribbonSpriteRequests = requests.filter((url) => url.includes("ribbon-icons.svg"));
  assert.ok(
    ribbonSpriteRequests.length > 0,
    "real external uses request the bundled Ribbon sprite",
  );
  assert.equal(
    ribbonSpriteRequests.every((url) => url === "https://ple.test/assets/ribbon-icons.svg"),
    true,
    "Ribbon icons request only the exact same-origin bundled sprite asset",
  );
  assert.equal(
    requests.some((url) =>
      /(?:fontawesome|fortawesome|fonts\.googleapis|fonts\.gstatic)/iu.test(url),
    ),
    false,
    "the Ribbon browser evidence has no CDN or icon-font request",
  );
  process.stdout.write(`Ribbon design fixture evidence passed; screenshots: ${outputDirectory}\n`);
} finally {
  await browser.close();
}
