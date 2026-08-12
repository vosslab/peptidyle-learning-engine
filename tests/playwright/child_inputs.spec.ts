// child_inputs.spec.ts - private Node-child input boundary behavior.

import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { expect, test } from "@playwright/test";

import { childInputsFromArguments } from "../walkthrough/children/child_inputs";

test("arrangement accepts its manifest-only input and rejects former broad input", () => {
  const directory = mkdtempSync(join(tmpdir(), "ple-arrangement-input-"));
  chmodSync(directory, 0o700);
  const inputPath = join(directory, "walkthrough-inputs.json");
  const minimalInput = {
    schemaVersion: 1,
    stage: "arrangement",
    chapterOneManifestFile: join(directory, "local-chapter-one-pilot.json"),
  };
  try {
    writeFileSync(inputPath, JSON.stringify(minimalInput), { encoding: "ascii", mode: 0o600 });
    expect(childInputsFromArguments(["--inputs", inputPath], "arrangement")).toMatchObject(
      minimalInput,
    );

    writeFileSync(
      inputPath,
      JSON.stringify({ ...minimalInput, baseUrl: "http://127.0.0.1:8080" }),
      { encoding: "ascii", mode: 0o600 },
    );
    expect(() => childInputsFromArguments(["--inputs", inputPath], "arrangement")).toThrow(
      "walkthrough-inputs",
    );
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
