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
    ["Home", "Courses", "My Active Courses", "BCHM 355"],
    "the narrow breadcrumb uses the compact Course identity when the long name does not fit",
  );
}

const TEXT_ZOOM_SENTENCE =
  "Preserve readable text and reachable controls as users enlarge text or zoom the page.";

async function readTextZoomTargets(page) {
  return page.evaluate(() => {
    const link = document.querySelector("a.ple-app-ribbon__link");
    const label = link?.querySelector(".ple-app-ribbon__control-label");
    const profile = document.querySelector(".ple-app-ribbon__profile");
    const outside = document.querySelector("#outside");
    if (!(link instanceof HTMLElement) || !(label instanceof HTMLElement)) {
      throw new Error("text zoom requires a Ribbon link label");
    }
    if (!(profile instanceof HTMLElement) || !(outside instanceof HTMLButtonElement)) {
      throw new Error("text zoom requires the Profile control and the outside button");
    }
    const textNode = [...label.childNodes].find(
      (node) => node.nodeType === Node.TEXT_NODE && node.textContent.trim().length > 0,
    );
    const outsideText = [...outside.childNodes].find(
      (node) => node.nodeType === Node.TEXT_NODE && node.textContent.trim().length > 0,
    );
    if (textNode === undefined || outsideText === undefined) {
      throw new Error("text zoom requires visible label text");
    }
    function textBox(node, element) {
      const range = document.createRange();
      range.selectNodeContents(node);
      const text = range.getBoundingClientRect();
      const box = element.getBoundingClientRect();
      let clipTop = box.top;
      let clipBottom = box.bottom;
      let clipLeft = box.left;
      let clipRight = box.right;
      let parent = element.parentElement;
      while (parent !== null && parent !== document.body) {
        const style = getComputedStyle(parent);
        const clipsX = style.overflowX === "hidden" || style.overflowX === "clip";
        const clipsY = style.overflowY === "hidden" || style.overflowY === "clip";
        if (clipsX || clipsY) {
          const parentBox = parent.getBoundingClientRect();
          if (clipsX) {
            clipLeft = Math.max(clipLeft, parentBox.left);
            clipRight = Math.min(clipRight, parentBox.right);
          }
          if (clipsY) {
            clipTop = Math.max(clipTop, parentBox.top);
            clipBottom = Math.min(clipBottom, parentBox.bottom);
          }
        }
        parent = parent.parentElement;
      }
      const visibleWidth = Math.max(
        0,
        Math.min(text.right, clipRight) - Math.max(text.left, clipLeft),
      );
      const visibleHeight = Math.max(
        0,
        Math.min(text.bottom, clipBottom) - Math.max(text.top, clipTop),
      );
      return {
        text: node.textContent.trim(),
        fontSize: Number.parseFloat(getComputedStyle(element).fontSize),
        fits:
          text.width > 0 &&
          text.height > 0 &&
          visibleWidth >= text.width - 1 &&
          visibleHeight >= text.height * 0.85,
      };
    }
    function reachable(element) {
      const box = element.getBoundingClientRect();
      const x = box.left + box.width / 2;
      const y = box.top + box.height / 2;
      const hit = document.elementFromPoint(x, y);
      return (
        box.width > 1 &&
        box.height > 1 &&
        x >= 0 &&
        y >= 0 &&
        x <= window.innerWidth &&
        y <= window.innerHeight &&
        (hit === element || element.contains(hit))
      );
    }
    window.scrollTo(0, 0);
    const labelPaint = textBox(textNode, label);
    const linkReachable = reachable(link);
    const profileReachable = reachable(profile);
    outside.scrollIntoView({ block: "center", inline: "nearest" });
    const outsidePaint = textBox(outsideText, outside);
    return {
      rootFont: Number.parseFloat(getComputedStyle(document.documentElement).fontSize),
      label: labelPaint,
      outside: outsidePaint,
      linkReachable,
      profileReachable,
      outsideReachable: reachable(outside),
    };
  });
}

export async function assertTextEnlargementAndZoom(page) {
  const resting = await readTextZoomTargets(page);
  assert.equal(resting.rootFont, 16, TEXT_ZOOM_SENTENCE);
  assert.equal(resting.label.fits, true, TEXT_ZOOM_SENTENCE);
  assert.equal(resting.outside.fits, true, TEXT_ZOOM_SENTENCE);
  assert.equal(resting.linkReachable, true, TEXT_ZOOM_SENTENCE);
  assert.equal(resting.profileReachable, true, TEXT_ZOOM_SENTENCE);
  assert.equal(resting.outsideReachable, true, TEXT_ZOOM_SENTENCE);
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "200%";
  });
  try {
    const enlarged = await readTextZoomTargets(page);
    assert.ok(enlarged.rootFont >= resting.rootFont * 1.9, TEXT_ZOOM_SENTENCE);
    assert.equal(enlarged.label.text, resting.label.text, TEXT_ZOOM_SENTENCE);
    assert.equal(enlarged.outside.text, resting.outside.text, TEXT_ZOOM_SENTENCE);
    assert.ok(enlarged.label.fontSize >= resting.label.fontSize * 1.9, TEXT_ZOOM_SENTENCE);
    assert.ok(enlarged.outside.fontSize >= resting.outside.fontSize * 1.9, TEXT_ZOOM_SENTENCE);
    assert.equal(enlarged.label.fits, true, TEXT_ZOOM_SENTENCE);
    assert.equal(enlarged.outside.fits, true, TEXT_ZOOM_SENTENCE);
    assert.equal(enlarged.linkReachable, true, TEXT_ZOOM_SENTENCE);
    assert.equal(enlarged.profileReachable, true, TEXT_ZOOM_SENTENCE);
    assert.equal(enlarged.outsideReachable, true, TEXT_ZOOM_SENTENCE);
  } finally {
    await page.evaluate(() => {
      document.documentElement.style.fontSize = "";
    });
  }
  await page.evaluate(() => {
    document.documentElement.style.zoom = "2";
  });
  try {
    const zoomed = await readTextZoomTargets(page);
    assert.equal(zoomed.label.text, resting.label.text, TEXT_ZOOM_SENTENCE);
    assert.equal(zoomed.outside.text, resting.outside.text, TEXT_ZOOM_SENTENCE);
    assert.equal(zoomed.label.fits, true, TEXT_ZOOM_SENTENCE);
    assert.equal(zoomed.outside.fits, true, TEXT_ZOOM_SENTENCE);
    assert.equal(zoomed.linkReachable, true, TEXT_ZOOM_SENTENCE);
    assert.equal(zoomed.profileReachable, true, TEXT_ZOOM_SENTENCE);
    assert.equal(zoomed.outsideReachable, true, TEXT_ZOOM_SENTENCE);
  } finally {
    await page.evaluate(() => {
      document.documentElement.style.zoom = "";
    });
  }
}

const DISCLOSURE_SENTENCE =
  "Use clearly labeled expandable sections with chevrons for longer details and secondary settings, supporting keyboard, pointer, and touch interaction.";
const PROGRESSIVE_DISCLOSURE_SENTENCE =
  "Use progressive disclosure to keep common tasks compact while making supporting details easy to find.";
const ROSTER_TOOLS_DETAIL =
  "Import reviewed Student Authentication Email, Course roster ID, and Course roster name rows.";

export async function assertDisclosureChevrons(page) {
  await page.evaluate((detail) => {
    const host = document.createElement("div");
    host.id = "disclosure-chevron";
    const details = document.createElement("details");
    details.className = "roster-tools";
    const summary = document.createElement("summary");
    summary.textContent = "Roster tools";
    const support = document.createElement("p");
    support.className = "field-help roster-tools-help";
    support.textContent = detail;
    details.append(summary, support);
    host.append(details);
    document.body.append(host);
  }, ROSTER_TOOLS_DETAIL);
  const summary = page.locator("#disclosure-chevron summary");
  const details = page.locator("#disclosure-chevron details");
  await summary.scrollIntoViewIfNeeded();
  const closed = await summary.evaluate((element) => {
    const chevron = getComputedStyle(element, "::before");
    return {
      label: element.textContent.replace(/\s+/g, " ").trim(),
      width: Number.parseFloat(chevron.width),
      height: Number.parseFloat(chevron.height),
      borderRight: chevron.borderRightStyle,
      borderBottom: chevron.borderBottomStyle,
      transform: chevron.transform,
    };
  });
  assert.equal(closed.label, "Roster tools", DISCLOSURE_SENTENCE);
  assert.ok(closed.width > 0 && closed.height > 0, DISCLOSURE_SENTENCE);
  assert.equal(closed.borderRight, "solid", DISCLOSURE_SENTENCE);
  assert.equal(closed.borderBottom, "solid", DISCLOSURE_SENTENCE);
  const compact = await details.evaluate((element) => {
    const support = element.querySelector(".roster-tools-help");
    const summary = element.querySelector("summary");
    return {
      open: element.open,
      height: element.offsetHeight,
      summaryHeight: summary.offsetHeight,
      summaryVisible: summary.getClientRects().length > 0,
      supportHeight: support.offsetHeight,
      supportVisible: support.getClientRects().length > 0,
    };
  });
  assert.equal(compact.open, false, PROGRESSIVE_DISCLOSURE_SENTENCE);
  assert.equal(compact.summaryVisible, true, PROGRESSIVE_DISCLOSURE_SENTENCE);
  assert.equal(
    compact.supportVisible,
    false,
    `${PROGRESSIVE_DISCLOSURE_SENTENCE} ${compact.supportHeight}`,
  );
  assert.equal(compact.supportHeight, 0, PROGRESSIVE_DISCLOSURE_SENTENCE);
  assert.ok(compact.height <= compact.summaryHeight + 1, PROGRESSIVE_DISCLOSURE_SENTENCE);
  await summary.focus();
  await page.keyboard.press("Enter");
  assert.equal(await details.evaluate((element) => element.open), true, DISCLOSURE_SENTENCE);
  const revealed = await details.evaluate((element) => {
    const support = element.querySelector(".roster-tools-help");
    const summary = element.querySelector("summary");
    return {
      height: element.offsetHeight,
      summaryVisible: summary.getClientRects().length > 0,
      supportText: support.textContent.replace(/\s+/g, " ").trim(),
      supportVisible: support.getClientRects().length > 0,
    };
  });
  assert.equal(revealed.summaryVisible, true, PROGRESSIVE_DISCLOSURE_SENTENCE);
  assert.equal(revealed.supportVisible, true, PROGRESSIVE_DISCLOSURE_SENTENCE);
  assert.equal(revealed.supportText, ROSTER_TOOLS_DETAIL, PROGRESSIVE_DISCLOSURE_SENTENCE);
  assert.ok(revealed.height > compact.height, PROGRESSIVE_DISCLOSURE_SENTENCE);
  const opened = await summary.evaluate(
    (element) => getComputedStyle(element, "::before").transform,
  );
  assert.notEqual(opened, closed.transform, DISCLOSURE_SENTENCE);
  await summary.click();
  assert.equal(await details.evaluate((element) => element.open), false, DISCLOSURE_SENTENCE);
  const box = await summary.boundingBox();
  if (box === null) throw new Error(DISCLOSURE_SENTENCE);
  const session = await page.context().newCDPSession(page);
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await session.send("Input.dispatchTouchEvent", {
    type: "touchStart",
    touchPoints: [{ x, y }],
  });
  await session.send("Input.dispatchTouchEvent", {
    type: "touchEnd",
    touchPoints: [],
  });
  await session.detach();
  assert.equal(await details.evaluate((element) => element.open), true, DISCLOSURE_SENTENCE);
}

const ESSENTIAL_SENTENCE =
  "Keep essential information, primary actions, and current status visible in the main interface.";

export async function assertEssentialInterface(page) {
  const visible = await page.evaluate(() => {
    window.scrollTo(0, 0);
    function shown(element) {
      const box = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return (
        style.display !== "none" &&
        style.visibility !== "hidden" &&
        box.width > 1 &&
        box.height > 1 &&
        box.top >= 0 &&
        box.left >= 0 &&
        box.bottom <= window.innerHeight &&
        box.right <= window.innerWidth
      );
    }
    const ribbon = document.querySelector(".ple-app-ribbon");
    if (!(ribbon instanceof HTMLElement))
      throw new Error("essential interface requires the Ribbon");
    const role = ribbon.querySelector(".ple-app-ribbon__user-role");
    const tabs = [
      ...ribbon.querySelectorAll('nav[aria-label="Ribbon tabs"] a.ple-app-ribbon__link'),
    ];
    const current = ribbon.querySelector(
      'nav[aria-label="Ribbon tabs"] a.ple-app-ribbon__link[aria-current="page"]',
    );
    if (!(role instanceof HTMLElement) || !(current instanceof HTMLElement) || tabs.length === 0) {
      throw new Error("essential interface requires a role, a current tab, and primary tabs");
    }
    const menu = ribbon.querySelector('[role="menu"]');
    return {
      role: role.textContent.replace(/\s+/g, " ").trim(),
      roleVisible: shown(role),
      tabLabels: tabs.map((tab) => tab.textContent.replace(/\s+/g, " ").trim()),
      tabsVisible: tabs.every((tab) => shown(tab)),
      current: current.textContent.replace(/\s+/g, " ").trim(),
      currentVisible: shown(current),
      menuOpen: menu instanceof HTMLElement && shown(menu),
    };
  });
  assert.equal(visible.role, "Instructor", ESSENTIAL_SENTENCE);
  assert.equal(visible.roleVisible, true, ESSENTIAL_SENTENCE);
  assert.ok(visible.tabLabels.length >= 2, ESSENTIAL_SENTENCE);
  assert.equal(visible.tabsVisible, true, ESSENTIAL_SENTENCE);
  assert.ok(visible.current.length > 0, ESSENTIAL_SENTENCE);
  assert.equal(visible.currentVisible, true, ESSENTIAL_SENTENCE);
  assert.ok(visible.tabLabels.includes(visible.current), ESSENTIAL_SENTENCE);
  assert.equal(visible.menuOpen, false, ESSENTIAL_SENTENCE);
}

const TOOLTIP_SENTENCE =
  "Use tooltips for brief supplementary explanations, available on hover and keyboard focus.";

async function readBriefTooltip(page) {
  return page.evaluate(() => {
    const button = document.querySelector("button.ple-app-ribbon__profile[title]");
    if (!(button instanceof HTMLButtonElement)) {
      throw new Error("profile tooltip control is missing");
    }
    const style = getComputedStyle(button, "::after");
    const host = button.getBoundingClientRect();
    const width = Number.parseFloat(style.width);
    const height = Number.parseFloat(style.height);
    const content = style.content;
    const shown = content !== "none" && content !== "normal" && width > 0 && height > 0;
    let clipped = false;
    let below = false;
    if (shown && style.top !== "auto" && style.right !== "auto") {
      const right = host.right - Number.parseFloat(style.right);
      const left = right - width;
      const top = host.top + Number.parseFloat(style.top);
      const box = { left, top, right, bottom: top + height, width, height };
      below = box.top >= host.bottom - 1;
      const viewportRight = window.innerWidth;
      const viewportBottom = window.innerHeight;
      const visibleWidth = Math.min(box.right, viewportRight) - Math.max(box.left, 0);
      const visibleHeight = Math.min(box.bottom, viewportBottom) - Math.max(box.top, 0);
      clipped = visibleWidth < box.width - 1 || visibleHeight < box.height - 1;
      const clips = /hidden|clip|scroll|auto/;
      for (let node = button.parentElement; node; node = node.parentElement) {
        const ancestorStyle = getComputedStyle(node);
        const ancestor = node.getBoundingClientRect();
        if (ancestor.width < 1 || ancestor.height < 1) continue;
        const overlapWidth =
          Math.min(box.right, ancestor.right) - Math.max(box.left, ancestor.left);
        const overlapHeight =
          Math.min(box.bottom, ancestor.bottom) - Math.max(box.top, ancestor.top);
        if (clips.test(ancestorStyle.overflowX) && overlapWidth < box.width - 1) clipped = true;
        if (clips.test(ancestorStyle.overflowY) && overlapHeight < box.height - 1) clipped = true;
      }
    }
    return {
      title: button.getAttribute("title"),
      expanded: button.getAttribute("aria-expanded"),
      content,
      shown,
      clipped,
      below,
    };
  });
}

export async function assertBriefTooltips(page) {
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.locator("#outside").hover();
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  });
  const resting = await readBriefTooltip(page);
  assert.equal(resting.title, "Profile", TOOLTIP_SENTENCE);
  assert.equal(resting.expanded, "false", TOOLTIP_SENTENCE);
  assert.equal(resting.shown, false, TOOLTIP_SENTENCE);

  const profile = page.getByRole("button", { name: "Profile", exact: true });
  await profile.hover();
  const hovered = await readBriefTooltip(page);
  assert.equal(hovered.content, JSON.stringify(hovered.title), TOOLTIP_SENTENCE);
  assert.equal(hovered.shown, true, TOOLTIP_SENTENCE);
  assert.equal(hovered.clipped, false, TOOLTIP_SENTENCE);
  assert.equal(hovered.below, true, TOOLTIP_SENTENCE);

  await page.locator("#outside").hover();
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  });
  const cleared = await readBriefTooltip(page);
  assert.equal(cleared.shown, false, TOOLTIP_SENTENCE);

  await profile.evaluate((element) => element.focus({ focusVisible: true }));
  const focused = await readBriefTooltip(page);
  assert.equal(focused.content, JSON.stringify(focused.title), TOOLTIP_SENTENCE);
  assert.equal(focused.shown, true, TOOLTIP_SENTENCE);
  assert.equal(focused.clipped, false, TOOLTIP_SENTENCE);
  assert.equal(focused.below, true, TOOLTIP_SENTENCE);
  assert.equal(
    await profile.evaluate((element) => element.matches(":focus-visible")),
    true,
    TOOLTIP_SENTENCE,
  );

  await page.locator("#outside").hover();
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  });
  const blurred = await readBriefTooltip(page);
  assert.equal(blurred.shown, false, TOOLTIP_SENTENCE);

  await profile.click();
  await page.getByRole("menu", { name: "Profile menu", exact: true }).waitFor({ state: "visible" });
  await profile.hover();
  const expanded = await readBriefTooltip(page);
  assert.equal(expanded.expanded, "true", TOOLTIP_SENTENCE);
  assert.equal(expanded.shown, false, TOOLTIP_SENTENCE);
  await page.keyboard.press("Escape");
}
