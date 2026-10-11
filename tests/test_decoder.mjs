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
  const hostilePayloads = ['{"__proto__":1}', '{"constructor":1}', '{"prototype":1}'];

  for (const payload of hostilePayloads) {
    assert.throws(
      () => decodeDictionary(JSON.parse(payload), "$.parameters", decodeNumber),
      DecodeError,
    );
  }

  const decoded = decodeDictionary({ alpha: 1 }, "$.parameters", decodeNumber);
  assert.equal(Object.getPrototypeOf(decoded), null);
  assert.equal(decoded.alpha, 1);
});

test("field decoding refuses inherited values", () => {
  const inherited = Object.create({ authority: "attacker-controlled" });
  assert.throws(() => decodeField(inherited, "authority", "$"), DecodeError);
});
