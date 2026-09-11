// Narrow routed-shell geometry assertions shared by M2 browser evidence.

import assert from "node:assert/strict";

export async function assertNarrowRoutedShell(currentCase) {
  const evidence = await currentCase.evaluate((root) => {
    const ribbon = root.querySelector(".ple-app-ribbon");
    const contentFlow = root.querySelector("main.shell");
    const prelude = root.querySelector(".ple-shell__breadcrumb-prelude");
    const mainContent = root.querySelector("#main-content");
    if (
      !(ribbon instanceof HTMLElement) ||
      !(contentFlow instanceof HTMLElement) ||
      !(prelude instanceof HTMLElement) ||
      !(mainContent instanceof HTMLElement)
    ) {
      throw new Error("narrow routed-shell capture is missing its required regions");
    }
    const preludeBounds = prelude.getBoundingClientRect();
    const mainBounds = mainContent.getBoundingClientRect();
    const breadcrumbItems = [...prelude.querySelectorAll("li")].map((item) => {
      const label = item.querySelector("a, span");
      if (!(label instanceof HTMLElement)) throw new Error("breadcrumb item has no label");
      return {
        text: label.textContent,
        clientWidth: label.clientWidth,
        scrollWidth: label.scrollWidth,
      };
    });
    const breadcrumbNav = prelude.querySelector("nav");
    const breadcrumbList = prelude.querySelector("ol");
    if (!(breadcrumbNav instanceof HTMLElement) || !(breadcrumbList instanceof HTMLElement)) {
      throw new Error("breadcrumb prelude has no nav/list");
    }
    return {
      ribbonImmediatelyPrecedesContentFlow:
        contentFlow.previousElementSibling?.querySelector(".ple-app-ribbon") === ribbon,
      breadcrumbIsFirstContentPrelude: contentFlow.firstElementChild === prelude,
      firstContentFollowsBreadcrumb: prelude.nextElementSibling === mainContent,
      firstContentOriginBelowPrelude: mainBounds.top >= preludeBounds.bottom,
      breadcrumbLandmarks: prelude.querySelectorAll('nav[aria-label="Breadcrumb"]').length,
      documentHasNoHorizontalOverflow:
        document.documentElement.scrollWidth <= document.documentElement.clientWidth,
      documentWidth: document.documentElement.clientWidth,
      documentScrollWidth: document.documentElement.scrollWidth,
      overflowSources: [...document.body.querySelectorAll("*")]
        .filter(
          (element) => element.getBoundingClientRect().right > document.documentElement.clientWidth,
        )
        .slice(0, 4)
        .map((element) => element.className || element.tagName),
      topRowMetrics: [
        ...root.querySelectorAll(".ple-app-ribbon__row-frame, .ple-app-ribbon__row"),
      ].map((element) => ({
        className: element.className,
        clientWidth: element.clientWidth,
        scrollWidth: element.scrollWidth,
        right: element.getBoundingClientRect().right,
      })),
      breadcrumbItems,
      breadcrumbWidths: {
        nav: breadcrumbNav.clientWidth,
        list: breadcrumbList.clientWidth,
        prelude: prelude.clientWidth,
      },
      ribbonTop: ribbon.getBoundingClientRect().top,
      breadcrumbTop: preludeBounds.top,
      contentTop: mainBounds.top,
    };
  });
  const {
    ribbonTop,
    breadcrumbTop,
    contentTop,
    documentHasNoHorizontalOverflow,
    documentWidth,
    documentScrollWidth,
    overflowSources,
    topRowMetrics,
    breadcrumbItems,
    breadcrumbWidths,
    ...structure
  } = evidence;
  assert.deepEqual(
    structure,
    {
      ribbonImmediatelyPrecedesContentFlow: true,
      breadcrumbIsFirstContentPrelude: true,
      firstContentFollowsBreadcrumb: true,
      firstContentOriginBelowPrelude: true,
      breadcrumbLandmarks: 1,
    },
    "the narrow routed shell keeps Ribbon, breadcrumb prelude, and first route content in order",
  );
  assert.equal(
    documentHasNoHorizontalOverflow,
    true,
    `the narrow routed shell has no document-level horizontal overflow ` +
      `(${documentScrollWidth}px of ${documentWidth}px; ${overflowSources.join(", ")}; ` +
      `${JSON.stringify(topRowMetrics)})`,
  );
  assert.ok(
    ribbonTop <= breadcrumbTop && breadcrumbTop <= contentTop,
    "the narrow breadcrumb follows the dense Ribbon",
  );
  assert.deepEqual(
    breadcrumbItems.map((item) => item.text),
    ["Courses", "Course C-1"],
    "the narrow breadcrumb preserves the root and current labels",
  );
  assert.equal(
    breadcrumbItems[0].scrollWidth <= breadcrumbItems[0].clientWidth,
    true,
    "the narrow root breadcrumb stays fully recognizable",
  );
  assert.equal(
    breadcrumbItems[1].scrollWidth <= breadcrumbItems[1].clientWidth,
    true,
    `the narrow current breadcrumb receives enough remaining width for this Course label: ${JSON.stringify({ breadcrumbItems, breadcrumbWidths })}`,
  );
}
