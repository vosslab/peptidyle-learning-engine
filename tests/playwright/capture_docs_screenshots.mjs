#!/usr/bin/env node
// Capture the canonical real-stack walkthrough screenshots into role-organized corpus paths.

import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

import { runCorpusCapture } from "./helper_corpus_capture_runner.mjs";

function repositoryRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
}

function parseArguments() {
  const suppliedArguments = process.argv.slice(2);
  const verifyOnly = suppliedArguments.includes("--verify-only");
  const walkthroughArguments = suppliedArguments.filter((argument) => argument !== "--verify-only");
  const prohibitedArguments = new Set([
    "--keep",
    "--instructor-setup-only",
    "--student-repeat-only",
  ]);
  for (const argument of walkthroughArguments) {
    if (argument === "--skip-build") {
      throw new Error("--skip-build is not accepted; omit it for AUTO reuse or pass --build");
    }
    if (prohibitedArguments.has(argument)) {
      throw new Error("documentation screenshots require the full cleanup-enabled walkthrough");
    }
  }
  const hasMasterSeed = walkthroughArguments.some(
    (argument) => argument === "--master-seed" || argument.startsWith("--master-seed="),
  );
  const seededArguments = hasMasterSeed
    ? walkthroughArguments
    : ["--master-seed", "42", ...walkthroughArguments];
  return { mode: verifyOnly ? "verify" : "refresh", walkthroughArguments: seededArguments };
}

function runWalkthrough(root, argumentsToPass, directory) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      "bash",
      [
        "tests/walkthrough/run_ui_walkthrough.sh",
        "--screenshot-directory",
        directory,
        ...argumentsToPass,
      ],
      {
        cwd: root,
        env: { ...process.env, PLE_UI_CORPUS_CAPTURE_OWNER: "live" },
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
      reject(new Error(`UI walkthrough failed with ${detail}`));
    });
  });
}

async function main() {
  const root = repositoryRoot();
  const { mode, walkthroughArguments } = parseArguments();
  await runCorpusCapture({
    root,
    owner: "live",
    pipeline: "live",
    mode,
    label: "documentation screenshots",
    runCapture: async (directory) => await runWalkthrough(root, walkthroughArguments, directory),
  });
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : "unknown capture failure";
  process.stderr.write(`FAIL: documentation screenshot capture: ${message}\n`);
  process.exitCode = 1;
});
