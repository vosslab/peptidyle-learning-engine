// Production-browser proof for M12's eight native response controls.
// It uses only visible seeded sign-in and keyboard interaction.  M13 submission/grading is absent.

import { chromium } from "playwright";

const [port, course, assignment] = process.argv.slice(2);
if (!/^[0-9]+$/u.test(port ?? "") || !/^C-[1-9][0-9]{0,9}$/u.test(course ?? "") || !/^A-[1-9][0-9]{0,9}$/u.test(assignment ?? "")) {
  throw new Error("expected fixed gateway port and public C-/A- Assignment references");
}

const origin = `https://localhost:${port}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();
const forbiddenRequest = /\/submission(?:s|\/)|\/grade(?:s|\/)|\/feedback(?:s|\/)/iu;
let prohibitedNetwork = false;
let logicalAssetRequests = 0;
page.on("request", (request) => {
  const requestUrl = new URL(request.url());
  if (forbiddenRequest.test(requestUrl.pathname)) prohibitedNetwork = true;
  if (requestUrl.origin === origin && requestUrl.pathname.startsWith("/api/assets/")) {
    logicalAssetRequests += 1;
  }
});

async function ready(control) {
  await control.locator("[id$='-format-status']").getByText("Response format is ready.").waitFor();
}

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Mary Student" }).click();
  await page.goto(`${origin}/courses/${course}/assignments/${assignment}`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Start Assignment" }).focus();
  await page.keyboard.press("Enter");
  const controls = page.locator("section[data-route-surface='assignmentOverview'] article.question-presentation section.question-response-control");
  await controls.nth(7).waitFor({ state: "visible" });
  if (await controls.count() !== 8) throw new Error("M12 browser prerequisite did not issue exactly eight native controls");

  const controlWithLegend = (name) =>
    controls.filter({ has: page.getByRole("group", { name }) });
  const single = controlWithLegend("Choose your response").filter({ has: page.locator("input[type='radio']") });
  await single.locator("input").first().focus(); await page.keyboard.press("Space"); await ready(single);
  const multiple = controlWithLegend("Choose your response").filter({ has: page.locator("input[type='checkbox']") });
  await multiple.locator("input").first().focus(); await page.keyboard.press("Space"); await ready(multiple);
  const fill = controls.filter({ has: page.getByLabel("Short written response") });
  await fill.getByLabel("Short written response").focus(); await page.keyboard.type("one"); await ready(fill);
  const multi = controlWithLegend("Complete each blank");
  await multi.locator("input").first().focus(); await page.keyboard.type("one"); await page.keyboard.press("Tab"); await page.keyboard.type("two"); await ready(multi);
  const numeric = controls.filter({ has: page.getByLabel("Numeric response") });
  await numeric.getByLabel("Numeric response").focus(); await page.keyboard.type("1"); await ready(numeric);
  const matching = controlWithLegend("Match each prompt");
  await matching.getByRole("radio").nth(0).focus(); await page.keyboard.press("Space"); await matching.getByRole("radio").nth(3).focus(); await page.keyboard.press("Space"); await ready(matching);
  const ordering = controlWithLegend("Put the Ordering Items in order");
  await ordering.getByRole("button", { name: /Move Ordering Item 1 later/u }).focus(); await page.keyboard.press("Space"); await ready(ordering);
  const hotspot = controlWithLegend(/Choose the labeled image region/u);
  const hotspotImage = hotspot.locator("xpath=ancestor::article[contains(@class, 'question-presentation')]").locator("img");
  await hotspotImage.waitFor();
  const hotspotImageHandle = await hotspotImage.elementHandle();
  if (hotspotImageHandle === null) throw new Error("fixed HOTSPOT logical asset image was unavailable");
  await page.waitForFunction(
    (image) => image instanceof HTMLImageElement && image.complete && image.naturalWidth > 0,
    hotspotImageHandle,
  );
  if (!(await hotspotImage.evaluate((image) => image.complete && image.naturalWidth > 0))) {
    throw new Error("fixed HOTSPOT logical asset did not render");
  }
  await hotspot.locator("input").first().click();
  await hotspot.getByRole("button", { name: "Clear response" }).click();
  await hotspot.locator("input").first().focus(); await page.keyboard.press("Space"); await ready(hotspot);

  if (await page.getByRole("button", { name: /Submit answer|submit assignment|grade|feedback/i }).count() !== 0) throw new Error("M12 format-only controls exposed an M13 control");
  if (prohibitedNetwork) throw new Error("M12 format-only keyboard interaction made a submission/grading request");
  if (logicalAssetRequests === 0) throw new Error("fixed HOTSPOT did not request its logical same-origin asset route");
} finally {
  await context.close();
  await browser.close();
}
