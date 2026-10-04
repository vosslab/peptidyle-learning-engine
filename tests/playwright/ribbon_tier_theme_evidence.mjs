// Render both tier surfaces in every supported theme and display mode.
import assert from "node:assert/strict";
import { join } from "node:path";
import { THEME_OPTIONS, themeStyle, themeTokens } from "../../src/appearance/theme_registry.ts";

export async function assertTierThemeSurfaces(browser, markup, outputDirectory, colors) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  try {
    const page = await context.newPage();
    await page.setContent(markup);
    for (const mode of ["light", "dark"]) {
      for (const { id } of THEME_OPTIONS) {
        const panel = page.locator(
          `[data-ribbon-treatment="fieldstation"] [data-ribbon-design-panel="theme"][data-ribbon-design-name="${id}"]`,
        );
        const evidence = await panel.evaluate(
          (element, style) => {
            element.setAttribute("style", style);
            const ribbon = element.querySelector(".ple-app-ribbon");
            const top = ribbon.querySelector(".ple-app-ribbon__top-bar");
            const row = ribbon.querySelector(".ple-app-ribbon__tasks");
            const tabs = [
              ...ribbon.querySelectorAll(".ple-app-ribbon__tabs .ple-app-ribbon__link"),
            ];
            const folder = tabs.find((tab) => tab.hasAttribute("aria-current"));
            if (folder === undefined) throw new Error("Theme route has no selected Tier 1.");
            for (const animation of element.getAnimations({ subtree: true })) animation.finish();
            const selected = row.querySelector('[aria-current="page"]');
            const paint = (target, fallback) => {
              const computed = getComputedStyle(target);
              const face = target.matches('.ple-app-ribbon__tabs [aria-current="page"]')
                ? getComputedStyle(target, "::after")
                : computed;
              return {
                color: computed.color,
                background:
                  face.backgroundColor === "rgba(0, 0, 0, 0)" ? fallback : face.backgroundColor,
              };
            };
            const topColor = getComputedStyle(top).backgroundColor;
            const rowColor = getComputedStyle(row).backgroundColor;
            return {
              folder: paint(folder, topColor),
              top: topColor,
              row: rowColor,
              apron: getComputedStyle(ribbon)
                .getPropertyValue("--ple-ribbon-folder-selected-surface")
                .trim(),
              selected: paint(selected, rowColor),
              content: getComputedStyle(ribbon).getPropertyValue("--ple-theme-canvas").trim(),
              labels: [...tabs, ...row.querySelectorAll(".ple-app-ribbon__link")].map((target) =>
                paint(target, target.closest(".ple-app-ribbon__tasks") ? rowColor : topColor),
              ),
            };
          },
          themeStyle(themeTokens(id, mode)),
        );
        const parse = colors.parseColor;
        assert.deepEqual(
          parse(evidence.folder.background),
          parse(evidence.apron),
          `${id}/${mode}: selected curved tab flows into the Ribbon apron`,
        );
        // A regression floor for state surfaces, separate from text accessibility.
        // Fresh all-theme render review remains the aesthetic acceptance step.
        for (const [tier, active, inactive] of [
          ["Tier 1", evidence.folder.background, evidence.top],
          ["Tier 2", evidence.selected.background, evidence.row],
        ]) {
          const ratio = colors.contrast(parse(active), parse(inactive));
          assert.ok(
            ratio >= 1.5,
            `${id}/${mode}: ${tier} active/inactive surface contrast ${ratio.toFixed(2)}:1`,
          );
        }
        assert.deepEqual(
          parse(evidence.selected.background),
          parse(evidence.content),
          `${id}/${mode}: selected section joins content`,
        );
        for (const pair of evidence.labels) {
          const ratio = colors.contrast(parse(pair.color), parse(pair.background));
          assert.ok(ratio >= 5.5, `${id}/${mode}: navigation text contrast ${ratio.toFixed(2)}:1`);
        }
        if (id === "grass" || id === "tundra") {
          await panel.screenshot({
            path: join(outputDirectory, `tier-selection-${id}-${mode}.png`),
            animations: "disabled",
          });
        }
      }
    }
  } finally {
    await context.close();
  }
}
