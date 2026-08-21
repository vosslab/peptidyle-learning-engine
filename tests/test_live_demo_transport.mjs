// WP-PROF-LD2 same-origin browser transport and closed wire-contract tests.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeLiveDemoOwnershipStart,
  decodeLiveDemoOwnershipStatus,
  decodeSeededDemoAccounts,
} from "../src/api/live_demo.ts";
import { ApiRequestError, createHttpApiClient } from "../src/api/http_client.ts";
import { registerLiveDemoSysadminWithBrowser } from "../src/api/http_client/live_demo.ts";

const OWNERSHIP_PROOF = "A".repeat(43);
const CEREMONY_ID = "0198e000-0000-7000-8000-000000000701";

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

test("live-demo decoders accept only the closed persona and ceremony projections", () => {
  const accounts = {
    accounts: [
      { persona: "elenaInstructor", displayName: "Elena Instructor" },
      { persona: "maryStudent", displayName: "Mary Student" },
    ],
  };
  assert.deepEqual(decodeSeededDemoAccounts(accounts), accounts);
  assert.throws(
    () =>
      decodeSeededDemoAccounts({
        accounts: [
          accounts.accounts[0],
          { persona: "elenaInstructor", displayName: "Different account" },
        ],
      }),
    DecodeError,
  );
  assert.throws(
    () => decodeSeededDemoAccounts({ accounts: [{ persona: "sysadmin", displayName: "No" }] }),
    DecodeError,
  );
  assert.throws(
    () => decodeSeededDemoAccounts({ accounts: [{ ...accounts.accounts[0], role: "instructor" }] }),
    DecodeError,
  );
  assert.throws(
    () => decodeSeededDemoAccounts({ accounts: [{ ...accounts.accounts[0], userId: "private" }] }),
    DecodeError,
  );
  assert.throws(
    () => decodeSeededDemoAccounts({ accounts: [{ persona: "maryStudent", displayName: " " }] }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeSeededDemoAccounts({
        accounts: [{ persona: "maryStudent", displayName: "M".repeat(201) }],
      }),
    DecodeError,
  );
  assert.throws(
    () => decodeLiveDemoOwnershipStart({ ceremonyId: "not-a-uuid", options: {} }),
    DecodeError,
  );
  assert.throws(
    () => decodeLiveDemoOwnershipStart({ ceremonyId: CEREMONY_ID, options: [] }),
    DecodeError,
  );
  assert.deepEqual(decodeLiveDemoOwnershipStatus({ available: false }), { available: false });
  assert.throws(() => decodeLiveDemoOwnershipStatus({ available: "false" }), DecodeError);
});

test("live-demo requests stay same-origin, no-store, and carry only their closed request bodies", async () => {
  const requests = [];
  const client = createHttpApiClient({
    basePath: "/ple",
    fetch: createLiveDemoFetch(async (request) => {
      requests.push(request.clone());
      const url = new URL(request.url);
      const path = url.pathname.replace(/^\/ple/u, "");
      if (path === "/api/auth/live-demo/accounts" && request.method === "GET") {
        return jsonResponse({
          accounts: [{ persona: "maryStudent", displayName: "Mary Student" }],
        });
      }
      if (path === "/api/auth/live-demo/accounts") return jsonResponse({ authenticated: true });
      if (path === "/api/auth/live-demo/sysadmin-ownership" && request.method === "GET") {
        return jsonResponse({ available: true });
      }
      if (path === "/api/auth/live-demo/sysadmin-ownership") {
        return jsonResponse({ ceremonyId: CEREMONY_ID, options: {} });
      }
      return jsonResponse({ authenticated: true });
    }),
  });

  await client.listSeededDemoAccounts();
  await client.selectSeededDemoAccount("maryStudent");
  await client.getLiveDemoSysadminOwnershipStatus();
  await client.startLiveDemoSysadminOwnership(OWNERSHIP_PROOF);
  await client.completeLiveDemoSysadminOwnership(OWNERSHIP_PROOF, CEREMONY_ID, "Laptop passkey", {
    id: "credential",
    rawId: "credential",
    response: { attestationObject: "proof" },
    type: "public-key",
  });

  assert.equal(requests.length, 5);
  const paths = requests.map((request) => new URL(request.url).pathname);
  assert.deepEqual(paths, [
    "/ple/api/auth/live-demo/accounts",
    "/ple/api/auth/live-demo/accounts",
    "/ple/api/auth/live-demo/sysadmin-ownership",
    "/ple/api/auth/live-demo/sysadmin-ownership",
    "/ple/api/auth/live-demo/sysadmin-ownership/complete",
  ]);
  for (const request of requests) {
    assert.equal(request.credentials, "same-origin");
    assert.equal(request.cache, "no-store");
  }
  assert.equal(await requests[1].text(), '{"persona":"maryStudent"}');
  assert.equal(await requests[3].text(), `{"ownershipProof":"${OWNERSHIP_PROOF}"}`);
  assert.equal(
    await requests[4].text(),
    `{"ownershipProof":"${OWNERSHIP_PROOF}","ceremonyId":"${CEREMONY_ID}","label":"Laptop passkey","credential":{"id":"credential","rawId":"credential","response":{"attestationObject":"proof"},"type":"public-key"}}`,
  );
  assert.throws(() => client.selectSeededDemoAccount("unexpected"), DecodeError);
  assert.throws(() => client.startLiveDemoSysadminOwnership("not-a-proof"), DecodeError);
  assert.throws(
    () =>
      client.completeLiveDemoSysadminOwnership(OWNERSHIP_PROOF, CEREMONY_ID, "L".repeat(81), {
        id: "credential",
        rawId: "credential",
        response: { attestationObject: "proof" },
        type: "public-key",
      }),
    DecodeError,
  );
  assert.equal(requests.length, 5);
});

test("unavailable Sysadmin ownership remains an HTTP absence, not protocol success", async () => {
  const client = createHttpApiClient({
    fetch: createLiveDemoFetch(async () => jsonResponse({ unavailable: true }, 404)),
  });
  await assert.rejects(
    client.getLiveDemoSysadminOwnershipStatus(),
    (error) => error instanceof ApiRequestError && error.status === 404,
  );
});

test("live-demo ownership uses the shared browser registration conversion and canonical credential JSON", async () => {
  const navigatorDescriptor = Object.getOwnPropertyDescriptor(globalThis, "navigator");
  const credentialDescriptor = Object.getOwnPropertyDescriptor(globalThis, "PublicKeyCredential");
  const calls = [];

  class FakePublicKeyCredential {
    static parseCreationOptionsFromJSON(options) {
      calls.push({ kind: "parse", options });
      return { challenge: new Uint8Array([1]) };
    }

    toJSON() {
      return {
        id: "credential",
        rawId: "credential",
        response: { attestationObject: "attestation" },
        type: "public-key",
      };
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
        create: async (options) => {
          calls.push({ kind: "create", options });
          return new FakePublicKeyCredential();
        },
      },
    },
  });

  try {
    const completed = [];
    await registerLiveDemoSysadminWithBrowser(
      {
        listSeededDemoAccounts: async () => ({ accounts: [] }),
        selectSeededDemoAccount: async () => ({ authenticated: true }),
        getLiveDemoSysadminOwnershipStatus: async () => ({ available: true }),
        startLiveDemoSysadminOwnership: async (proof) => {
          assert.equal(proof, OWNERSHIP_PROOF);
          return { ceremonyId: CEREMONY_ID, options: { challenge: "one" } };
        },
        completeLiveDemoSysadminOwnership: async (proof, ceremonyId, label, credential) => {
          completed.push({ proof, ceremonyId, label, credential });
          return { authenticated: true };
        },
      },
      OWNERSHIP_PROOF,
      "Laptop passkey",
    );
    assert.deepEqual(
      calls.map((call) => call.kind),
      ["parse", "create"],
    );
    assert.deepEqual(completed, [
      {
        proof: OWNERSHIP_PROOF,
        ceremonyId: CEREMONY_ID,
        label: "Laptop passkey",
        credential: {
          id: "credential",
          rawId: "credential",
          response: { attestationObject: "attestation" },
          type: "public-key",
        },
      },
    ]);
  } finally {
    if (navigatorDescriptor === undefined) delete globalThis.navigator;
    else Object.defineProperty(globalThis, "navigator", navigatorDescriptor);
    if (credentialDescriptor === undefined) delete globalThis.PublicKeyCredential;
    else Object.defineProperty(globalThis, "PublicKeyCredential", credentialDescriptor);
  }
});
