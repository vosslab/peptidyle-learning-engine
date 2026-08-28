// WP-INST-LD3 direct seeded-role browser transport and closed wire-contract tests.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeSeededDemoAccounts } from "../src/api/live_demo.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { authenticatePasskeyWithBrowser } from "../src/api/http_client/enrollment.ts";

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

test("live-demo decoders accept the closed five-persona projection", () => {
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

test("ordinary passkey sign-in converts browser WebAuthn JSON before completing", async () => {
  const navigatorDescriptor = Object.getOwnPropertyDescriptor(globalThis, "navigator");
  const credentialDescriptor = Object.getOwnPropertyDescriptor(globalThis, "PublicKeyCredential");
  const calls = [];
  class FakePublicKeyCredential {
    static parseRequestOptionsFromJSON(options) {
      calls.push(options);
      return { challenge: new Uint8Array([1]) };
    }
    constructor(response) {
      this.response = response;
    }
    toJSON() {
      return { id: "credential", rawId: "credential", response: this.response, type: "public-key" };
    }
  }
  Object.defineProperty(globalThis, "PublicKeyCredential", {
    configurable: true,
    value: FakePublicKeyCredential,
  });
  Object.defineProperty(globalThis, "navigator", {
    configurable: true,
    value: {
      credentials: {
        get: async (options) => {
          calls.push(options);
          return new FakePublicKeyCredential({ signature: "signature" });
        },
      },
    },
  });
  try {
    let completed;
    await authenticatePasskeyWithBrowser({
      startPasskeyAuthentication: async () => ({
        ceremonyId: "0198e000-0000-7000-8000-000000000701",
        options: { publicKey: { challenge: "wire" } },
      }),
      completePasskeyAuthentication: async (ceremonyId, credential) => {
        completed = { ceremonyId, credential };
        return { authenticated: true };
      },
    });
    assert.deepEqual(calls[0], { challenge: "wire" });
    assert.deepEqual(calls[1], {
      publicKey: { challenge: new Uint8Array([1]) },
      mediation: "required",
    });
    assert.deepEqual(completed, {
      ceremonyId: "0198e000-0000-7000-8000-000000000701",
      credential: {
        id: "credential",
        rawId: "credential",
        response: { signature: "signature" },
        type: "public-key",
      },
    });
  } finally {
    if (navigatorDescriptor === undefined) delete globalThis.navigator;
    else Object.defineProperty(globalThis, "navigator", navigatorDescriptor);
    if (credentialDescriptor === undefined) delete globalThis.PublicKeyCredential;
    else Object.defineProperty(globalThis, "PublicKeyCredential", credentialDescriptor);
  }
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
