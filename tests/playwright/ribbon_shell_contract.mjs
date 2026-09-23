// Signed-in shell contracts: role-stable tier-one navigation and fixed geometry.
// Selector contract: ApplicationShell owns the breadcrumb/main rails; PageFrame owns the title;
// AppRibbon exposes stable tier-one control IDs.

import assert from "node:assert/strict";

import { RIBBON_ROUTE_MATERIALIZATIONS } from "../support/ribbon_model_fixtures.ts";
import { CANONICAL_VIEWPORTS } from "./screenshot_corpus/manifest.ts";
import { caseLocator, openRibbonShellEvidencePage, waitForPath } from "./ribbon_shell_helpers.mjs";

function tierOneControlIds(ribbon) {
  return ribbon
    .locator('nav[aria-label="Ribbon tabs"] [data-ribbon-control-id]')
    .evaluateAll((controls) =>
      controls.map((control) => control.getAttribute("data-ribbon-control-id")),
    );
}

async function shellGeometry(shellCase) {
  return shellCase.evaluate((root) => {
    const breadcrumb = root.querySelector(".ple-shell__breadcrumb-prelude");
    const mainContent = root.querySelector("#main-content");
    const pageTitle = root.querySelector(".page-frame__title");
    if (
      !(breadcrumb instanceof HTMLElement) ||
      !(mainContent instanceof HTMLElement) ||
      !(pageTitle instanceof HTMLElement)
    ) {
      throw new Error("Signed-in shell fixture is missing its breadcrumb, main content, or title.");
    }
    return {
      breadcrumbTop: breadcrumb.getBoundingClientRect().top,
      mainContentTop: mainContent.getBoundingClientRect().top,
      pageTitleLeft: pageTitle.getBoundingClientRect().left,
    };
  });
}

const { browser, page, harnessServer, pageErrors, consoleErrors } =
  await openRibbonShellEvidencePage();
try {
  const fixtureCase = caseLocator(page, "fixture-shell");
  const shellOffsets = new Map();
  const studentTitleEdges = new Map();
  const tierOneSequences = new Map();

  for (const [viewportId, viewport] of Object.entries(CANONICAL_VIEWPORTS)) {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    for (const materialization of RIBBON_ROUTE_MATERIALIZATIONS) {
      const { productRole, pathname, route } = materialization;
      if (productRole !== "student" && viewportId !== "laptop") continue;

      await page.evaluate(
        ({ role, routeId }) => window.ribbonShell.fixtureNavigateRoute(role, routeId),
        { role: productRole, routeId: route.id },
      );
      await waitForPath(page, "fixture-shell", pathname);
      const ribbon = fixtureCase.getByRole("region", {
        name: "PLE application Ribbon",
        exact: true,
      });
      await ribbon.waitFor();
      await page.waitForFunction(
        ({ role }) =>
          document
            .querySelector('[data-m10-case="fixture-shell"] .ple-app-ribbon')
            ?.getAttribute("data-ribbon-product-role") === role,
        { role: productRole },
      );

      const { pageTitleLeft, ...offsets } = await shellGeometry(fixtureCase);
      const key = `${viewportId}/${productRole}`;
      const previousOffsets = shellOffsets.get(key);
      if (previousOffsets === undefined) shellOffsets.set(key, offsets);
      else
        assert.deepEqual(offsets, previousOffsets, `${key} shell geometry drifts at ${route.id}`);

      if (productRole === "student") {
        const previousTitleLeft = studentTitleEdges.get(viewportId);
        if (previousTitleLeft === undefined) studentTitleEdges.set(viewportId, pageTitleLeft);
        else assert.equal(pageTitleLeft, previousTitleLeft, `${key} PageFrame title origin drifts`);
      }

      const ids = await tierOneControlIds(ribbon);
      assert.notEqual(ids.length, 0, `${key}/${route.id} has tier-one controls`);
      const previousIds = tierOneSequences.get(key);
      if (previousIds === undefined) tierOneSequences.set(key, ids);
      else assert.deepEqual(ids, previousIds, `${key} tier-one navigation drifts at ${route.id}`);
    }
  }

  assert.deepEqual(pageErrors, [], "signed-in route sweep raises no browser page errors");
  assert.deepEqual(consoleErrors, [], "signed-in route sweep raises no browser console errors");
  process.stdout.write(
    "Ribbon navigation and shell geometry remain stable across role-reachable routes.\n",
  );
} finally {
  await browser.close();
  await harnessServer.close();
}
