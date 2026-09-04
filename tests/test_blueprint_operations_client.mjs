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
  completed: { blueprint: "BP-4", revision: "1" },
};

function noStoreJson(value) {
  return new Response(JSON.stringify(value), {
    status: 200,
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("B2 client uses the one closed Blueprint-operation record", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push({ request, body: request.body === null ? null : await request.text() });
      const path = new URL(request.url).pathname;
      return noStoreJson(path.endsWith("/apply") ? completedResponse : previewResponse);
    },
  });

  const preview = await client.previewBlueprintOperation(previewRequest);
  const completed = await client.applyBlueprintOperation({
    request: previewRequest,
  });

  assert.equal(preview.operation, "fork_blueprint_course");
  assert.equal(completed.operation, "fork_blueprint_course");
  assert.equal(requests[0].request.url, "https://ple.example/api/blueprint-operations/preview");
  assert.equal(requests[1].request.url, "https://ple.example/api/blueprint-operations/apply");
  assert.equal(requests[1].request.cache, "no-store");
  assert.equal(requests[1].request.credentials, "same-origin");
  assert.deepEqual(JSON.parse(requests[1].body), { request: previewRequest });
});

test("B2 client rejects retired operations and malformed apply intents before transport", async () => {
  const client = createHttpApiClient({
    fetch: () => Promise.reject(new Error("transport must not run")),
  });
  assert.throws(
    () => client.previewBlueprintOperation({ operation: "fork_alpha", request: {} }),
    ApiProtocolError,
  );
  assert.throws(
    () =>
      client.previewBlueprintOperation({ operation: "adopt_blueprint_assignment", request: {} }),
    ApiProtocolError,
  );
  assert.throws(
    () =>
      client.previewBlueprintOperation({ operation: "instantiate_blueprint_course", request: {} }),
    ApiProtocolError,
  );
  assert.throws(
    () =>
      client.previewBlueprintOperation({ operation: "shift_course_instance_term", request: {} }),
    ApiProtocolError,
  );
  assert.throws(
    () => client.previewBlueprintOperation({ operation: "rollover_course_instance", request: {} }),
    ApiProtocolError,
  );
  assert.throws(
    () =>
      client.previewBlueprintOperation({
        operation: "controlled_update_blueprint_assignment",
        request: {},
      }),
    ApiProtocolError,
  );
  assert.throws(
    () =>
      client.previewBlueprintOperation({
        operation: "create_selected_blueprint_assignment",
        request: {},
      }),
    ApiProtocolError,
  );
  assert.throws(
    () => client.applyBlueprintOperation({ request: previewRequest, retry_token: "retired" }),
    ApiProtocolError,
  );
});

test("B2 client rejects a separate replay status in a Blueprint operation result", async () => {
  const client = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        noStoreJson({
          operation: "fork_blueprint_course",
          completed: { blueprint: "BP-4", revision: "1", replay: "applied" },
        }),
      ),
  });

  await assert.rejects(
    () => client.applyBlueprintOperation({ request: previewRequest }),
    /response\.completed\.replay must be a field allowed by this response contract/u,
  );
});
