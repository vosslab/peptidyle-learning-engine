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

const courseThemeRegistry =
  await import("../../src/features/course_appearance/course_theme_registry.ts");
import {
  bundledRibbonDesignFixtureCss,
  loadRibbonDesignFixtureForSsr,
} from "../support/ribbon_design_fixture_loader.ts";

const globalCss = readFileSync(new URL("../../src/style.css", import.meta.url), "utf8");
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
const documentMarkup = [
  viewportMarkup,
  stylesheetMarkup,
  `</head><body>${markup}</body></html>`,
].join("");
const outputDirectory = mkdtempSync(join(tmpdir(), "ple_ribbon_m9b_density_"));

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
          const value = getComputedStyle(current).backgroundColor;
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
      const scopeLabel = ribbon.querySelector(".ple-app-ribbon__course-scope-label");
      const areaSeparator = ribbon.querySelector(
        ".ple-app-ribbon__task-area + .ple-app-ribbon__task-area",
      );
      if (
        !(selectedTab instanceof HTMLElement) ||
        !(unselectedTab instanceof HTMLElement) ||
        !(selectedTask instanceof HTMLElement) ||
        !(unselectedTask instanceof HTMLElement) ||
        !(scopeLabel instanceof HTMLElement) ||
        !(areaSeparator instanceof HTMLElement)
      ) {
        throw new Error(
          "production Fieldstation fixture lacks a required density-evidence state hook",
        );
      }

      const ordinary = links.filter((link) => !link.hasAttribute("aria-current"));
      const selectedBefore = box(selectedTab);
      const selectedStyle = getComputedStyle(selectedTab);
      const selectedUnderline = getComputedStyle(selectedTab, "::after");
      const underlinePaint = (style) => ({
        backgroundColor: style.backgroundColor,
        height: style.height,
      });
      selectedTab.removeAttribute("aria-current");
      const unselectedSameControl = {
        box: box(selectedTab),
        fontWeight: getComputedStyle(selectedTab).fontWeight,
        underline: underlinePaint(getComputedStyle(selectedTab, "::after")),
      };
      selectedTab.setAttribute("aria-current", "page");

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
        const targets = {
          context: panelRibbon.querySelector(".ple-app-ribbon__course-scope-label"),
          contextDetails: panelRibbon.querySelector(".ple-app-ribbon__context-details"),
          unselectedTab: panelRibbon.querySelector(
            ".ple-app-ribbon__tabs .ple-app-ribbon__link:not([aria-current])",
          ),
          selectedTab: panelRibbon.querySelector(
            '.ple-app-ribbon__tabs .ple-app-ribbon__link[aria-current="page"]',
          ),
          taskArea: panelRibbon.querySelector(".ple-app-ribbon__task-area-label"),
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
            scopeMarker: {
              foreground: getComputedStyle(targets.context, "::before").backgroundColor,
              background: backgroundAt(targets.context),
            },
            tabUnderline: {
              foreground: getComputedStyle(targets.selectedTab, "::after").backgroundColor,
              background: backgroundAt(targets.selectedTab),
            },
          },
          focusTreatment: {
            innerEdge: {
              style: getComputedStyle(focusTarget, "::before").borderBlockStartStyle,
              width: getComputedStyle(focusTarget, "::before").borderBlockStartWidth,
            },
            outerOffset: getComputedStyle(focusTarget).outlineOffset,
          },
          rowSurfaces: {
            context: getComputedStyle(panelRibbon.querySelector(".ple-app-ribbon__context"))
              .backgroundColor,
            tabs: getComputedStyle(panelRibbon.querySelector(".ple-app-ribbon__tabs"))
              .backgroundColor,
            tasks: getComputedStyle(panelRibbon.querySelector(".ple-app-ribbon__tasks"))
              .backgroundColor,
          },
          accent: getComputedStyle(panelRibbon)
            .getPropertyValue("--ple-ribbon-course-accent")
            .trim(),
          paints: {
            scopeMarker: getComputedStyle(
              panelRibbon.querySelector(".ple-app-ribbon__course-scope-label"),
              "::before",
            ).backgroundColor,
            tabUnderline: getComputedStyle(
              panelRibbon.querySelector(
                '.ple-app-ribbon__tabs .ple-app-ribbon__link[aria-current="page"]',
              ),
              "::after",
            ).backgroundColor,
            taskBackground: getComputedStyle(
              panelRibbon.querySelector(
                '.ple-app-ribbon__tasks .ple-app-ribbon__link[aria-current="page"]',
              ),
            ).backgroundColor,
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
          };
        }),
        selection: {
          selected: {
            box: selectedBefore,
            fontWeight: selectedStyle.fontWeight,
            underline: underlinePaint(selectedUnderline),
          },
          unselectedSameControl,
        },
        roleSwap: { before: roleSwapBefore, after: roleSwapAfter },
        hierarchy: {
          contextBackground: getComputedStyle(ribbon.querySelector(".ple-app-ribbon__context"))
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
    new Set(evidence.presentation.map((control) => control.presentation)).size,
    2,
    "the selected Instructor fixture visibly exercises both standard and compact presentations",
  );
  assert.equal(
    evidence.gaps.between > evidence.gaps.within,
    true,
    "Task Area separation exceeds within-area proximity",
  );
  assert.equal(
    evidence.flat.every((control) => {
      const transparentBorders = control.borderColors.every(
        (color, index) =>
          Number.parseFloat(control.borderWidths[index]) === 0 || parseColor(color).alpha === 0,
      );
      return (
        transparentBorders &&
        parseColor(control.background).alpha === 0 &&
        control.shadow === "none"
      );
    }),
    true,
    "resting Ribbon controls remain flat rather than becoming bordered or shadowed cards",
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
  assert.equal(
    Number.parseFloat(evidence.selection.selected.underline.height) > 0 &&
      parseColor(evidence.selection.selected.underline.backgroundColor).alpha > 0 &&
      parseColor(evidence.selection.unselectedSameControl.underline.backgroundColor).alpha === 0,
    true,
    "selected Tab adds a non-color underline shape without changing its box: " +
      JSON.stringify(evidence.selection),
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
    courseThemeRegistry.COURSE_THEME_OPTIONS.map((theme) => theme.id).sort(),
    "browser evidence measures each closed course-theme panel once",
  );
  for (const theme of evidence.themes) {
    assert.equal(
      new Set(Object.values(theme.rowSurfaces).map((paint) => JSON.stringify(parseColor(paint))))
        .size,
      3,
      `${theme.id} keeps Context, Tab, and Task Rows on distinct neutral planes: ` +
        JSON.stringify(theme.rowSurfaces),
    );
    assert.equal(
      Number.parseFloat(theme.focusTreatment.outerOffset) > 0 &&
        Number.parseFloat(theme.focusTreatment.innerEdge.width) > 0 &&
        theme.focusTreatment.innerEdge.style !== "none",
      true,
      `${theme.id} keeps a two-part inner-edge and outer-offset focus indicator: ` +
        JSON.stringify(theme.focusTreatment),
    );
    for (const [name, pair] of Object.entries({ ...theme.pairs, ...theme.indicators })) {
      const ratio = contrast(
        composite(parseColor(pair.foreground), pair.background),
        pair.background,
      );
      const threshold = ["separator", "focus", "scopeMarker", "tabUnderline"].includes(name)
        ? 3
        : 5.5;
      assert.equal(
        ratio >= threshold,
        true,
        `${theme.id}:${name} has ${ratio.toFixed(2)}:1 computed contrast ` +
          `(requires ${threshold}:1): ${JSON.stringify(pair)}`,
      );
    }
    assert.notEqual(theme.accent, "", `${theme.id} exposes the derived Ribbon accent alias`);
    const accentPaint = parseColor(theme.paints.scopeMarker);
    for (const [placement, paint] of Object.entries(theme.paints)) {
      assert.deepEqual(
        parseColor(paint),
        accentPaint,
        `${theme.id}:${placement} paints the same derived Ribbon accent ` +
          "at every semantic placement",
      );
    }
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
        underline: {
          color: getComputedStyle(tab, "::after").backgroundColor,
          height: getComputedStyle(tab, "::after").height,
        },
      },
      unselectedTab: {
        weight: unselectedTabStyle.fontWeight,
        underline: {
          color: getComputedStyle(unselectedTab, "::after").backgroundColor,
          height: getComputedStyle(unselectedTab, "::after").height,
        },
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
      focusedUnselectedInnerEdge: {
        color: getComputedStyle(unselectedTab, "::before").borderBlockStartColor,
        style: getComputedStyle(unselectedTab, "::before").borderBlockStartStyle,
        width: getComputedStyle(unselectedTab, "::before").borderBlockStartWidth,
      },
    };
  }, FIELDSTATION_INSTRUCTOR);
  assert.equal(
    forcedEvidence.selectedTab.weight !== forcedEvidence.unselectedTab.weight &&
      Number.parseFloat(forcedEvidence.selectedTab.underline.height) > 0 &&
      forcedEvidence.selectedTab.underline.color !== forcedEvidence.unselectedTab.underline.color,
    true,
    "forced colors keeps selected Tab distinct through both weight and underline shape: " +
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
    Number.parseFloat(forcedEvidence.focusedUnselectedOutline.width) > 0 &&
      Number.parseFloat(forcedEvidence.focusedUnselectedOutline.offset) > 0 &&
      forcedEvidence.focusedUnselectedOutline.style !== "none" &&
      Number.parseFloat(forcedEvidence.focusedUnselectedInnerEdge.width) > 0 &&
      forcedEvidence.focusedUnselectedInnerEdge.style !== "none" &&
      JSON.stringify(forcedEvidence.focusedUnselectedOutline) !==
        JSON.stringify(forcedEvidence.unfocusedOutline),
    true,
    "forced colors keeps a two-part inner-edge and outer-offset focus indicator",
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
  process.stdout.write(`Ribbon density evidence passed; screenshots: ${outputDirectory}\n`);
} finally {
  await browser.close();
}
