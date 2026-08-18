#!/usr/bin/env node
// Single front door for regenerating the committed UI screenshot corpus in docs/screenshots/.
//
// The corpus is produced by two capture implementations (see
// tests/playwright/ui_corpus_manifest.ts): a cheap mock-backed pipeline that needs no containers,
// and a live pipeline that needs a running Podman stack for real grading and renderer output.
// Forcing both into one execution path would make ordinary mock regeneration depend on containers
// even when only mock evidence changed, which is the cost that let the corpus go stale in the first
// place. This script stays a thin dispatcher: it always refreshes the mock set, and refreshes the
// live set only when a local stack is already reachable, so a maintainer with no stack running still
// gets full mock coverage instead of a hard failure.

import { execFile } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

import {
  bootstrapProvenanceFromHistory,
  reconcileCorpus,
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

  if (skipLive) {
    process.stdout.write("\n== live corpus skipped (--skip-live) ==\n");
  } else {
    process.stdout.write("\n== live corpus (requires a running local stack) ==\n");
    process.stdout.write(
      "Run `source source_me.sh && python3 local_stack.py status` to check first, or pass\n" +
        "--skip-live to regenerate only the mock-covered images.\n" +
        `Run directly: node tests/playwright/capture_docs_screenshots.mjs${verifyOnly ? " --verify-only" : ""}\n`,
    );
  }

  if (!verifyOnly) {
    await bootstrapProvenanceFromHistory(root);
  }
  process.stdout.write("\n== corpus reconciliation ==\n");
  process.stdout.write(`${await summarize(root)}\n`);
  const { missing, unowned } = await reconcileCorpus(root);
  if (missing.length > 0 || unowned.length > 0) {
    process.stdout.write(
      "FAIL: the corpus and its manifest disagree about which artifacts exist.\n",
    );
    process.exitCode = 1;
    return;
  }
  process.stdout.write("PASS: every committed artifact has exactly one manifest owner.\n");
}

await main();
