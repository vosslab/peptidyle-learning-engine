import assert from "node:assert/strict";
import test from "node:test";

import {
  createVirtualAuthenticator,
  removeVirtualAuthenticator,
  type CdpProtocolSession,
} from "./playwright/helper_live_demo";

type CdpCall = { readonly method: string; readonly params?: object };

class FakeCdpSession implements CdpProtocolSession {
  readonly calls: CdpCall[] = [];

  constructor(
    private readonly addResult: unknown,
    private readonly removalError: Error | undefined = undefined,
  ) {}

  async send(method: string, params?: object): Promise<unknown> {
    await Promise.resolve();
    this.calls.push(params === undefined ? { method } : { method, params });
    if (method === "WebAuthn.addVirtualAuthenticator") return this.addResult;
    if (method === "WebAuthn.removeVirtualAuthenticator" && this.removalError !== undefined) {
      throw this.removalError;
    }
    return {};
  }
}

test("createVirtualAuthenticator accepts an ID and uses the exact CDP command sequence", async () => {
  const session = new FakeCdpSession({ authenticatorId: "virtual-authenticator" });

  const authenticator = await createVirtualAuthenticator(session);

  assert.deepEqual(authenticator, { session, id: "virtual-authenticator" });
  assert.deepEqual(session.calls, [
    { method: "WebAuthn.enable" },
    {
      method: "WebAuthn.addVirtualAuthenticator",
      params: {
        options: {
          protocol: "ctap2",
          transport: "internal",
          hasResidentKey: true,
          hasUserVerification: true,
          isUserVerified: true,
          automaticPresenceSimulation: true,
        },
      },
    },
  ]);
});

test("createVirtualAuthenticator rejects malformed CDP authenticator IDs", async () => {
  for (const result of [null, [], {}, { authenticatorId: "" }, { authenticatorId: 42 }]) {
    const session = new FakeCdpSession(result);
    await assert.rejects(
      createVirtualAuthenticator(session),
      /CDP did not return a virtual authenticator identifier/u,
    );
    assert.deepEqual(session.calls.slice(0, 1), [{ method: "WebAuthn.enable" }]);
  }
});

test("removeVirtualAuthenticator disables WebAuthn even when removal throws", async () => {
  const removalError = new Error("remove failed");
  const session = new FakeCdpSession({ authenticatorId: "virtual-authenticator" }, removalError);

  await assert.rejects(
    removeVirtualAuthenticator({ session, id: "virtual-authenticator" }),
    removalError,
  );

  assert.deepEqual(session.calls, [
    {
      method: "WebAuthn.removeVirtualAuthenticator",
      params: { authenticatorId: "virtual-authenticator" },
    },
    { method: "WebAuthn.disable" },
  ]);
});
