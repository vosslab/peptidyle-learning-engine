// Instructor Account vetting, approval, and account-list behavior.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { chromium } from "playwright";

const instructorAccountsHtml = `<!doctype html>
<html><body><div id="instructor-accounts"></div></body></html>`;

async function loadInstructorAccountApprovalBundle() {
  const result = await build({
    bundle: true,
    entryPoints: [
      new URL("../support/instructor_account_approval_harness.tsx", import.meta.url).pathname,
    ],
    format: "iife",
    globalName: "InstructorAccountApprovalHarness",
    outfile: "instructor_account_approval_harness.js",
    platform: "browser",
    write: false,
    plugins: [
      solidPlugin({ solid: { generate: "dom", hydratable: false } }),
      {
        name: "ignore-css",
        setup(bundle) {
          bundle.onResolve({ filter: /\.css$/ }, (args) => ({
            path: args.path,
            namespace: "ignore-css",
          }));
          bundle.onLoad({ filter: /.*/, namespace: "ignore-css" }, () => ({
            contents: "export {}",
            loader: "js",
          }));
        },
      },
    ],
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined) throw new Error("Instructor Account bundle is missing JavaScript.");
  return Buffer.from(javascript.contents).toString("utf8");
}

async function submitInstructorAccount(page, bundle, mode) {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.setContent(instructorAccountsHtml, { waitUntil: "load" });
  await page.addScriptTag({ content: bundle });
  await page.evaluate((approvalMode) => {
    const target = document.getElementById("instructor-accounts");
    window.InstructorAccountApprovalHarness.mountInstructorAccountApproval(target, approvalMode);
  }, mode);
  await page.locator("#instructor-account-email").waitFor({ timeout: 8000 });
  await page.locator("#instructor-account-email").fill("ada@university.edu");
  await page.locator("#instructor-account-verified-display-name").fill("Ada Lovelace");
  await page.getByRole("button", { name: "Create Instructor Account", exact: true }).click();
  return pageErrors;
}

test("Sysadmins approve Instructors before they receive Instructor capabilities.", async () => {
  const bundle = await loadInstructorAccountApprovalBundle();
  const browser = await chromium.launch({ headless: true });
  try {
    const approved = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const approvedErrors = await submitInstructorAccount(approved, bundle, "approved");
    await approved
      .getByText("Instructor Account created.", { exact: true })
      .waitFor({ timeout: 8000 });
    const approvedRecord = await approved.evaluate(() => window.instructorAccountApproval);
    const approvedPosts = approvedRecord.requests.filter(
      (request) => request.method === "POST" && request.path !== "/api/instructor-accounts/find",
    );
    assert.deepEqual(
      approvedPosts.map((request) => request.path),
      ["/api/instructor-identity-vetting-decisions", "/api/instructor-accounts"],
    );
    assert.deepEqual(JSON.parse(approvedPosts[0].body), {
      normalizedEmail: "ada@university.edu",
      verifiedInstructorDisplayName: "Ada Lovelace",
    });
    assert.deepEqual(JSON.parse(approvedPosts[1].body), {
      normalizedEmail: "ada@university.edu",
      vettingDecisionId: approvedRecord.decisionId,
    });

    const rejected = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const rejectedErrors = await submitInstructorAccount(rejected, bundle, "rejected");
    await rejected
      .getByText(
        "The Instructor Account could not be created. Check the normalized email and try again.",
        { exact: true },
      )
      .waitFor({ timeout: 8000 });
    const rejectedRecord = await rejected.evaluate(() => window.instructorAccountApproval);
    const rejectedPosts = rejectedRecord.requests.filter(
      (request) => request.method === "POST" && request.path !== "/api/instructor-accounts/find",
    );
    assert.deepEqual(
      rejectedPosts.map((request) => request.path),
      ["/api/instructor-identity-vetting-decisions"],
    );
    assert.deepEqual(approvedErrors, []);
    assert.deepEqual(rejectedErrors, []);
  } finally {
    await browser.close();
  }
});

test("Sysadmins should be able to find users quickly by name or email.", async () => {
  const bundle = await loadInstructorAccountApprovalBundle();
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));
    await page.setContent(instructorAccountsHtml, { waitUntil: "load" });
    await page.addScriptTag({ content: bundle });
    await page.evaluate(() => {
      const target = document.getElementById("instructor-accounts");
      window.InstructorAccountApprovalHarness.mountInstructorAccountApproval(target, "approved");
    });
    await page.getByText("U7K3M2PA0", { exact: true }).waitFor({ timeout: 8000 });
    const list = page.getByRole("list", { name: "Instructor Accounts" });
    await page.locator("#instructor-account-find").fill("Ada Lovelace");
    await page.getByRole("button", { name: "Find Instructor Account", exact: true }).click();
    await page.getByText("U0000035E", { exact: true }).waitFor({ timeout: 8000 });
    await page.locator("#instructor-account-find").fill("ada@university.edu");
    await page.getByRole("button", { name: "Find Instructor Account", exact: true }).click();
    await page
      .getByText("Instructor Account list updated.", { exact: true })
      .waitFor({ timeout: 8000 });
    const record = await page.evaluate(() => window.instructorAccountApproval);
    const finds = record.requests.filter(
      (request) => request.method === "POST" && request.path === "/api/instructor-accounts/find",
    );
    assert.deepEqual(
      finds.map((request) => JSON.parse(request.body)),
      [
        { query: "", state: null, pageSize: 50, afterAccountId: null },
        { query: "Ada Lovelace", state: null, pageSize: 50, afterAccountId: null },
        { query: "ada@university.edu", state: null, pageSize: 50, afterAccountId: null },
      ],
    );
    const listed = await list.innerText();
    assert.match(listed, /U0000035E/);
    assert.doesNotMatch(listed, /ada@university\.edu/);
    assert.doesNotMatch(listed, /Ada Lovelace/);
    assert.equal(await page.locator("#instructor-account-find").inputValue(), "ada@university.edu");
    assert.deepEqual(pageErrors, []);
  } finally {
    await browser.close();
  }
});

test("User pages should clearly show role, account status, and other important administrative information.", async () => {
  const bundle = await loadInstructorAccountApprovalBundle();
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));
    await page.setContent(instructorAccountsHtml, { waitUntil: "load" });
    await page.addScriptTag({ content: bundle });
    await page.evaluate(() => {
      const target = document.getElementById("instructor-accounts");
      window.InstructorAccountApprovalHarness.mountInstructorAccountApproval(target, "approved");
    });
    const list = page.getByRole("list", { name: "Instructor Accounts" });
    const row = list.getByRole("listitem").filter({ hasText: "U7K3M2PA0" });
    await row.getByText("Role:", { exact: true }).waitFor({ timeout: 8000 });
    await row.getByText("Instructor", { exact: true }).waitFor({ timeout: 8000 });
    await row.getByText("State:", { exact: true }).waitFor({ timeout: 8000 });
    await row.getByText("Active", { exact: true }).waitFor({ timeout: 8000 });
    await row.getByText("Last successful sign-in:", { exact: true }).waitFor({ timeout: 8000 });
    await row
      .getByText("No successful sign-in recorded", { exact: true })
      .waitFor({ timeout: 8000 });
    const listed = await row.innerText();
    assert.match(listed, /U7K3M2PA0/);
    assert.doesNotMatch(listed, /ada@university\.edu/);
    assert.doesNotMatch(listed, /Ada Lovelace/);
    assert.deepEqual(pageErrors, []);
  } finally {
    await browser.close();
  }
});

test("Account lists should support searching, filtering, and scanning large numbers of users.", async () => {
  const bundle = await loadInstructorAccountApprovalBundle();
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));
    await page.setContent(instructorAccountsHtml, { waitUntil: "load" });
    await page.addScriptTag({ content: bundle });
    await page.evaluate(() => {
      const target = document.getElementById("instructor-accounts");
      window.InstructorAccountApprovalHarness.mountInstructorAccountApproval(target, "approved");
    });
    await page.getByText("U7K3M2PA0", { exact: true }).waitFor({ timeout: 8000 });
    const list = page.getByRole("list", { name: "Instructor Accounts" });
    await page.locator("#instructor-account-find").fill("Ada Lovelace");
    await page.getByRole("button", { name: "Find Instructor Account", exact: true }).click();
    await list.getByText("U0000035E", { exact: true }).waitFor({ timeout: 8000 });
    await page.locator("#instructor-account-state").selectOption("deactivated");
    await list.getByText("Deactivated", { exact: true }).waitFor({ timeout: 8000 });
    await page.locator("#instructor-account-page-size").selectOption("100");
    await page.getByRole("button", { name: "Next Instructor Accounts", exact: true }).click();
    await list.getByText("UABCDEFGM", { exact: true }).waitFor({ timeout: 8000 });
    const listed = await list.innerText();
    assert.match(listed, /UABCDEFGM/);
    assert.doesNotMatch(listed, /U0000035E/);
    assert.doesNotMatch(listed, /U7K3M2PA0/);
    assert.doesNotMatch(listed, /Ada Lovelace/);
    const record = await page.evaluate(() => window.instructorAccountApproval);
    const finds = record.requests.filter(
      (request) => request.method === "POST" && request.path === "/api/instructor-accounts/find",
    );
    assert.deepEqual(
      finds.map((request) => JSON.parse(request.body)),
      [
        { query: "", state: null, pageSize: 50, afterAccountId: null },
        { query: "Ada Lovelace", state: null, pageSize: 50, afterAccountId: null },
        { query: "Ada Lovelace", state: "deactivated", pageSize: 50, afterAccountId: null },
        { query: "Ada Lovelace", state: "deactivated", pageSize: 100, afterAccountId: null },
        { query: "Ada Lovelace", state: "deactivated", pageSize: 100, afterAccountId: "U0000035E" },
      ],
    );
    assert.deepEqual(pageErrors, []);
  } finally {
    await browser.close();
  }
});

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
