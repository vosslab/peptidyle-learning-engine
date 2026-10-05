// Rendered geometry coverage for the shipped Student Course entry banner.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { chromium } from "playwright";

test("the shipped course entry banner keeps a 5:1 ratio as its width changes", async () => {
  const source = readFileSync(
    new URL("../../src/features/course_appearance/course_entry_banner.tsx", import.meta.url),
    "utf8",
  );
  const styles = source.match(/const COURSE_ENTRY_BANNER_STYLES = `([\s\S]*?)`;/);
  assert.ok(styles, "shipped banner styles");
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    for (const width of [1280, 800, 400]) {
      await page.setViewportSize({ width, height: 800 });
      await page.setContent(
        `<!doctype html><style>${styles[1]}</style><div class="course-entry-banner-frame"></div>`,
        { waitUntil: "load" },
      );
      const box = await page.locator(".course-entry-banner-frame").boundingBox();
      assert.ok(box);
      const ratio = box.width / box.height;
      assert.ok(Math.abs(ratio - 5) < 0.08, `${width}px ratio ${ratio}`);
      if (width === 1280) {
        assert.ok(box.width < width - 8, "wide viewport keeps the banner narrower than the page");
        const centered = Math.abs(box.x - (width - box.width) / 2);
        assert.ok(centered < 2, `banner is centered, offset ${centered}`);
      }
    }
  } finally {
    await browser.close();
  }
});
