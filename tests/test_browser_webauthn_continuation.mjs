import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  webAuthnContinuationAcknowledgementPathFromEnvironment,
  webAuthnContinuationFromEnvironment,
  webAuthnContinuationPathForProducerFromEnvironment,
} from "../tests/playwright/browser_suite_live_config.ts";
import {
  createVirtualAuthenticator,
  exportWebAuthnContinuation,
  importWebAuthnContinuationIntoSession,
  removeVirtualAuthenticator,
  writeWebAuthnContinuationAcknowledgement,
  writeWebAuthnContinuation,
} from "../tests/playwright/helper_live_demo.ts";

function continuation(overrides = {}) {
  return {
    version: 1,
    origin: "https://localhost:55123",
    rpId: "localhost",
    credentials: [
      {
        credentialId: "AA",
        isResidentCredential: true,
        rpId: "localhost",
        privateKey: "AQI",
        signCount: 0,
        userHandle: "Aw",
        backupEligibility: false,
        backupState: false,
      },
    ],
    ...overrides,
  };
}

function claimedScenarioInput(overrides = {}) {
  return {
    schemaVersion: 2,
    scenarioId: "auth_authorization",
    namespace: "bs1-0123456789ab-auth_authorization",
    baseUrl: "https://localhost:55123/",
    personas: ["morgan_sysadmin"],
    baselineReads: ["genetics_practice_course"],
    sysadminRequirement: "claimed",
    visibleObservation: "visible_passkey_entry",
    ...overrides,
  };
}

function parse(contents) {
  return webAuthnContinuationFromEnvironment(
    { PLE_BROWSER_SUITE_WEBAUTHN_CONTINUATION_FILE: "/private/continuation.json" },
    "https://localhost:55123",
    () => contents,
    () => {},
  );
}

function temporaryDirectory() {
  const directory = mkdtempSync(join(tmpdir(), "ple-webauthn-continuation-"));
  return {
    directory,
    cleanup() {
      rmSync(directory, { force: true, recursive: true });
    },
  };
}

test("WebAuthn continuation decoder accepts one canonical resident credential", () => {
  const parsed = parse(JSON.stringify(continuation()));
  assert.equal(parsed.origin, "https://localhost:55123");
  assert.equal(parsed.credentials.length, 1);
  assert.equal(parsed.credentials[0].credentialId, "AA");
});

test("WebAuthn continuation decoder rejects unsafe paths and adversarial private files", () => {
  assert.throws(() => webAuthnContinuationPathForProducerFromEnvironment({}));
  assert.throws(() =>
    webAuthnContinuationPathForProducerFromEnvironment({
      PLE_BROWSER_SUITE_WEBAUTHN_CONTINUATION_FILE: "relative.json",
    }),
  );
  assert.throws(() => webAuthnContinuationAcknowledgementPathFromEnvironment({}));
  assert.throws(() =>
    webAuthnContinuationAcknowledgementPathFromEnvironment({
      PLE_BROWSER_SUITE_WEBAUTHN_CONTINUATION_ACK_FILE: "relative.json",
    }),
  );
  assert.throws(() =>
    webAuthnContinuationPathForProducerFromEnvironment({
      PLE_BROWSER_SUITE_WEBAUTHN_CONTINUATION_FILE: " /private/continuation.json",
    }),
  );

  const temporary = temporaryDirectory();
  try {
    const input = join(temporary.directory, "continuation.json");
    const target = join(temporary.directory, "target.json");
    const environment = { PLE_BROWSER_SUITE_WEBAUTHN_CONTINUATION_FILE: input };
    assert.throws(() =>
      webAuthnContinuationFromEnvironment(environment, "https://localhost:55123"),
    );
    writeFileSync(target, JSON.stringify(continuation()), { encoding: "ascii", mode: 0o600 });
    symlinkSync(target, input);
    assert.throws(() =>
      webAuthnContinuationFromEnvironment(environment, "https://localhost:55123"),
    );
    rmSync(input);
    writeFileSync(input, JSON.stringify(continuation()), { encoding: "ascii", mode: 0o600 });
    chmodSync(input, 0o644);
    assert.throws(() =>
      webAuthnContinuationFromEnvironment(environment, "https://localhost:55123"),
    );
    chmodSync(input, 0o600);
    writeFileSync(input, "x".repeat(16_385), { encoding: "ascii", mode: 0o600 });
    assert.throws(() =>
      webAuthnContinuationFromEnvironment(environment, "https://localhost:55123"),
    );
  } finally {
    temporary.cleanup();
  }
});

test("WebAuthn continuation decoder rejects schema, canonical, origin, RP, and credential changes", () => {
  const base = continuation();
  const variants = [
    { ...base, unexpected: true },
    { ...base, origin: "https://localhost:55123/" },
    { ...base, origin: "https://localhost:55124" },
    { ...base, rpId: "example.test" },
    { ...base, credentials: [] },
    { ...base, credentials: [base.credentials[0], base.credentials[0]] },
    {
      ...base,
      credentials: [{ ...base.credentials[0], rpId: "example.test" }],
    },
    {
      ...base,
      credentials: [{ ...base.credentials[0], credentialId: "=" }],
    },
    {
      ...base,
      credentials: [
        { ...base.credentials[0], userHandle: Buffer.from("x".repeat(65)).toString("base64url") },
      ],
    },
    {
      ...base,
      credentials: [{ ...base.credentials[0], backupState: true }],
    },
    {
      ...base,
      credentials: [{ ...base.credentials[0], extra: true }],
    },
  ];
  for (const value of variants) assert.throws(() => parse(JSON.stringify(value)));
  assert.throws(() => parse(`${JSON.stringify(base)}\n`));
  assert.throws(() => parse(JSON.stringify({ origin: base.origin, ...base })));
});

test("WebAuthn continuation writer exclusively creates canonical mode-0600 evidence", () => {
  const temporary = temporaryDirectory();
  try {
    const path = join(temporary.directory, "continuation.json");
    writeWebAuthnContinuation(path, continuation());
    assert.equal(readFileSync(path, "ascii"), JSON.stringify(continuation()));
    assert.throws(() => writeWebAuthnContinuation(path, continuation()));
  } finally {
    temporary.cleanup();
  }
});

test("WebAuthn acknowledgement writer exclusively binds a claimed visible passkey entry", () => {
  const temporary = temporaryDirectory();
  try {
    const path = join(temporary.directory, "acknowledgement.json");
    const input = claimedScenarioInput();
    writeWebAuthnContinuationAcknowledgement(path, input);
    assert.equal(
      readFileSync(path, "ascii"),
      JSON.stringify({
        event: "visible_sysadmin_passkey_sign_in",
        namespace: input.namespace,
        origin: "https://localhost:55123",
        scenarioId: input.scenarioId,
        schemaVersion: 1,
      }),
    );
    assert.throws(() => writeWebAuthnContinuationAcknowledgement(path, input));
    assert.throws(() =>
      writeWebAuthnContinuationAcknowledgement(
        join(temporary.directory, "ordinary.json"),
        claimedScenarioInput({ sysadminRequirement: "not_required" }),
      ),
    );
  } finally {
    temporary.cleanup();
  }
});

class FakeCdpSession {
  constructor(authenticatorId, credentialResponse = undefined) {
    this.authenticatorId = authenticatorId;
    this.credentialResponse = credentialResponse;
    this.calls = [];
  }

  async send(method, params = undefined) {
    this.calls.push(params === undefined ? { method } : { method, params });
    if (method === "WebAuthn.addVirtualAuthenticator") {
      return { authenticatorId: this.authenticatorId };
    }
    if (method === "WebAuthn.getCredentials") return this.credentialResponse;
    if (method === "WebAuthn.addCredential") return {};
    return {};
  }
}

test("WebAuthn helper normalizes CDP binary fields and uses the exact export/import calls", async () => {
  const source = new FakeCdpSession("source-auth", {
    credentials: [
      {
        credentialId: "/w==",
        isResidentCredential: true,
        rpId: "localhost",
        privateKey: "AQI=",
        signCount: 9,
        userHandle: "Aw==",
        backupEligibility: false,
        backupState: false,
        userDisplayName: "Morgan Reyes",
        userName: "morgan.reyes@live-demo.ple.example",
      },
    ],
  });
  const sourceAuthenticator = await createVirtualAuthenticator(source);
  const exported = await exportWebAuthnContinuation(
    sourceAuthenticator,
    "https://localhost:55123",
    "localhost",
  );
  assert.deepEqual(exported, {
    version: 1,
    origin: "https://localhost:55123",
    rpId: "localhost",
    credentials: [
      {
        credentialId: "_w",
        isResidentCredential: true,
        rpId: "localhost",
        privateKey: "AQI",
        signCount: 9,
        userHandle: "Aw",
        backupEligibility: false,
        backupState: false,
      },
    ],
  });
  assert.deepEqual(source.calls, [
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
    { method: "WebAuthn.getCredentials", params: { authenticatorId: "source-auth" } },
  ]);

  const target = new FakeCdpSession("target-auth");
  await importWebAuthnContinuationIntoSession(target, exported);
  assert.deepEqual(target.calls, [
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
    {
      method: "WebAuthn.addCredential",
      params: {
        authenticatorId: "target-auth",
        credential: {
          credentialId: "/w==",
          isResidentCredential: true,
          rpId: "localhost",
          privateKey: "AQI=",
          signCount: 9,
          userHandle: "Aw==",
          backupEligibility: false,
          backupState: false,
        },
      },
    },
  ]);

  await removeVirtualAuthenticator(sourceAuthenticator);
  assert.deepEqual(source.calls.slice(-2), [
    {
      method: "WebAuthn.removeVirtualAuthenticator",
      params: { authenticatorId: "source-auth" },
    },
    { method: "WebAuthn.disable" },
  ]);
});

test("WebAuthn helper rejects a CDP record outside its closed resident-credential projection", async () => {
  const session = new FakeCdpSession("source-auth", {
    credentials: [
      {
        credentialId: "AA==",
        isResidentCredential: true,
        rpId: "localhost",
        privateKey: "AQI=",
        signCount: 0,
        userHandle: "Aw==",
        backupEligibility: false,
        backupState: false,
        userDisplayName: "Morgan Reyes",
        userName: "morgan.reyes@live-demo.ple.example",
        unexpected: true,
      },
    ],
  });
  const authenticator = await createVirtualAuthenticator(session);
  await assert.rejects(() =>
    exportWebAuthnContinuation(authenticator, "https://localhost:55123", "localhost"),
  );
});
