// Live Demo seeded-account browser transport and wire-contract tests.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeSeededDemoAccounts } from "../src/api/live_demo.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";

function jsonResponse(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: {
      "cache-control": "no-store",
      "content-type": "application/json; charset=utf-8",
    },
  });
}

function createLiveDemoFetch(handler) {
  return async (input, init) => {
    const request = new Request(new URL(input.toString(), "https://client.example.test"), init);
    return await handler(request);
  };
}

test("live-demo decoders accept the closed Seeded Demo Accounts response", () => {
  const accounts = {
    accounts: [
      { persona: "elenaInstructor", displayName: "Elena Instructor" },
      { persona: "maryStudent", displayName: "Mary Student" },
      { persona: "jackStudent", displayName: "Jack Student" },
      { persona: "averyStudent", displayName: "Avery Student" },
      { persona: "morganSysadmin", displayName: "Morgan Sysadmin" },
    ],
  };
  assert.deepEqual(decodeSeededDemoAccounts(accounts), accounts);
  assert.throws(
    () => decodeSeededDemoAccounts({ accounts: [{ persona: "sysadmin", displayName: "No" }] }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeSeededDemoAccounts({
        accounts: [{ ...accounts.accounts[0], role: "instructor" }],
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeSeededDemoAccounts({
        accounts: [{ persona: "maryStudent", displayName: "M".repeat(201) }],
      }),
    DecodeError,
  );
});

test("direct-role requests stay same-origin, no-store, and carry only persona", async () => {
  const requests = [];
  const client = createHttpApiClient({
    basePath: "/ple",
    fetch: createLiveDemoFetch(async (request) => {
      requests.push(request.clone());
      const path = new URL(request.url).pathname.replace(/^\/ple/u, "");
      if (path === "/api/auth/live-demo/accounts" && request.method === "GET") {
        return jsonResponse({
          accounts: [{ persona: "morganSysadmin", displayName: "Morgan Sysadmin" }],
        });
      }
      return jsonResponse({ authenticated: true });
    }),
  });

  await client.listSeededDemoAccounts();
  await client.selectSeededDemoAccount("morganSysadmin");

  assert.equal(requests.length, 2);
  assert.deepEqual(
    requests.map((request) => new URL(request.url).pathname),
    ["/ple/api/auth/live-demo/accounts", "/ple/api/auth/live-demo/accounts"],
  );
  for (const request of requests) {
    assert.equal(request.credentials, "same-origin");
    assert.equal(request.cache, "no-store");
  }
  assert.equal(await requests[1].text(), '{"persona":"morganSysadmin"}');
  assert.throws(() => client.selectSeededDemoAccount("unexpected"), DecodeError);
});
