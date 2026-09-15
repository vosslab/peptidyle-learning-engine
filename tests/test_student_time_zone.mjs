import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import {
  decodeProfileSettings,
  decodeUpdateAccountSettingsInput,
} from "../src/api/decoders/profile_settings.ts";

test("Account Settings uses one self-only no-store GET and PUT contract", async () => {
  const requests = [];
  const client = createHttpApiClient({
    basePath: "/live",
    fetch: async (input, init) => {
      requests.push({ input, init });
      const timeZone = init?.method === "PUT" ? "America/Los_Angeles" : "America/New_York";
      return new Response(JSON.stringify({ timeZone }), {
        headers: { "content-type": "application/json", "cache-control": "no-store" },
      });
    },
  });

  assert.deepEqual(await client.getAccountSettings(), {
    timeZone: "America/New_York",
  });
  assert.deepEqual(await client.updateAccountSettings({ timeZone: "America/Los_Angeles" }), {
    timeZone: "America/Los_Angeles",
  });
  assert.equal(requests[0].input, "/live/api/account/settings");
  assert.equal(requests[0].init?.method, undefined);
  assert.equal(requests[1].input, "/live/api/account/settings");
  assert.equal(requests[1].init.method, "PUT");
  assert.equal(requests[1].init.body, '{"timeZone":"America/Los_Angeles"}');
  assert.equal(requests[1].init.credentials, "same-origin");
  assert.equal(requests[1].init.cache, "no-store");
});

test("Account Settings rejects unsupported zones and client-supplied identity", async () => {
  assert.throws(
    () => decodeProfileSettings({ timeZone: "America/Chicago", accountId: "private" }),
    DecodeError,
  );
  const client = createHttpApiClient({
    fetch: async () =>
      new Response(JSON.stringify({ timeZone: "not/a-zone" }), {
        headers: { "content-type": "application/json", "cache-control": "no-store" },
      }),
  });
  await assert.rejects(client.getAccountSettings(), DecodeError);
  assert.throws(
    () => decodeUpdateAccountSettingsInput({ timeZone: "America/Chicago", accountId: "private" }),
    DecodeError,
  );
  await assert.rejects(client.updateAccountSettings({ timeZone: " America/Chicago" }), DecodeError);
});
