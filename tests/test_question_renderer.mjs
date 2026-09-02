// test_question_renderer.mjs - permanent behavior checks for question-safe prompt projection.

import assert from "node:assert/strict";
import test from "node:test";

import {
  QuestionContentError,
  requireAccessibilityDescription,
  resolveSameOriginAssetUrl,
} from "../src/components/question_renderer.tsx";

const asset = {
  asset: "00000000-0000-0000-0000-000000000001",
  checksum: "a".repeat(64),
};

test("asset URLs must be the resolver-derived logical asset route", () => {
  const priorLocation = globalThis.location;
  Object.defineProperty(globalThis, "location", {
    configurable: true,
    value: new URL("https://ple.example.test/assignment-attempts/R-1"),
  });
  try {
    assert.equal(
      resolveSameOriginAssetUrl(
        asset,
        () => new URL(`/api/assets/${asset.asset}`, globalThis.location.origin),
      ),
      `https://ple.example.test/api/assets/${asset.asset}`,
    );
    for (const resolver of [
      () => new URL("https://bucket.example.test/object"),
      () => new URL("/api/assets/another-asset", globalThis.location.origin),
      () => new URL(`/api/assets/${asset.asset}?raw-key=object`, globalThis.location.origin),
    ]) {
      assert.throws(() => resolveSameOriginAssetUrl(asset, resolver), QuestionContentError);
    }
  } finally {
    Object.defineProperty(globalThis, "location", { configurable: true, value: priorLocation });
  }
});

test("missing alternatives remain authoring errors", () => {
  assert.throws(() => requireAccessibilityDescription("   ", "image"), QuestionContentError);
  assert.equal(
    requireAccessibilityDescription("Residue contact map", "math"),
    "Residue contact map",
  );
});
