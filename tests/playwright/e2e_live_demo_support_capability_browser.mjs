// Visible Sysadmin use of an Instructor-issued, exact-course support capability.

import { chromium } from "playwright";

const [port, capabilityId] = process.argv.slice(2);
if (
  !/^[0-9]+$/u.test(port ?? "") ||
  !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u.test(capabilityId ?? "")
) {
  throw new Error("expected the fixed HTTPS gateway port and issued support capability");
}

const origin = `https://localhost:${port}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Morgan Sysadmin" }).click();
  await page.waitForURL(`${origin}/`);
  await page
    .getByRole("navigation", { name: "Ribbon tabs" })
    .getByRole("link", { name: "Scoped Support" })
    .click();
  await page.waitForURL(`${origin}/sysadmin/support-roster`);
  await page.locator('[data-route-surface="supportRoster"]').waitFor();
  await page.getByRole("heading", { name: "Scoped course roster support" }).waitFor();

  const responsePromise = page.waitForResponse((response) =>
    response.url().endsWith(`/api/support-capabilities/${capabilityId}/course-roster`),
  );
  await page.getByLabel("Support capability ID").fill(capabilityId);
  await page.getByRole("button", { name: "Open scoped roster" }).click();
  const response = await responsePromise;
  if (!response.ok() || !response.headers()["cache-control"]?.includes("no-store")) {
    throw new Error("scoped roster response did not retain the protected transport boundary");
  }
  const projection = await response.json();
  if (!Array.isArray(projection) || projection.length !== 1) {
    throw new Error("scoped roster did not contain the registered minimal projection");
  }
  const entry = projection[0];
  if (
    entry === null ||
    typeof entry !== "object" ||
    Array.isArray(entry) ||
    Object.keys(entry).sort().join(",") !== "rosterEmail,rosterId,state" ||
    entry.rosterId !== "m17-support" ||
    entry.rosterEmail !== "m17.support@live-demo.invalid" ||
    entry.state !== "invitationPending"
  ) {
    throw new Error("scoped roster payload contains an unexpected projection");
  }
  await page.getByRole("heading", { name: "Course roster", exact: true }).waitFor();
  const roster = page.getByRole("region", { name: "Scoped course roster" });
  await roster.getByText("m17.support@live-demo.invalid").waitFor();
  await roster.getByText("Roster ID: m17-support").waitFor();
  await roster.getByText("State: Invitation pending").waitFor();
  if ((await page.getByLabel("Support capability ID").inputValue()) !== "") {
    throw new Error("scoped support capability remained visible after its operation completed");
  }
} finally {
  await context.close();
  await browser.close();
}
