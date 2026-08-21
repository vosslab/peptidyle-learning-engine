// Selector contract: the built SPA shell must resolve its assets before its instructor-only boundary mounts.

import { expect, test } from "@playwright/test";

const DELIVERY_CHECK_PATH = "/instructor/courses/C-1/assignments/A-1/delivery-check";

test("a live-style deep link loads JavaScript and CSS from the gateway root", async ({ page }) => {
  const assetContentTypes = new Map<string, string>();
  page.on("response", (response) => {
    const pathname = new URL(response.url()).pathname;
    if (
      [
        "/main.js",
        "/main.css",
        "/style.css",
        "/wasm/ple_bridge.js",
        "/wasm/ple_bridge_bg.wasm",
      ].includes(pathname)
    ) {
      assetContentTypes.set(pathname, response.headers()["content-type"] ?? "");
    }
  });

  await page.goto(DELIVERY_CHECK_PATH);
  await expect(
    page.getByRole("heading", { name: "This page is available to instructors only" }),
  ).toBeVisible();
  expect(assetContentTypes.get("/main.js")).toContain("text/javascript");
  expect(assetContentTypes.get("/main.css")).toContain("text/css");
  expect(assetContentTypes.get("/style.css")).toContain("text/css");
  expect(assetContentTypes.get("/wasm/ple_bridge.js")).toContain("text/javascript");
  expect(assetContentTypes.get("/wasm/ple_bridge_bg.wasm")).toContain("application/wasm");
});
