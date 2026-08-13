// Adversarial behavior tests for the generic untrusted JSON decoder.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError, decodeDictionary, decodeField } from "../src/api/decoder.ts";

function decodeNumber(value, path) {
  if (typeof value !== "number") {
    throw new DecodeError(path, "a number");
  }
  return value;
}

test("dictionary decoding rejects prototype-control keys and creates no object prototype", () => {
  const hostilePayloads = [
    '{"__proto__":{"polluted":true}}',
    '{"constructor":{"polluted":true}}',
    '{"prototype":{"polluted":true}}',
  ];

  for (const payload of hostilePayloads) {
    const parsed = JSON.parse(payload);
    assert.throws(
      () => decodeDictionary(parsed, "$.parameters", decodeNumber),
      DecodeError,
      `dictionary must reject ${payload}`,
    );
  }

  const decoded = decodeDictionary({ alpha: 1 }, "$.parameters", decodeNumber);
  assert.equal(Object.getPrototypeOf(decoded), null);
  assert.equal(decoded.alpha, 1);
  assert.equal(Object.prototype.hasOwnProperty.call(decoded, "toString"), false);
  assert.equal({}.polluted, undefined);
});

test("field decoding refuses inherited values", () => {
  const inherited = Object.create({ authority: "attacker-controlled" });
  assert.throws(() => decodeField(inherited, "authority", "$"), DecodeError);
});
