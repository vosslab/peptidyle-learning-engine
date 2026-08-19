#!/usr/bin/env node
// Single front door for regenerating the committed UI screenshot corpus in docs/screenshots/.
//
// The corpus has narrow instructor-mock, student-mock, and live command owners (see
// tests/playwright/ui_corpus_manifest.ts). Both deterministic built-app mock sets always run. The
// live set runs unless the maintainer explicitly passes --skip-live, because it needs the Podman
// stack for real grading and renderer output. Each launcher delegates file lifecycle to the same
// shared runner while retaining only its command-specific execution.

import { execFile } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

import {
  reconcileCorpus,
  requiredVisualEvidenceIssues,
  summarize,
} from "../tests/playwright/ui_corpus_provenance.mjs";

function repositoryRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
}

function parseArguments(argumentsToParse) {
  const skipLive = argumentsToParse.includes("--skip-live");
  const verifyOnly = argumentsToParse.includes("--verify-only");
  const unknown = argumentsToParse.filter(
    (argument) => argument !== "--skip-live" && argument !== "--verify-only",
  );
  if (unknown.length > 0) {
    throw new Error(
      `usage: node devel/refresh_ui_corpus.mjs [--skip-live] [--verify-only]\nunknown argument: ${unknown.join(" ")}`,
    );
  }
  return { skipLive, verifyOnly };
}

function run(root, command, args) {
  return new Promise((resolve, reject) => {
    const child = execFile(command, args, { cwd: root }, (error) => {
      if (error) reject(error);
      else resolve();
    });
    child.stdout?.pipe(process.stdout);
    child.stderr?.pipe(process.stderr);
  });
}

async function main() {
  const root = repositoryRoot();
  const { skipLive, verifyOnly } = parseArguments(process.argv.slice(2));

  const mockArguments = verifyOnly ? ["--verify-only"] : [];
  process.stdout.write("== mock-backed corpus (no containers needed) ==\n");
  await run(root, "node", [
    "tests/playwright/capture_instructor_page_visuals.mjs",
    ...mockArguments,
  ]);

  process.stdout.write("\n== student/access corpus (no containers needed) ==\n");
  await run(root, "node", [
    "tests/playwright/capture_student_access_visuals.mjs",
    ...mockArguments,
  ]);

  if (skipLive) {
    process.stdout.write("\n== live corpus skipped (--skip-live) ==\n");
  } else {
    process.stdout.write("\n== live corpus (requires a running local stack) ==\n");
    const liveArguments = verifyOnly ? ["--verify-only"] : [];
    await run(root, "node", ["tests/playwright/capture_docs_screenshots.mjs", ...liveArguments]);
  }
  process.stdout.write("\n== corpus reconciliation ==\n");
  process.stdout.write(`${await summarize(root)}\n`);
  const { missing, unowned } = await reconcileCorpus(root);
  const requiredIssues = await requiredVisualEvidenceIssues(root);
  for (const issue of requiredIssues) {
    process.stdout.write(`  required visual evidence failure: ${issue}\n`);
  }
  if (missing.length > 0 || unowned.length > 0 || requiredIssues.length > 0) {
    process.stdout.write("FAIL: corpus ownership or required visual evidence is incomplete.\n");
    process.exitCode = 1;
    return;
  }
  process.stdout.write("PASS: every committed artifact has exactly one manifest owner.\n");
}

await main();
