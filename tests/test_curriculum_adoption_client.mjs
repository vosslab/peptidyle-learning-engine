import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError } from "../src/api/http_client/error.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";

const previewRequest = {
  operation: "fork_blueprint_course",
  request: {
    source: { reference: "BP-3", revision: "2" },
    replacements: [],
  },
};

const previewResponse = {
  operation: "fork_blueprint_course",
  preview: {
    source: { reference: "BP-3", revision: "2" },
    replacements: [],
    eligibility: { kind: "eligible" },
  },
};

const completedResponse = {
  operation: "fork_blueprint_course",
  completed: { blueprint: "BP-4", revision: "1", replay: "applied" },
};

function noStoreJson(value) {
  return new Response(JSON.stringify(value), {
    status: 200,
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("B2 client uses the one closed Blueprint Course adoption envelope", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push({ request, body: request.body === null ? null : await request.text() });
      const path = new URL(request.url).pathname;
      return noStoreJson(path.endsWith("/apply") ? completedResponse : previewResponse);
    },
  });

  const preview = await client.previewCurriculumAdoption(previewRequest);
  const completed = await client.applyCurriculumAdoption({
    request: previewRequest,
    idempotency_key: "fork-2026",
  });

  assert.equal(preview.operation, "fork_blueprint_course");
  assert.equal(completed.operation, "fork_blueprint_course");
  assert.equal(requests[0].request.url, "https://ple.example/api/curriculum-adoption/preview");
  assert.equal(requests[1].request.url, "https://ple.example/api/curriculum-adoption/apply");
  assert.equal(requests[1].request.cache, "no-store");
  assert.equal(requests[1].request.credentials, "same-origin");
  assert.deepEqual(JSON.parse(requests[1].body), {
    request: previewRequest,
    idempotency_key: "fork-2026",
  });
});

test("B2 client rejects retired operations and malformed apply intents before transport", async () => {
  const client = createHttpApiClient({
    fetch: () => Promise.reject(new Error("transport must not run")),
  });
  assert.throws(
    () => client.previewCurriculumAdoption({ operation: "fork_alpha", request: {} }),
    ApiProtocolError,
  );
  assert.throws(
    () =>
      client.applyCurriculumAdoption({ request: previewRequest, idempotency_key: "invalid key" }),
    ApiProtocolError,
  );
});
