#!/usr/bin/env node
// Report corpus ownership, provenance, and staleness for docs/screenshots/.
//
// This answers "is the committed visual evidence current?" by inspection, without re-running a
// capture pipeline. Ownership gaps fail the run because they mean the corpus and its manifest
// disagree. Staleness is reported because a browser source change does not always alter a surface.

import { fileURLToPath } from "node:url";
import path from "node:path";

import {
  bootstrapProvenanceFromHistory,
  reconcileCorpus,
  summarize,
} from "./ui_corpus_provenance.mjs";

function repositoryRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
}

function parseMode(argumentsToParse) {
  if (argumentsToParse.length === 0) return "report";
  if (argumentsToParse.length === 1 && argumentsToParse[0] === "--bootstrap-provenance") {
    return "bootstrap";
  }
  throw new Error("usage: node tests/playwright/verify_ui_corpus.mjs [--bootstrap-provenance]");
}

async function main() {
  const root = repositoryRoot();
  if (parseMode(process.argv.slice(2)) === "bootstrap") {
    await bootstrapProvenanceFromHistory(root);
    process.stdout.write("seeded provenance from image history\n");
  }
  const summary = await summarize(root);
  process.stdout.write(`${summary}\n`);
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
