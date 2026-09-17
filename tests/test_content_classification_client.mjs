// Stable browser contract for active authoring choices and retired discovery references.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeContentClassificationList } from "../src/api/decoders/content_classification.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";

const active = {
  uuid: "00000000-0000-0000-0000-000000000001",
  name: "Biology",
  isRetired: false,
};
const retired = {
  uuid: "00000000-0000-0000-0000-000000000002",
  name: "Historical Biology",
  isRetired: true,
};

function noStoreJson(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("classification decoding makes a retired Discipline explicit", () => {
  assert.deepEqual(
    decodeContentClassificationList({ disciplines: [active, retired] }, "disciplines"),
    [active, retired],
  );
  assert.throws(
    () =>
      decodeContentClassificationList(
        { disciplines: [{ ...active, isRetired: undefined }] },
        "disciplines",
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeContentClassificationList(
        { disciplines: [{ ...active, lifecycle: "active" }] },
        "disciplines",
      ),
    DecodeError,
  );
});

test("Discipline discovery and Sysadmin lifecycle use their closed same-origin paths", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      const path = new URL(request.url).pathname;
      if (path.endsWith("/discovery")) return noStoreJson({ disciplines: [active, retired] });
      if (path.endsWith("/disciplines") && request.method === "POST") {
        return noStoreJson(active, 201);
      }
      if (path.endsWith("/rename")) return noStoreJson({ ...active, name: "Life Sciences" });
      if (path.endsWith("/retire")) return noStoreJson(retired);
      if (path.endsWith("/restore")) return noStoreJson(active);
      return noStoreJson({ disciplines: [active] });
    },
  });

  assert.deepEqual(await client.listDisciplinesIncludingRetired(), [active, retired]);
  await client.createDiscipline(" Biology ");
  await client.renameDiscipline(active.uuid, "Life Sciences");
  await client.retireDiscipline(active.uuid);
  await client.restoreDiscipline(active.uuid);

  assert.deepEqual(
    requests.map((request) => [request.method, new URL(request.url).pathname]),
    [
      ["GET", "/api/content-classification/disciplines/discovery"],
      ["POST", "/api/content-classification/disciplines"],
      ["POST", `/api/content-classification/disciplines/${active.uuid}/rename`],
      ["POST", `/api/content-classification/disciplines/${active.uuid}/retire`],
      ["POST", `/api/content-classification/disciplines/${active.uuid}/restore`],
    ],
  );
  assert.deepEqual(await requests[1].json(), { name: "Biology" });
  assert.deepEqual(await requests[2].json(), { name: "Life Sciences" });
});
