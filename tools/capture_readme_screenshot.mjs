// capture_readme_screenshot.mjs - capture the deterministic learner assignment overview for README.
// Rerun: node tools/capture_readme_screenshot.mjs /tmp/peptidyle_assignment_overview.png

import { spawn } from "node:child_process";
import { once } from "node:events";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const port = 4187;
const baseUrl = `http://127.0.0.1:${port}`;
const assignmentPath =
  "/courses/0198e000-0000-7000-8000-000000000014/assignments/0198e000-0000-7000-8000-000000000006";
const outputPath = process.argv[2] ?? "/tmp/peptidyle_assignment_overview.png";

async function waitForPreview() {
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      const response = await fetch(`${baseUrl}/`);
      if (response.ok) return;
    } catch {
      // The server is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Mock preview did not start at ${baseUrl}. Build dist/ before capturing.`);
}

const server = spawn("node", ["tools/mock_preview_server.mjs", String(port)], {
  cwd: repoRoot,
  stdio: "inherit",
});

try {
  await waitForPreview();
  const browser = await chromium.launch();
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));

  await page.goto(`${baseUrl}/`);
  await page.evaluate((nextPath) => {
    history.pushState({}, "", nextPath);
    dispatchEvent(new PopStateEvent("popstate"));
  }, assignmentPath);
  await page.getByRole("heading", { name: "Peptide bond mastery", exact: true }).waitFor();
  await page.getByRole("button", { name: "Start or resume practice" }).waitFor();
  if (errors.length > 0) throw new Error(errors.join("\n"));

  await page.screenshot({ path: outputPath });
  await context.close();
  await browser.close();
} finally {
  server.kill();
  await once(server, "exit");
}
