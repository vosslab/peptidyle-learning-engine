import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { renderToString } from "solid-js/web";
import { createComponent } from "solid-js";

import { bundledAppRibbonCss, loadAppRibbonForSsr } from "../support/ribbon_component_ssr.ts";
import { M6_RIBBON_FIXTURES } from "../support/ribbon_model_fixtures.ts";
import { routeParams, routeScopeKey } from "../../src/navigation/route_params.ts";
import { routeContractForPathname } from "../../src/route_contract.ts";
import { RIBBON_TASK_CATALOG, TAB_CATALOG } from "../../src/ribbon/ribbon_catalog.ts";
import {
  RIBBON_ICON_ASSET_PATH,
  ribbonGlyphForDestination,
} from "../../src/ribbon/ribbon_icons.ts";

const catalogById = new Map(
  [...TAB_CATALOG, ...RIBBON_TASK_CATALOG].map((control) => [control.id, control]),
);
const fixtureParams = {
  courseRef: "C-1",
  assignmentRef: "A-1",
  assignmentAttemptRef: "R-1",
};
const permanentRowsTestName = [
  "AppRibbon preserves its three permanent semantic rows",
  "and withholds non-admitted controls",
].join(" ");
const bundledGlyphTestName = [
  "AppRibbon pairs the real bundled glyph with labels",
  "and reserves icon-only semantics",
].join(" ");
const modelGlyphDeclarationTestName = [
  "AppRibbon lets each supplied model declare whether",
  "a closed-map destination bears a glyph",
].join(" ");
const narrowPhoneTestName = [
  "AppRibbon CSS confines label hiding to declared",
  "narrow-phone icon-only controls",
].join(" ");
const catalogPresentationTestName = [
  "AppRibbon exposes catalog presentation without turning",
  "role or priority into visual geometry",
].join(" ");

function fixtureControls(model) {
  return [...model.tabs, ...model.taskAreas.flatMap((area) => area.controls)];
}

test("every fixture href is a canonical declared route with its catalog parameters", () => {
  for (const [fixtureName, model] of Object.entries(M6_RIBBON_FIXTURES)) {
    for (const control of fixtureControls(model)) {
      const catalog = catalogById.get(control.id);
      assert.ok(catalog, `${fixtureName}:${control.id} is catalogued`);
      const documentedUnavailable =
        catalog.destination.kind === "future" || catalog.id === "backToAssignments";
      if (documentedUnavailable) {
        assert.equal(
          control.availability,
          "Unavailable",
          `${fixtureName}:${control.id} is a documented unavailable fixture control`,
        );
        assert.equal(
          control.href,
          undefined,
          `${fixtureName}:${control.id} has no navigable fixture route`,
        );
        continue;
      }
      assert.equal(catalog.destination.kind, "route");
      assert.equal(
        control.availability,
        "Available",
        `${fixtureName}:${control.id} is a route-backed fixture control`,
      );
      assert.ok(control.href, `${fixtureName}:${control.id} supplies its declared route`);
      const target = routeContractForPathname(control.href);
      assert.equal(
        target?.id,
        catalog.destination.routeId,
        `${fixtureName}:${control.id} preserves route ID`,
      );
      assert.deepEqual(
        routeParams(target, control.href),
        Object.fromEntries(catalog.requiredParams.map((name) => [name, fixtureParams[name]])),
        `${fixtureName}:${control.id} supplies exact target parameters`,
      );
      assert.notEqual(
        routeScopeKey(control.href).kind,
        "invalid",
        `${fixtureName}:${control.id} has valid scope`,
      );
    }
  }
});

test(permanentRowsTestName, async () => {
  const RealAppRibbon = await loadAppRibbonForSsr();
  const html = renderToString(() =>
    createComponent(RealAppRibbon, { model: M6_RIBBON_FIXTURES.courseInstructor }),
  );
  assert.match(html, /aria-label="PLE application Ribbon"/);
  assert.deepEqual(
    [...html.matchAll(/data-ribbon-row="([^"]+)"/g)].map((match) => match[1]),
    ["context", "tabs", "tasks"],
  );
  assert.deepEqual(
    [...html.matchAll(/data-ribbon-row-frame="([^"]+)"/g)].map((match) => match[1]),
    ["context", "tabs", "tasks"],
    "each permanent labelled row has exactly one corresponding non-scrolling cue frame",
  );
  assert.match(html, /aria-label="Ribbon context"/);
  assert.match(html, /<nav[^>]*aria-label="Ribbon tabs"/);
  assert.match(html, /<nav[^>]*aria-label="Ribbon tasks"/);
  assert.match(html, /aria-current="page"/);
  assert.match(html, /data-ribbon-action="signOut"/);
  assert.match(html, /Problem Set 7/);
  assert.doesNotMatch(html, /Blueprint Updates|Course Setup/);
  assert.doesNotMatch(html, /role="tab(list)?"/);
  assert.ok(html.indexOf("signOut") < html.indexOf("Assignments"));
  assert.ok(html.indexOf("Assignments") < html.indexOf("Overview"));
});

test("AppRibbon retains a real empty Ribbon task navigation row", async () => {
  const RealAppRibbon = await loadAppRibbonForSsr();
  const html = renderToString(() =>
    createComponent(RealAppRibbon, { model: M6_RIBBON_FIXTURES.courseStudent }),
  );
  const taskNavigation = html.match(/<nav[^>]*aria-label="Ribbon tasks"[^>]*>(.*?)<\/nav>/);
  assert.ok(taskNavigation, "the empty Task Row remains a labelled navigation landmark");
  assert.doesNotMatch(
    taskNavigation[1],
    /<(?:a|button)\b/,
    "empty means no Task control is invented",
  );
  const taskFrame = html.match(/<section[^>]*data-ribbon-row-frame="tasks"[^>]*>(.*?)<\/section>/);
  assert.ok(taskFrame, "the empty Task row retains its direct paint-only cue frame");
  assert.deepEqual(
    [...taskFrame[1].matchAll(/data-ribbon-overflow-cue="([^"]+)"/g)].map((match) => match[1]),
    ["start", "end"],
    "the empty permanent row retains its noninteractive overflow affordance outside its scrollport",
  );
  assert.deepEqual(
    [...html.matchAll(/data-ribbon-row="([^"]+)"/g)].map((match) => match[1]),
    ["context", "tabs", "tasks"],
  );
});

test("AppRibbon emits its production component CSS through the Solid bundle path", async () => {
  const css = await bundledAppRibbonCss();
  assert.match(css, /\.ple-app-ribbon/);
  assert.match(css, /--ple-ribbon-block-size/);
  assert.match(css, /grid-template-rows/);
});

test(bundledGlyphTestName, async () => {
  const RealAppRibbon = await loadAppRibbonForSsr();
  const html = renderToString(() =>
    createComponent(RealAppRibbon, { model: M6_RIBBON_FIXTURES.courseInstructor }),
  );
  const iconControls = fixtureControls(M6_RIBBON_FIXTURES.courseInstructor).filter(
    (control) => control.availability === "Available" && ribbonGlyphForDestination(control.id),
  );
  for (const control of iconControls) {
    const glyph = ribbonGlyphForDestination(control.id);
    assert.match(
      html,
      new RegExp(
        [
          `data-ribbon-control="${control.id}"[^>]*>[\\s\\S]*?`,
          `<use href="${RIBBON_ICON_ASSET_PATH}#${glyph}"`,
        ].join(""),
      ),
      `${control.label} renders its one same-origin sprite glyph`,
    );
  }
  for (const control of fixtureControls(M6_RIBBON_FIXTURES.courseInstructor).filter(
    (item) => item.availability === "Available" && !ribbonGlyphForDestination(item.id),
  )) {
    const controlMarkup = html.match(
      new RegExp(`<a[^>]*data-ribbon-control="${control.id}"[^>]*>([\\s\\S]*?)</a>`),
    )?.[1];
    assert.ok(controlMarkup, `${control.label} renders`);
    assert.doesNotMatch(
      controlMarkup,
      /<svg|<use /,
      `${control.label} stays intentionally text-only`,
    );
  }
  assert.match(html, /<svg[^>]*aria-hidden="true"[^>]*focusable="false"[^>]*>/);
  assert.doesNotMatch(html, /<(?:img|i)\b/);
  assert.match(html, /aria-label="Sign out"[^>]*title="Sign out"[^>]*data-ribbon-action="signOut"/);
});

test(modelGlyphDeclarationTestName, async () => {
  const RealAppRibbon = await loadAppRibbonForSsr();
  const baseline = M6_RIBBON_FIXTURES.courseInstructor;
  const modelDeclaringAssignmentsTextOnly = {
    ...baseline,
    tabs: baseline.tabs.map((control) =>
      control.id === "assignments"
        ? { ...control, iconBearing: false, iconOnlySafe: true }
        : control,
    ),
  };
  const textOnlyMapEntryClaimingAGlyph = {
    ...baseline,
    tabs: baseline.tabs.map((control) =>
      control.id === "teachingOperations"
        ? { ...control, iconBearing: true, iconOnlySafe: true }
        : control,
    ),
  };

  const declaredTextOnlyHtml = renderToString(() =>
    createComponent(RealAppRibbon, { model: modelDeclaringAssignmentsTextOnly }),
  );
  const declaredTextOnlyAssignments = declaredTextOnlyHtml.match(
    /<a[^>]*data-ribbon-control="assignments"[^>]*>([\s\S]*?)<\/a>/,
  )?.[1];
  assert.ok(declaredTextOnlyAssignments, "the model-declared text-only destination still renders");
  assert.match(declaredTextOnlyAssignments, /Assignments/);
  assert.doesNotMatch(declaredTextOnlyAssignments, /<svg|<use |data-ribbon-icon-only-safe/);
  assert.doesNotMatch(
    declaredTextOnlyHtml.match(/<a[^>]*data-ribbon-control="assignments"[^>]*>/)?.[0] ?? "",
    /(?:aria-label|title)=/,
    "an undeclared glyph cannot trigger icon-only naming semantics",
  );

  const unmappedGlyphClaimHtml = renderToString(() =>
    createComponent(RealAppRibbon, { model: textOnlyMapEntryClaimingAGlyph }),
  );
  const unmappedGlyphClaim = unmappedGlyphClaimHtml.match(
    /<a[^>]*data-ribbon-control="teachingOperations"[^>]*>([\s\S]*?)<\/a>/,
  )?.[1];
  assert.ok(unmappedGlyphClaim, "the deliberately unmapped text-only destination still renders");
  assert.match(unmappedGlyphClaim, /Teaching Operations/);
  assert.doesNotMatch(unmappedGlyphClaim, /<svg|<use |data-ribbon-icon-only-safe/);

  const ordinaryHtml = renderToString(() => createComponent(RealAppRibbon, { model: baseline }));
  const ordinaryAssignments = ordinaryHtml.match(
    /<a[^>]*data-ribbon-control="assignments"[^>]*>([\s\S]*?)<\/a>/,
  )?.[1];
  assert.ok(ordinaryAssignments, "an ordinary declared icon-bearing destination renders");
  assert.equal(
    (ordinaryAssignments.match(/<svg\b/g) ?? []).length,
    1,
    "a valid model declaration and closed-map entry resolve exactly one glyph",
  );
  assert.match(
    ordinaryAssignments,
    new RegExp(`<use href="${RIBBON_ICON_ASSET_PATH}#clipboard-list"`),
  );
});

test(narrowPhoneTestName, async () => {
  const css = await bundledAppRibbonCss();
  assert.match(css, /@media \(max-width: 24rem\)/);
  assert.match(css, /data-ribbon-icon-only-safe=true/);
  assert.match(css, /clip-path: inset\(50%\)/);
  assert.match(css, /inline-size: 2\.75rem/);
  assert.match(css, /fill: currentColor/);
});

test(catalogPresentationTestName, async () => {
  const RealAppRibbon = await loadAppRibbonForSsr();
  const model = M6_RIBBON_FIXTURES.courseInstructor;
  const html = renderToString(() => createComponent(RealAppRibbon, { model }));

  for (const control of fixtureControls(model).filter(
    (item) => item.availability === "Available",
  )) {
    const catalog = catalogById.get(control.id);
    assert.ok(catalog, `${control.id} is catalogued`);
    assert.match(
      html,
      new RegExp(
        [
          `data-ribbon-control="${control.id}"[^>]*class=`,
          `"[^"]*ple-app-ribbon__link--${catalog.presentation}[^"]*"[^>]*`,
          `data-ribbon-presentation="${catalog.presentation}"`,
          "|",
          `class="[^"]*ple-app-ribbon__link--${catalog.presentation}[^"]*"[^>]*`,
          `data-ribbon-control="${control.id}"[^>]*`,
          `data-ribbon-presentation="${catalog.presentation}"`,
        ].join(""),
      ),
      `${control.label} keeps its catalog presentation declaration in the DOM`,
    );
  }

  const source = readFileSync(new URL("../../src/ribbon/app_ribbon.css", import.meta.url), "utf8");
  for (const alias of [
    "--ple-ribbon-space-tight",
    "--ple-ribbon-space-control",
    "--ple-ribbon-space-within-area",
    "--ple-ribbon-space-between-area",
    "--ple-ribbon-space-row",
    "--ple-ribbon-space-inline",
    "--ple-ribbon-space-cue",
  ]) {
    assert.match(
      source,
      new RegExp(`${alias}: var\\(--ple-space-[1-7]\\)`),
      `${alias} derives from the shared spacing scale`,
    );
  }

  const declaredRibbonSpacingAliases = new Set(
    [...source.matchAll(/(--ple-ribbon-space-[a-z-]+):\s*var\((--ple-space-[1-7])\)/g)].map(
      ([, alias]) => alias,
    ),
  );
  const spacingProperty = new RegExp(
    [
      "^(?:gap|(?:row|column)-gap|",
      "(?:padding|margin|scroll-padding)",
      "(?:-(?:(?:block|inline)(?:-(?:start|end))?|top|right|bottom|left))?)$",
    ].join(""),
  );
  const spacingDeclarations = [...source.matchAll(/([^{}]+)\{([^{}]*)\}/gs)].flatMap(
    ([, selector, body]) =>
      [...body.matchAll(/(^|;)\s*([a-z-]+)\s*:\s*([^;{}]+)\s*(?=;|$)/gm)].flatMap(
        ([, , property, value]) =>
          spacingProperty.test(property)
            ? [{ selector: selector.trim(), property, value: value.trim() }]
            : [],
      ),
  );
  assert.ok(spacingDeclarations.length > 0, "the production Ribbon exposes spacing declarations");
  for (const declaration of spacingDeclarations) {
    const alias = declaration.value.match(/^var\((--ple-ribbon-space-[a-z-]+)\)$/)?.[1];
    const deliberateOffscreenMargin =
      declaration.property === "margin" &&
      declaration.value === "-1px" &&
      declaration.selector ===
        [
          '.ple-app-ribbon__link[data-ribbon-icon-only-safe="true"] ',
          ".ple-app-ribbon__control-label,\n  ",
          ".ple-app-ribbon__sign-out .ple-app-ribbon__control-label",
        ].join("");
    assert.equal(
      Boolean(alias && declaredRibbonSpacingAliases.has(alias)) ||
        declaration.value === "0" ||
        declaration.value === "auto" ||
        deliberateOffscreenMargin,
      true,
      [
        "Ribbon spacing must use a declared local scale alias (or its one deliberate exception):",
        `${declaration.selector} { ${declaration.property}: ${declaration.value} }`,
      ].join(" "),
    );
  }
  assert.match(source, /\.ple-app-ribbon__link--standard/);
  assert.match(source, /\.ple-app-ribbon__link--compact/);
  assert.doesNotMatch(source, /\[(?:data-ribbon-)?(?:role|priority)(?:[=\]])/);
  assert.doesNotMatch(source, /\.(?:role|priority)-/);
  const ordinaryDeclarations = [...source.matchAll(/([^{}]+)\{([^{}]*)\}/gs)].flatMap(
    ([, selector, body]) =>
      [...body.matchAll(/(^|;)\s*([a-z-]+)\s*:\s*([^;{}]+)\s*(?=;|$)/gm)].map(
        ([, , property, value]) => ({
          selector: selector.trim(),
          property,
          value: value.trim(),
        }),
      ),
  );
  const accentDefinitions = ordinaryDeclarations.filter(
    (declaration) => declaration.property === "--ple-ribbon-course-accent",
  );
  assert.equal(accentDefinitions.length, 1, "the Ribbon defines its course accent exactly once");
  assert.deepEqual(
    accentDefinitions[0] && {
      selector: accentDefinitions[0].selector,
      property: accentDefinitions[0].property,
    },
    { selector: ".ple-app-ribbon", property: "--ple-ribbon-course-accent" },
    "the single course-accent definition belongs to the Ribbon root",
  );
  assert.match(
    accentDefinitions[0]?.value ?? "",
    /^var\(--ple-accent(?:-strong)?\)$/,
    "the one Ribbon course accent derives only through the approved semantic theme recipe",
  );
  const accentPaintConsumers = ordinaryDeclarations
    .filter(
      (declaration) =>
        declaration.property !== "--ple-ribbon-course-accent" &&
        declaration.value.includes("var(--ple-ribbon-course-accent)"),
    )
    .map(({ selector, property }) => ({ selector, property }))
    .sort((left, right) =>
      `${left.selector}:${left.property}`.localeCompare(`${right.selector}:${right.property}`),
    );
  assert.deepEqual(
    accentPaintConsumers,
    [
      { selector: ".ple-app-ribbon__course-scope-label::before", property: "background" },
      {
        selector: '.ple-app-ribbon__tabs .ple-app-ribbon__link[aria-current="page"]::after',
        property: "background",
      },
      {
        selector: '.ple-app-ribbon__tasks .ple-app-ribbon__link[aria-current="page"]',
        property: "background",
      },
    ].sort((left, right) =>
      `${left.selector}:${left.property}`.localeCompare(`${right.selector}:${right.property}`),
    ),
    "the derived course accent has exactly its three semantic production paint placements",
  );
  assert.doesNotMatch(
    source,
    /var\(--ple-theme-[a-z0-9-]+\)/i,
    "Ribbon production paint never bypasses the semantic theme recipe with a raw theme anchor",
  );
});
