// test_question_renderer.mjs - permanent behavior checks for question-safe prompt projection.

import assert from "node:assert/strict";
import test from "node:test";

import {
  QuestionContentError,
  projectServerSanitizedMarkup,
  requireAccessibilityDescription,
  resolveSameOriginAssetUrl,
} from "../src/components/question_renderer.tsx";

const asset = {
  asset: "00000000-0000-0000-0000-000000000001",
  checksum: "a".repeat(64),
};

test("hostile supplied markup is refused before it can reach a DOM sink", () => {
  for (const hostile of [
    "<script>globalThis.wasExecuted = true</script>",
    '<p onclick="globalThis.wasExecuted = true">Click</p>',
    '<p style="background-image:url(https://attacker.example/pixel)">x</p>',
    '<meta http-equiv="refresh" content="0;url=https://attacker.example">',
    '<video poster="https://attacker.example/poster.png"></video>',
    '<a ping="https://attacker.example/ping">x</a>',
    '<svg><image href="https://attacker.example/figure"></image></svg>',
    '<img src="java&#x0a;script:alert(1)">',
    '<iframe src="https://untrusted.example"></iframe>',
    "<p><strong>malformed</p>",
  ]) {
    assert.throws(() => projectServerSanitizedMarkup(hostile), QuestionContentError, hostile);
  }
  assert.equal(globalThis.wasExecuted, undefined);
});

test("supplied markup is projected into an allowlisted tree with logical asset indirection", () => {
  const projection = projectServerSanitizedMarkup(
    '<p>Use the figure.</p><img data-asset-id="00000000-0000-0000-0000-000000000001">',
  );
  assert.equal(projection.tree[0]?.kind, "element");
  assert.throws(
    () => projectServerSanitizedMarkup('<img src="/api/assets/raw">'),
    QuestionContentError,
  );
});

test("asset URLs must be the resolver-derived logical asset route", () => {
  const priorLocation = globalThis.location;
  Object.defineProperty(globalThis, "location", {
    configurable: true,
    value: new URL("https://ple.example.test/runs/run-1"),
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
