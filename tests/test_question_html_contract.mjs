import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeQuestionContentBlock } from "../src/api/decoders/question_response_format.ts";
import { createPleQuestionJsonClient } from "../src/features/ple_question_json_authoring/question_json_client.ts";
import {
  parsePleQuestionJsonSource,
  serializePleQuestionJsonSource,
} from "../src/features/ple_question_json_authoring/question_json_codec.ts";

const asset = {
  questionImageAssetId: "11111111-1111-4111-8111-111111111111",
  checksum: "a".repeat(64),
};

test("Native source preserves a large table prompt", () => {
  const rows = "<tr><td>Genetic marker</td><td>Observed allele</td></tr>".repeat(2_000);
  const source = {
    format: "pleQuestionJson",
    prompt: `<table>${rows}</table>`,
    response: {
      kind: "singleChoice",
      choices: [
        { id: "a", text: "First" },
        { id: "b", text: "Second" },
      ],
      correctChoice: "a",
    },
  };
  const parsed = parsePleQuestionJsonSource(JSON.stringify(source));
  assert.equal(parsed.prompt, source.prompt);
  assert.equal(
    parsePleQuestionJsonSource(serializePleQuestionJsonSource(parsed)).prompt,
    source.prompt,
  );
});

test("HTML content decoding preserves exact source and requires closed image bindings", () => {
  const block = {
    kind: "html",
    html: '<p style="color:red">Choose <img src="assets/cell %20.png" alt="cell"></p>',
    images: [{ sourceRelativeName: "assets/cell %20.png", questionImageAssetTuple: asset }],
  };
  assert.deepEqual(decodeQuestionContentBlock(block, "$", true), block);
  assert.deepEqual(
    decodeQuestionContentBlock({ kind: "text", markdown: "<b>literal</b>" }, "$", true),
    {
      kind: "text",
      markdown: "<b>literal</b>",
    },
  );
  const hostileBindings = [
    { sourceRelativeName: "https://example.invalid/cell.png", questionImageAssetTuple: asset },
    { sourceRelativeName: "../cell.png", questionImageAssetTuple: asset },
    {
      sourceRelativeName: "assets/cell.png",
      questionImageAssetTuple: { ...asset, url: "https://example.invalid" },
    },
    {
      sourceRelativeName: "assets/cell.png",
      questionImageAssetTuple: { ...asset, checksum: "bad" },
    },
  ];
  for (const binding of hostileBindings)
    assert.throws(
      () => decodeQuestionContentBlock({ ...block, images: [binding] }, "$", true),
      DecodeError,
    );
  assert.throws(
    () => decodeQuestionContentBlock({ ...block, authorScript: "alert(1)" }, "$", true),
    DecodeError,
  );
});

test("import image uploads bind exact names and original bytes without changing the Draft edit", async () => {
  const descriptor = { ...asset, mediaType: "image/png", intrinsicWidth: 2, intrinsicHeight: 3 };
  const requests = [];
  const client = createPleQuestionJsonClient({
    fetch: async (path, init) => {
      requests.push({ path, init });
      return new Response(JSON.stringify(descriptor), {
        status: 201,
        headers: { "content-type": "application/json" },
      });
    },
  });
  const image = new Blob([new Uint8Array([1, 2, 3])], { type: "image/png" });
  for (const name of ["assets/cell %20.png", "other/cell.png"])
    assert.deepEqual(
      await client.uploadQuestionImage("draft_1", image, "7", undefined, name),
      descriptor,
    );
  assert.equal(requests.length, 2);
  for (const [index, request] of requests.entries()) {
    assert.equal(request.path, "/api/authoring/drafts/draft_1/images");
    assert.equal(request.init.method, "POST");
    assert.equal(request.init.credentials, "same-origin");
    const headers = new Headers(request.init.headers);
    assert.equal(headers.get("if-match"), '"7"');
    assert.equal(headers.get("content-type"), "image/png");
    assert.equal(
      headers.get("x-ple-source-relative-name"),
      ["assets/cell %20.png", "other/cell.png"][index],
    );
    assert.deepEqual(
      new Uint8Array(await request.init.body.arrayBuffer()),
      new Uint8Array([1, 2, 3]),
    );
  }
  await assert.rejects(client.uploadQuestionImage("draft_1", image, "7", undefined, "../cell.png"));
  assert.equal(requests.length, 2);
});
