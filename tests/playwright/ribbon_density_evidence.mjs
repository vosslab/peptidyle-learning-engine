// Compiled-DOM evidence for the production Fieldstation visual system.
//
// This intentionally complements the design inventory and responsive mechanics.
// It measures only the visual-system promises that a browser can
// resolve: hierarchy, spacing, theme paint/contrast, and OS preferences.

import assert from "node:assert/strict";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";
import { chromium } from "playwright";
import { assertTierThemeSurfaces } from "./ribbon_tier_theme_evidence.mjs";

const courseThemeRegistry = await import("../../src/appearance/theme_registry.ts");
import {
  bundledRibbonDesignFixtureCss,
  loadRibbonDesignFixtureForSsr,
} from "../support/ribbon_design_fixture_loader.ts";

const globalCss = [
  readFileSync(new URL("../../src/style.css", import.meta.url), "utf8"),
  readFileSync(new URL("../../src/style_responsive.css", import.meta.url), "utf8"),
].join("\n");
const accessibilityCss = readFileSync(
  new URL("../../src/styles/accessibility.css", import.meta.url),
  "utf8",
);
const fixtureCss = await bundledRibbonDesignFixtureCss();
const RibbonDesignFixture = await loadRibbonDesignFixtureForSsr();
const markup = renderToString(() => createComponent(RibbonDesignFixture, {}));
const viewportMarkup = [
  "<!doctype html><html><head>",
  '<meta name="viewport"',
  'content="width=device-width, initial-scale=1.0">',
].join("");
const stylesheetMarkup = [
  "<style>",
  globalCss,
  accessibilityCss,
  fixtureCss,
  "html,body{margin:0;inline-size:100%;}",
  "</style>",
].join("\n");
const documentTheme = courseThemeRegistry.themeStyle(courseThemeRegistry.themeTokens("tundra"));
const documentMarkup = [
  viewportMarkup,
  stylesheetMarkup,
  `</head><body style="${documentTheme}">${markup}</body></html>`,
].join("");
const outputDirectory = mkdtempSync(join(tmpdir(), "ple_ribbon_density_"));

const FIELDSTATION_INSTRUCTOR =
  '[data-ribbon-treatment="fieldstation"] ' +
  '[data-ribbon-design-panel="schema"][data-ribbon-design-name="courseInstructor"]';
const FIELDSTATION_THEMES =
  '[data-ribbon-treatment="fieldstation"] [data-ribbon-design-panel="theme"]';

function channel(value) {
  return Math.max(0, Math.min(255, Number.parseFloat(value)));
}

function parseColor(value) {
  const hex = value.match(/^#([\da-f]{6})$/iu);
  if (hex !== null) {
    return {
      red: Number.parseInt(hex[1].slice(0, 2), 16),
      green: Number.parseInt(hex[1].slice(2, 4), 16),
      blue: Number.parseInt(hex[1].slice(4, 6), 16),
      alpha: 1,
    };
  }
  const rawNumbers = value.match(/[\d.]+/gu)?.map(Number);
  const numbers = rawNumbers?.map((number, index) =>
    value.startsWith("color(srgb") && index < 3 ? channel(number * 255) : channel(number),
  );
  if (numbers === undefined || numbers.length < 3) {
    throw new Error(`expected a resolved rgb/rgba color, received ${value}`);
  }
  return { red: numbers[0], green: numbers[1], blue: numbers[2], alpha: numbers[3] ?? 1 };
}

function composite(foreground, background) {
  const alpha = foreground.alpha + background.alpha * (1 - foreground.alpha);
  if (alpha === 0) return { red: 0, green: 0, blue: 0, alpha: 0 };
  return {
    red:
      (foreground.red * foreground.alpha +
        background.red * background.alpha * (1 - foreground.alpha)) /
      alpha,
    green:
      (foreground.green * foreground.alpha +
        background.green * background.alpha * (1 - foreground.alpha)) /
      alpha,
    blue:
      (foreground.blue * foreground.alpha +
        background.blue * background.alpha * (1 - foreground.alpha)) /
      alpha,
    alpha,
  };
}

function luminance(color) {
  const linear = (component) => {
    const normalized = component / 255;
    return normalized <= 0.04045 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * linear(color.red) + 0.7152 * linear(color.green) + 0.0722 * linear(color.blue);
}

function contrast(foreground, background) {
  const first = luminance(foreground);
  const second = luminance(background);
  return (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05);
}

const browser = await chromium.launch({ headless: true });
try {
  await assertTierThemeSurfaces(browser, documentMarkup, outputDirectory, { parseColor, contrast });
  const normal = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await normal.newPage();
  await page.setContent(documentMarkup);
  await page.waitForFunction(() => document.querySelectorAll(".ple-app-ribbon").length > 0);
  // Capture the production Fieldstation before the local, test-only Task Area
  // clone below. The saved visual evidence must show only canonical fixture UI.
  await page.locator('[data-ribbon-treatment="fieldstation"]').screenshot({
    path: join(outputDirectory, "m9b-fieldstation-1280.png"),
  });

  const evidence = await page.evaluate(
    ({ instructorSelector, themeSelector }) => {
      const rgba = (value) => {
        const rawNumbers = value.match(/[\d.]+/gu)?.map(Number);
        const numbers = rawNumbers?.map((number, index) =>
          value.startsWith("color(srgb") && index < 3 ? number * 255 : number,
        );
        if (numbers === undefined || numbers.length < 3)
          throw new Error(`unresolved paint ${value}`);
        return { red: numbers[0], green: numbers[1], blue: numbers[2], alpha: numbers[3] ?? 1 };
      };
      const backgroundAt = (element) => {
        let background = { red: 255, green: 255, blue: 255, alpha: 1 };
        const ancestors = [];
        for (
          let current = element;
          current instanceof HTMLElement;
          current = current.parentElement
        ) {
          ancestors.push(current);
        }
        for (const current of ancestors.reverse()) {
          const value = getComputedStyle(
            current,
            current.matches('.ple-app-ribbon__tabs [aria-current="page"]') ? "::after" : null,
          ).backgroundColor;
          const color = rgba(value);
          background = {
            red: color.red * color.alpha + background.red * (1 - color.alpha),
            green: color.green * color.alpha + background.green * (1 - color.alpha),
            blue: color.blue * color.alpha + background.blue * (1 - color.alpha),
            alpha: 1,
          };
        }
        return background;
      };
      const box = (element) => {
        const style = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return {
          height: rect.height,
          width: rect.width,
          boxSizing: style.boxSizing,
          borderBlockEndWidth: style.borderBlockEndWidth,
          borderBlockStartWidth: style.borderBlockStartWidth,
          minBlockSize: style.minBlockSize,
          paddingBlockEnd: style.paddingBlockEnd,
          paddingBlockStart: style.paddingBlockStart,
          paddingInlineEnd: style.paddingInlineEnd,
          paddingInlineStart: style.paddingInlineStart,
        };
      };
      const relativeBox = (element, ancestor) => {
        const rect = element.getBoundingClientRect();
        const ancestorRect = ancestor.getBoundingClientRect();
        return {
          left: rect.left - ancestorRect.left,
          top: rect.top - ancestorRect.top,
          width: rect.width,
          height: rect.height,
          right: rect.right - ancestorRect.left,
          bottom: rect.bottom - ancestorRect.top,
        };
      };
      // The canonical Instructor model deliberately has one Task Area. Clone
      // its real rendered area only inside this browser oracle so the
      // production separator/proximity rule is measured without teaching the
      // durable schema inventory an artificial application state.
      const ensureMultipleTaskAreas = (ribbon) => {
        const tasks = ribbon.querySelector(".ple-app-ribbon__tasks");
        const first = tasks?.querySelector(".ple-app-ribbon__task-area");
        if (!(tasks instanceof HTMLElement) || !(first instanceof HTMLElement)) {
          throw new Error("Ribbon lacks a Task Area for density evidence");
        }
        if (tasks.querySelectorAll(".ple-app-ribbon__task-area").length < 2) {
          const duplicate = first.cloneNode(true);
          if (!(duplicate instanceof HTMLElement)) throw new Error("Task Area clone failed");
          duplicate.dataset.ribbonTaskArea = "m9b-density-comparison";
          for (const control of duplicate.querySelectorAll('[aria-current="page"]')) {
            control.removeAttribute("aria-current");
          }
          tasks.append(duplicate);
        }
      };
      const instructor = document.querySelector(instructorSelector);
      if (!(instructor instanceof HTMLElement))
        throw new Error("missing selected Fieldstation Instructor panel");
      const ribbon = instructor.querySelector(".ple-app-ribbon");
      if (!(ribbon instanceof HTMLElement)) throw new Error("missing selected production Ribbon");
      ensureMultipleTaskAreas(ribbon);
      const links = [...ribbon.querySelectorAll(".ple-app-ribbon__link")];
      const selectedTab = ribbon.querySelector(
        '.ple-app-ribbon__tabs .ple-app-ribbon__link[aria-current="page"]',
      );
      const unselectedTab = ribbon.querySelector(
        ".ple-app-ribbon__tabs .ple-app-ribbon__link:not([aria-current])",
      );
      const selectedTask = ribbon.querySelector(
        '.ple-app-ribbon__tasks .ple-app-ribbon__link[aria-current="page"]',
      );
      const unselectedTask = ribbon.querySelector(
        ".ple-app-ribbon__tasks .ple-app-ribbon__link:not([aria-current])",
      );
      const areaSeparator = ribbon.querySelector(
        ".ple-app-ribbon__task-area + .ple-app-ribbon__task-area",
      );
      if (
        !(selectedTab instanceof HTMLElement) ||
        !(unselectedTab instanceof HTMLElement) ||
        !(selectedTask instanceof HTMLElement) ||
        !(unselectedTask instanceof HTMLElement) ||
        !(areaSeparator instanceof HTMLElement)
      ) {
        throw new Error(
          "production Fieldstation fixture lacks a required density-evidence state hook",
        );
      }

      const ordinary = links.filter((link) => !link.hasAttribute("aria-current"));
      const taskRow = ribbon.querySelector(".ple-app-ribbon__tasks");
      if (!(taskRow instanceof HTMLElement)) throw new Error("missing Tier 2 row");
      const settle = (element) => {
        for (const animation of element.getAnimations({ subtree: true })) animation.finish();
      };
      const selectedSnapshot = {
        box: box(selectedTab),
        fontWeight: getComputedStyle(selectedTab).fontWeight,
        background: getComputedStyle(selectedTab, "::after").backgroundColor,
      };
      selectedTab.removeAttribute("aria-current");
      settle(selectedTab);
      const unselectedSameControl = {
        box: box(selectedTab),
        fontWeight: getComputedStyle(selectedTab).fontWeight,
        background: getComputedStyle(selectedTab).backgroundColor,
      };
      selectedTab.setAttribute("aria-current", "page");
      settle(selectedTab);
      const selectedTaskBox = box(selectedTask);
      selectedTask.removeAttribute("aria-current");
      const unselectedTaskBox = box(selectedTask);
      selectedTask.setAttribute("aria-current", "page");
      settle(selectedTask);

      const roleSwapBefore = box(unselectedTab);
      unselectedTab.setAttribute("data-ribbon-role", "destructive");
      unselectedTab.setAttribute("data-ribbon-priority", "critical");
      const roleSwapAfter = box(unselectedTab);
      unselectedTab.removeAttribute("data-ribbon-role");
      unselectedTab.removeAttribute("data-ribbon-priority");

      const local = (property) => getComputedStyle(ribbon).getPropertyValue(property).trim();
      const themePanels = [...document.querySelectorAll(themeSelector)].map((panel) => {
        if (!(panel instanceof HTMLElement)) throw new Error("invalid theme panel");
        const panelRibbon = panel.querySelector(".ple-app-ribbon");
        if (!(panelRibbon instanceof HTMLElement)) throw new Error("theme panel missing Ribbon");
        ensureMultipleTaskAreas(panelRibbon);
        const taskViewport = panelRibbon.querySelector(".ple-app-ribbon__tasks");
        const selectedTask = panelRibbon.querySelector(
          '.ple-app-ribbon__tasks [aria-current="page"]',
        );
        if (!(taskViewport instanceof HTMLElement) || !(selectedTask instanceof HTMLElement)) {
          throw new Error("theme panel lacks its selected collection tab");
        }
        selectedTask.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "instant" });
        const targets = {
          unselectedTab: panelRibbon.querySelector(
            ".ple-app-ribbon__tabs .ple-app-ribbon__link:not([aria-current])",
          ),
          selectedTab: panelRibbon.querySelector(
            '.ple-app-ribbon__tabs .ple-app-ribbon__link[aria-current="page"]',
          ),
          unselectedTask: panelRibbon.querySelector(
            ".ple-app-ribbon__tasks .ple-app-ribbon__link:not([aria-current])",
          ),
          selectedTask: panelRibbon.querySelector(
            '.ple-app-ribbon__tasks .ple-app-ribbon__link[aria-current="page"]',
          ),
          separator: panelRibbon.querySelector(
            ".ple-app-ribbon__task-area + .ple-app-ribbon__task-area",
          ),
        };
        const focusTarget = targets.unselectedTab;
        if (!(focusTarget instanceof HTMLElement)) {
          throw new Error(`missing focus target in ${panel.dataset.ribbonDesignName}`);
        }
        focusTarget.focus();
        return {
          id: panel.dataset.ribbonDesignName,
          pairs: Object.fromEntries(
            Object.entries(targets).map(([name, target]) => {
              if (!(target instanceof HTMLElement))
                throw new Error(`missing ${name} in ${panel.dataset.ribbonDesignName}`);
              const style = getComputedStyle(target);
              const foreground = name === "separator" ? style.borderInlineStartColor : style.color;
              return [name, { foreground, background: backgroundAt(target) }];
            }),
          ),
          indicators: {
            focus: {
              foreground: getComputedStyle(focusTarget).outlineColor,
              background: backgroundAt(focusTarget),
            },
          },
          folder: {
            selectedTabBackground: getComputedStyle(targets.selectedTab, "::after").backgroundColor,
            apronBackground: getComputedStyle(panelRibbon)
              .getPropertyValue("--ple-ribbon-folder-selected-surface")
              .trim(),
            restingShape: getComputedStyle(targets.unselectedTab, "::after").content,
            ribbonBackground: getComputedStyle(
              panelRibbon.querySelector(".ple-app-ribbon__top-bar"),
            ).backgroundColor,
            unselectedTabBackground: getComputedStyle(targets.unselectedTab).backgroundColor,
            topSpace:
              targets.selectedTab.getBoundingClientRect().top -
              panelRibbon.getBoundingClientRect().top,
            bottomSpace:
              taskViewport.getBoundingClientRect().top -
              targets.selectedTab.getBoundingClientRect().bottom,
            taskRowBackground: getComputedStyle(taskViewport).backgroundColor,
            contentBackground: getComputedStyle(panelRibbon)
              .getPropertyValue("--ple-theme-canvas")
              .trim(),
            selectedTaskBackground: getComputedStyle(targets.selectedTask).backgroundColor,
            selectedTaskBottomEdge: getComputedStyle(targets.selectedTask).borderBottomColor,
            selectedWeight: getComputedStyle(targets.selectedTab).fontWeight,
            unselectedWeight: getComputedStyle(targets.unselectedTab).fontWeight,
          },
          focusTreatment: {
            outlineWidth: getComputedStyle(focusTarget).outlineWidth,
            outlineStyle: getComputedStyle(focusTarget).outlineStyle,
            outerOffset: getComputedStyle(focusTarget).outlineOffset,
          },
          rowSurfaces: {
            context: getComputedStyle(panelRibbon.querySelector(".ple-app-ribbon__top-bar"))
              .backgroundColor,
            tabs: getComputedStyle(panelRibbon.querySelector(".ple-app-ribbon__tabs"))
              .backgroundColor,
            tasks: getComputedStyle(panelRibbon.querySelector(".ple-app-ribbon__tasks"))
              .backgroundColor,
          },
          accent: getComputedStyle(panelRibbon)
            .getPropertyValue("--ple-ribbon-course-accent")
            .trim(),
          geometry: {
            selectionWithinTaskViewport: (() => {
              const task = relativeBox(selectedTask, taskViewport);
              const viewport = taskViewport.getBoundingClientRect();
              return (
                task.left >= 0 &&
                task.right <= viewport.width &&
                task.top >= 0 &&
                task.bottom <= viewport.height
              );
            })(),
          },
        };
      });

      return {
        ribbonBox: box(ribbon),
        presentation: [
          ...document.querySelectorAll(
            '[data-ribbon-treatment="fieldstation"] .ple-app-ribbon__link',
          ),
        ].map((link) => ({
          id: link.dataset.ribbonControl,
          presentation: link.dataset.ribbonPresentation,
          className: link.className,
          role: link.dataset.ribbonRole,
          priority: link.dataset.ribbonPriority,
        })),
        gaps: {
          within: Number.parseFloat(
            getComputedStyle(ribbon.querySelector(".ple-app-ribbon__task-area")).gap,
          ),
          between: Number.parseFloat(
            getComputedStyle(ribbon.querySelector(".ple-app-ribbon__tasks")).gap,
          ),
        },
        flat: ordinary.map((link) => {
          const style = getComputedStyle(link);
          return {
            id: link.dataset.ribbonControl,
            tier: link.closest(".ple-app-ribbon__tasks") === null ? "tab" : "task",
            unavailable: link.classList.contains("ple-app-ribbon__link--unavailable"),
            background: style.backgroundColor,
            borderColors: [
              style.borderTopColor,
              style.borderRightColor,
              style.borderBottomColor,
              style.borderLeftColor,
            ],
            borderWidths: [
              style.borderTopWidth,
              style.borderRightWidth,
              style.borderBottomWidth,
              style.borderLeftWidth,
            ],
            shadow: style.boxShadow,
            radius: style.borderTopLeftRadius,
            textDecoration: style.textDecorationLine,
          };
        }),
        selectedTask: {
          box: selectedTaskBox,
          unselectedBox: unselectedTaskBox,
          weight: getComputedStyle(selectedTask).fontWeight,
          unselectedWeight: getComputedStyle(unselectedTask).fontWeight,
          background: getComputedStyle(selectedTask).backgroundColor,
          bottom: selectedTask.getBoundingClientRect().bottom,
          rowBottom: taskRow.getBoundingClientRect().bottom,
          textDecoration: getComputedStyle(selectedTask).textDecorationLine,
        },
        selection: {
          selected: selectedSnapshot,
          unselectedSameControl,
          taskRowBackground: getComputedStyle(taskRow).backgroundColor,
        },
        roleSwap: { before: roleSwapBefore, after: roleSwapAfter },
        hierarchy: {
          contextBackground: getComputedStyle(ribbon.querySelector(".ple-app-ribbon__top-bar"))
            .backgroundColor,
          taskSeparator: getComputedStyle(areaSeparator).borderInlineStartWidth,
          accent: local("--ple-ribbon-course-accent"),
        },
        themes: themePanels,
      };
    },
    { instructorSelector: FIELDSTATION_INSTRUCTOR, themeSelector: FIELDSTATION_THEMES },
  );

  assert.equal(
    evidence.presentation.length > 0,
    true,
    "production Fieldstation exposes visible controls",
  );
  assert.equal(
    evidence.presentation.every(
      (control) =>
        (control.presentation === "standard" || control.presentation === "compact") &&
        control.className.includes(`ple-app-ribbon__link--${control.presentation}`) &&
        control.role === undefined &&
        control.priority === undefined,
    ),
    true,
    "visible controls expose only catalog presentation, never role or priority as a physical hook",
  );
  assert.equal(
    evidence.gaps.between > evidence.gaps.within,
    true,
    "Task Area separation exceeds within-area proximity",
  );
  assert.equal(
    evidence.flat.every((control) => {
      const widths = control.borderWidths.map((width) => Number.parseFloat(width));
      const sameBox = widths.every((width) => width === widths[0] && width > 0);
      if (!sameBox || control.shadow !== "none") return false;
      return (
        parseColor(control.background).alpha === 0 &&
        control.borderColors.every((color) => parseColor(color).alpha === 0) &&
        control.radius === "0px"
      );
    }),
    true,
    "resting choices blend into their continuous rows: " + JSON.stringify(evidence.flat),
  );
  assert.equal(
    evidence.selectedTask.weight !== evidence.selectedTask.unselectedWeight &&
      !evidence.selectedTask.textDecoration.includes("underline"),
    true,
    "selected Tier 2 keeps heavier text without relying on an underline: " +
      JSON.stringify(evidence.selectedTask),
  );
  assert.deepEqual(
    evidence.selectedTask.box,
    evidence.selectedTask.unselectedBox,
    "Tier 2 selection preserves the control box",
  );
  assert.ok(
    Math.abs(evidence.selectedTask.bottom - evidence.selectedTask.rowBottom) <= 1,
    "selected Tier 2 reaches the row floor to connect with content",
  );
  assert.ok(
    evidence.selectedTask.box.height <= evidence.selection.selected.box.height,
    "both tiers stay compact without oversized folders",
  );
  assert.deepEqual(
    evidence.selection.selected.box,
    evidence.selection.unselectedSameControl.box,
    "selection changes no control geometry",
  );
  assert.notEqual(
    evidence.selection.selected.fontWeight,
    evidence.selection.unselectedSameControl.fontWeight,
    "selected Tab has a non-color weight channel",
  );
  assert.notDeepEqual(
    parseColor(evidence.selection.selected.background),
    parseColor(evidence.hierarchy.contextBackground),
    "selected Tier 1 tab emerges from the colored bar: " + JSON.stringify(evidence.selection),
  );
  assert.equal(
    parseColor(evidence.selection.selected.background).alpha > 0 &&
      parseColor(evidence.selection.unselectedSameControl.background).alpha === 0,
    true,
    "only the selected Tier 1 has its own surface: " + JSON.stringify(evidence.selection),
  );
  assert.deepEqual(
    evidence.roleSwap.before,
    evidence.roleSwap.after,
    "role/priority changes cannot change control geometry",
  );
  assert.notEqual(
    evidence.hierarchy.contextBackground,
    "rgba(0, 0, 0, 0)",
    "Context has a quiet distinct surface",
  );
  assert.equal(
    Number.parseFloat(evidence.hierarchy.taskSeparator) > 0,
    true,
    "Task Areas retain a visible separator",
  );
  assert.notEqual(
    evidence.hierarchy.accent,
    "",
    "Ribbon exposes its derived semantic accent alias",
  );

  assert.deepEqual(
    evidence.themes.map((theme) => theme.id).sort(),
    courseThemeRegistry.THEME_OPTIONS.map((theme) => theme.id).sort(),
    "browser evidence measures each closed course-theme panel once",
  );
  for (const theme of evidence.themes) {
    assert.equal(
      parseColor(theme.rowSurfaces.tabs).alpha,
      0,
      `${theme.id} Tier 1 choices sit within the continuous Ribbon bar`,
    );
    assert.equal(
      Number.parseFloat(theme.focusTreatment.outerOffset) > 0 &&
        Number.parseFloat(theme.focusTreatment.outlineWidth) >= 2 &&
        theme.focusTreatment.outlineStyle !== "none",
      true,
      `${theme.id} keeps a visible offset keyboard focus ring: ` +
        JSON.stringify(theme.focusTreatment),
    );
    assert.deepEqual(
      parseColor(theme.folder.selectedTabBackground),
      parseColor(theme.folder.apronBackground),
      `${theme.id} selected Tier 1 tab flows into its Ribbon apron: ` +
        JSON.stringify(theme.folder),
    );
    assert.equal(
      parseColor(theme.folder.selectedTabBackground).alpha > 0,
      true,
      `${theme.id} selected Tier 1 tab surface is painted`,
    );
    assert.notEqual(
      theme.folder.selectedWeight,
      theme.folder.unselectedWeight,
      `${theme.id} selected Tier 1 tab has a heavier weight`,
    );
    assert.notDeepEqual(
      parseColor(theme.folder.selectedTabBackground),
      parseColor(theme.folder.unselectedTabBackground),
      `${theme.id} resting choices stay integrated into the Ribbon`,
    );
    assert.ok(
      theme.folder.topSpace > 0 && theme.folder.bottomSpace > 0,
      `${theme.id} selected tab keeps space above and an apron separating Tier 2`,
    );
    assert.equal(
      theme.folder.restingShape,
      "none",
      `${theme.id} unselected Tier 1 choices have no enclosed tab silhouette`,
    );
    assert.deepEqual(
      parseColor(theme.folder.selectedTaskBackground),
      parseColor(theme.folder.contentBackground),
      `${theme.id} selected Tier 2 shares the content surface`,
    );
    assert.deepEqual(
      parseColor(theme.folder.selectedTaskBottomEdge),
      parseColor(theme.folder.contentBackground),
      `${theme.id} selected Tier 2 has an open bottom edge into content`,
    );
    assert.notDeepEqual(
      parseColor(theme.folder.selectedTaskBackground),
      parseColor(theme.folder.taskRowBackground),
      `${theme.id} the selected Tier 2 surface separates naturally from its row`,
    );
    for (const [name, pair] of Object.entries({ ...theme.pairs, ...theme.indicators })) {
      const ratio = contrast(
        composite(parseColor(pair.foreground), pair.background),
        pair.background,
      );
      const threshold = ["separator", "focus"].includes(name) ? 3 : 5.5;
      assert.equal(
        ratio >= threshold,
        true,
        `${theme.id}:${name} has ${ratio.toFixed(2)}:1 computed contrast ` +
          `(requires ${threshold}:1): ${JSON.stringify(pair)}`,
      );
    }
    assert.notEqual(theme.accent, "", `${theme.id} exposes the derived Ribbon accent alias`);
    assert.equal(
      theme.geometry.selectionWithinTaskViewport,
      true,
      `${theme.id} keeps the selected collection reachable without clipping`,
    );
  }
  await normal.close();

  const forced = await browser.newContext({
    forcedColors: "active",
    viewport: { width: 1280, height: 800 },
  });
  const forcedPage = await forced.newPage();
  await forcedPage.setContent(documentMarkup);
  const forcedEvidence = await forcedPage.evaluate((selector) => {
    const panel = document.querySelector(selector);
    const ribbon = panel?.querySelector(".ple-app-ribbon");
    if (!(ribbon instanceof HTMLElement))
      throw new Error("missing forced-colors production Ribbon");
    const tasks = ribbon.querySelector(".ple-app-ribbon__tasks");
    const firstArea = tasks?.querySelector(".ple-app-ribbon__task-area");
    if (!(tasks instanceof HTMLElement) || !(firstArea instanceof HTMLElement)) {
      throw new Error("missing forced-colors Task Area");
    }
    if (tasks.querySelectorAll(".ple-app-ribbon__task-area").length < 2) {
      const duplicate = firstArea.cloneNode(true);
      if (!(duplicate instanceof HTMLElement))
        throw new Error("forced-colors Task Area clone failed");
      for (const control of duplicate.querySelectorAll('[aria-current="page"]')) {
        control.removeAttribute("aria-current");
      }
      tasks.append(duplicate);
    }
    const tab = ribbon.querySelector(
      '.ple-app-ribbon__tabs .ple-app-ribbon__link[aria-current="page"]',
    );
    const unselectedTab = ribbon.querySelector(
      ".ple-app-ribbon__tabs .ple-app-ribbon__link:not([aria-current])",
    );
    const task = ribbon.querySelector(
      '.ple-app-ribbon__tasks .ple-app-ribbon__link[aria-current="page"]',
    );
    const unselectedTask = ribbon.querySelector(
      ".ple-app-ribbon__tasks .ple-app-ribbon__link:not([aria-current])",
    );
    const separator = ribbon.querySelector(
      ".ple-app-ribbon__task-area + .ple-app-ribbon__task-area",
    );
    if (
      !(tab instanceof HTMLElement) ||
      !(unselectedTab instanceof HTMLElement) ||
      !(task instanceof HTMLElement) ||
      !(unselectedTask instanceof HTMLElement) ||
      !(separator instanceof HTMLElement)
    )
      throw new Error("missing forced-colors density-evidence state");
    const tabStyle = getComputedStyle(tab);
    const unselectedTabStyle = getComputedStyle(unselectedTab);
    const taskStyle = getComputedStyle(task);
    const unselectedTaskStyle = getComputedStyle(unselectedTask);
    const separatorStyle = getComputedStyle(separator);
    const tasksStyle = getComputedStyle(tasks);
    const unfocusedOutline = {
      color: unselectedTabStyle.outlineColor,
      style: unselectedTabStyle.outlineStyle,
      width: unselectedTabStyle.outlineWidth,
    };
    unselectedTab.focus();
    const focusedUnselectedStyle = getComputedStyle(unselectedTab);
    return {
      selectedTab: {
        weight: tabStyle.fontWeight,
        outlineStyle: tabStyle.outlineStyle,
        outlineWidth: tabStyle.outlineWidth,
        textDecorationLine: tabStyle.textDecorationLine,
      },
      unselectedTab: {
        weight: unselectedTabStyle.fontWeight,
        outlineStyle: unselectedTabStyle.outlineStyle,
        outlineWidth: unselectedTabStyle.outlineWidth,
        textDecorationLine: unselectedTabStyle.textDecorationLine,
      },
      selectedTask: { background: taskStyle.backgroundColor, color: taskStyle.color },
      unselectedTask: {
        background: unselectedTaskStyle.backgroundColor,
        color: unselectedTaskStyle.color,
      },
      taskCanvas: tasksStyle.backgroundColor,
      separatorColor: separatorStyle.borderInlineStartColor,
      separatorWidth: separatorStyle.borderInlineStartWidth,
      unfocusedOutline,
      focusedUnselectedOutline: {
        color: focusedUnselectedStyle.outlineColor,
        offset: focusedUnselectedStyle.outlineOffset,
        style: focusedUnselectedStyle.outlineStyle,
        width: focusedUnselectedStyle.outlineWidth,
      },
    };
  }, FIELDSTATION_INSTRUCTOR);
  assert.equal(
    forcedEvidence.selectedTab.weight !== forcedEvidence.unselectedTab.weight &&
      forcedEvidence.selectedTab.outlineStyle !== "none" &&
      Number.parseFloat(forcedEvidence.selectedTab.outlineWidth) > 0 &&
      forcedEvidence.selectedTab.textDecorationLine.includes("underline") &&
      !forcedEvidence.unselectedTab.textDecorationLine.includes("underline"),
    true,
    "forced colors keeps selected Tab distinct through outline and text underline: " +
      JSON.stringify(forcedEvidence),
  );
  assert.equal(
    Number.parseFloat(forcedEvidence.separatorWidth) > 0 &&
      forcedEvidence.separatorColor !== forcedEvidence.taskCanvas,
    true,
    "forced colors keeps the Task Area separator visibly distinct from its Canvas",
  );
  assert.notDeepEqual(
    forcedEvidence.selectedTask,
    forcedEvidence.unselectedTask,
    "forced colors gives the selected Task a distinct system text/background state",
  );
  assert.equal(
    Number.parseFloat(forcedEvidence.focusedUnselectedOutline.width) >= 2 &&
      Number.parseFloat(forcedEvidence.focusedUnselectedOutline.offset) > 0 &&
      forcedEvidence.focusedUnselectedOutline.style !== "none" &&
      JSON.stringify(forcedEvidence.focusedUnselectedOutline) !==
        JSON.stringify(forcedEvidence.unfocusedOutline),
    true,
    "forced colors keeps a distinct offset keyboard focus ring",
  );
  await forced.close();

  const reduced = await browser.newContext({
    reducedMotion: "reduce",
    viewport: { width: 1280, height: 800 },
  });
  const reducedPage = await reduced.newPage();
  await reducedPage.setContent(documentMarkup);
  const motion = await reducedPage.evaluate(() =>
    [...document.querySelectorAll(".ple-app-ribbon, .ple-app-ribbon *")].map((element) => {
      const style = getComputedStyle(element);
      return { animation: style.animationName, transition: style.transitionDuration };
    }),
  );
  assert.equal(
    motion.every(
      ({ animation, transition }) =>
        animation === "none" &&
        transition.split(",").every((duration) => Number.parseFloat(duration) === 0),
    ),
    true,
    "reduced motion removes Ribbon animations and transitions in compiled CSS",
  );
  await reduced.close();

  const phone = await browser.newContext({ viewport: { width: 375, height: 800 } });
  const phonePage = await phone.newPage();
  await phonePage.setContent(documentMarkup);
  const phoneEvidence = await phonePage.evaluate((selector) => {
    const panel = document.querySelector(selector);
    const ribbon = panel?.querySelector(".ple-app-ribbon");
    const tasks = ribbon?.querySelector(".ple-app-ribbon__tasks");
    const frame = ribbon?.querySelector('[data-ribbon-row-frame="tasks"]');
    if (
      !(ribbon instanceof HTMLElement) ||
      !(tasks instanceof HTMLElement) ||
      !(frame instanceof HTMLElement)
    ) {
      throw new Error("missing phone Ribbon task row");
    }
    const area = tasks.querySelector(".ple-app-ribbon__task-area");
    if (!(area instanceof HTMLElement)) throw new Error("missing phone task area");
    let guard = 0;
    while (tasks.scrollWidth <= tasks.clientWidth + 1 && guard < 16) {
      const clone = area.cloneNode(true);
      if (!(clone instanceof HTMLElement)) throw new Error("phone task clone failed");
      for (const control of clone.querySelectorAll("[aria-current]")) {
        control.removeAttribute("aria-current");
      }
      tasks.append(clone);
      guard += 1;
    }
    const cue = frame.querySelector(".ple-app-ribbon__overflow-cue--end");
    if (!(cue instanceof HTMLElement)) throw new Error("missing phone end cue");
    cue.setAttribute("data-ribbon-overflow-active", "true");
    const rowStyle = getComputedStyle(tasks);
    const ribbonBox = ribbon.getBoundingClientRect();
    const frameBox = frame.getBoundingClientRect();
    const rowBox = tasks.getBoundingClientRect();
    const cueBox = cue.getBoundingClientRect();
    const links = [...tasks.querySelectorAll(".ple-app-ribbon__link")].map((link) => {
      const box = link.getBoundingClientRect();
      const label = link.querySelector(".ple-app-ribbon__control-label");
      const labelStyle = label instanceof HTMLElement ? getComputedStyle(label) : rowStyle;
      const crossesEnd = box.left < rowBox.right - 1 && box.right > rowBox.right - 1;
      return {
        textOverflow: labelStyle.textOverflow,
        whiteSpace: labelStyle.whiteSpace,
        crossesEnd,
        coveredByCue: !crossesEnd || (cueBox.left < box.right && cueBox.right > box.left),
      };
    });
    return {
      flexWrap: rowStyle.flexWrap,
      overflowX: rowStyle.overflowX,
      whiteSpace: rowStyle.whiteSpace,
      frameSpansRibbon: frameBox.width >= ribbonBox.width - 1,
      overflows: tasks.scrollWidth > tasks.clientWidth + 1,
      links,
    };
  }, FIELDSTATION_INSTRUCTOR);
  assert.equal(
    phoneEvidence.frameSpansRibbon &&
      phoneEvidence.flexWrap === "nowrap" &&
      phoneEvidence.overflowX === "auto" &&
      phoneEvidence.whiteSpace === "nowrap" &&
      phoneEvidence.overflows &&
      phoneEvidence.links.length > 0 &&
      phoneEvidence.links.every(
        (link) =>
          link.whiteSpace === "nowrap" && link.textOverflow !== "ellipsis" && link.coveredByCue,
      ),
    true,
    "phone Tier 2 spans the row, scrolls one line, and covers a clipped label: " +
      JSON.stringify(phoneEvidence),
  );
  await phone.close();
  process.stdout.write(`Ribbon density evidence passed; screenshots: ${outputDirectory}\n`);
} finally {
  await browser.close();
}
