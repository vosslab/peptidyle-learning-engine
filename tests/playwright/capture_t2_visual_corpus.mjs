#!/usr/bin/env node
// Refresh the bounded WP-PROF-T2 instructor visual corpus through the shared lifecycle.

import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

import { runCorpusCapture } from "./helper_corpus_capture_runner.mjs";

function repositoryRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
}

function captureMode() {
  const argumentsToParse = process.argv.slice(2);
  if (argumentsToParse.length === 0) return "refresh";
  if (argumentsToParse.length === 1 && argumentsToParse[0] === "--verify-only") return "verify";
  throw new Error("usage: node tests/playwright/capture_t2_visual_corpus.mjs [--verify-only]");
}

function runCapture(root, directory) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      "npx",
      ["playwright", "test", "tests/playwright/t2_visual_corpus.spec.ts", "--workers=1"],
      {
        cwd: root,
        env: {
          ...process.env,
          PLE_INSTRUCTOR_PAGE_VISUALS_DIR: directory,
          PLE_UI_CORPUS_CAPTURE_OWNER: "t2Mock",
        },
        stdio: "inherit",
      },
    );
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) {
        resolve();
        return;
      }
      const detail = signal === null ? `exit status ${code}` : `signal ${signal}`;
      reject(new Error(`Playwright failed with ${detail}`));
    });
  });
}

async function main() {
  const root = repositoryRoot();
  await runCorpusCapture({
    root,
    owner: "t2Mock",
    pipeline: "mock",
    mode: captureMode(),
    label: "WP-PROF-T2 teaching-operations visuals",
    runCapture: async (directory) => await runCapture(root, directory),
  });
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : "unknown capture failure";
  process.stderr.write(`FAIL: WP-PROF-T2 visual capture: ${message}\n`);
  process.exitCode = 1;
});
