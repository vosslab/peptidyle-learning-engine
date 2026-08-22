// CDP-only WebAuthn support for the private live-demo browser journey.

import { lstatSync, writeFileSync } from "node:fs";

import type { Page } from "@playwright/test";

import type {
  BrowserScenarioInputV1,
  WebAuthnContinuation,
  WebAuthnCredential,
} from "./browser_suite_live_config";

export interface CdpProtocolSession {
  send(method: string, params?: object): Promise<unknown>;
}

export interface VirtualAuthenticator {
  readonly session: CdpProtocolSession;
  readonly id: string;
}

type CdpCredentialRecord = {
  readonly credentialId: string;
  readonly isResidentCredential: boolean;
  readonly rpId: "localhost";
  readonly privateKey: string;
  readonly signCount: number;
  readonly userHandle: string;
  readonly backupEligibility: boolean;
  readonly backupState: boolean;
  readonly userDisplayName: string;
  readonly userName: string;
};

type CdpCredentialInput = {
  readonly credentialId: string;
  readonly isResidentCredential: true;
  readonly rpId: "localhost";
  readonly privateKey: string;
  readonly signCount: number;
  readonly userHandle: string;
  readonly backupEligibility: boolean;
  readonly backupState: boolean;
};

const VIRTUAL_AUTHENTICATOR_OPTIONS = {
  protocol: "ctap2",
  transport: "internal",
  hasResidentKey: true,
  hasUserVerification: true,
  isUserVerified: true,
  automaticPresenceSimulation: true,
} as const;

/** Creates the browser's normal virtual security key; no application transport is replaced. */
export async function installVirtualAuthenticator(page: Page): Promise<VirtualAuthenticator> {
  const session = await page.context().newCDPSession(page);
  return await createVirtualAuthenticator(session);
}

/** Creates one WebAuthn virtual authenticator through a typed CDP session. */
export async function createVirtualAuthenticator(
  session: CdpProtocolSession,
): Promise<VirtualAuthenticator> {
  await session.send("WebAuthn.enable");
  const result = await session.send("WebAuthn.addVirtualAuthenticator", {
    options: VIRTUAL_AUTHENTICATOR_OPTIONS,
  });
  if (
    !isRecord(result) ||
    typeof result.authenticatorId !== "string" ||
    result.authenticatorId === ""
  ) {
    throw new Error("CDP did not return a virtual authenticator identifier");
  }
  return { session, id: result.authenticatorId };
}

/** Remove and disable the test-owned WebAuthn domain before closing its browser context. */
export async function removeVirtualAuthenticator(
  authenticator: VirtualAuthenticator,
): Promise<void> {
  try {
    await authenticator.session.send("WebAuthn.removeVirtualAuthenticator", {
      authenticatorId: authenticator.id,
    });
  } finally {
    await authenticator.session.send("WebAuthn.disable");
  }
}

/** Exports the one resident credential in canonical continuation form after visible reauthentication. */
export async function exportWebAuthnContinuation(
  authenticator: VirtualAuthenticator,
  origin: string,
  rpId: "localhost",
): Promise<WebAuthnContinuation> {
  const response = await authenticator.session.send("WebAuthn.getCredentials", {
    authenticatorId: authenticator.id,
  });
  const credentials = requireCdpCredentialResponse(response, rpId);
  const credential = credentials[0];
  if (credential === undefined) throw new Error("CDP did not return a resident credential");
  return {
    version: 1,
    origin,
    rpId,
    credentials: [continuationCredentialFromCdp(credential)],
  };
}

/** Imports an already decoded continuation into a fresh browser context's virtual authenticator. */
export async function importWebAuthnContinuation(
  page: Page,
  continuation: WebAuthnContinuation,
): Promise<VirtualAuthenticator> {
  const session = await page.context().newCDPSession(page);
  return await importWebAuthnContinuationIntoSession(session, continuation);
}

/** Imports a strict continuation through the exact CDP calls used by a fresh authenticator. */
export async function importWebAuthnContinuationIntoSession(
  session: CdpProtocolSession,
  continuation: WebAuthnContinuation,
): Promise<VirtualAuthenticator> {
  const authenticator = await createVirtualAuthenticator(session);
  const credential = continuation.credentials[0];
  if (credential === undefined) throw new Error("WebAuthn continuation has no credential");
  await authenticator.session.send("WebAuthn.addCredential", {
    authenticatorId: authenticator.id,
    credential: cdpCredentialFromContinuation(credential),
  });
  return authenticator;
}

/** Exclusively creates the owner-selected private continuation after the visible first-claim proof. */
export function writeWebAuthnContinuation(path: string, continuation: WebAuthnContinuation): void {
  const contents = JSON.stringify(continuation);
  // ASVS 5.3.2 and 6.7.1: the owner-selected capability has one restrictive, non-replaceable write.
  writeExclusivePrivateAscii(path, contents, "WebAuthn continuation");
}

/** Acknowledges the claimed child's visible passkey entry without exposing authenticator material. */
export function writeWebAuthnContinuationAcknowledgement(
  path: string,
  scenarioInput: BrowserScenarioInputV1,
): void {
  if (scenarioInput.sysadminRequirement !== "claimed") {
    throw new Error("WebAuthn continuation acknowledgement belongs only to claimed scenarios");
  }
  const contents = JSON.stringify({
    event: "visible_sysadmin_passkey_sign_in",
    namespace: scenarioInput.namespace,
    origin: new URL(scenarioInput.baseUrl).origin,
    scenarioId: scenarioInput.scenarioId,
    schemaVersion: 1,
  });
  // ASVS 2.3.1: bind the successful child to its required visible credential transition.
  writeExclusivePrivateAscii(path, contents, "WebAuthn continuation acknowledgement");
}

function writeExclusivePrivateAscii(path: string, contents: string, description: string): void {
  writeFileSync(path, contents, { encoding: "ascii", flag: "wx", mode: 0o600 });
  const metadata = lstatSync(path);
  if (!metadata.isFile() || metadata.isSymbolicLink() || (metadata.mode & 0o777) !== 0o600) {
    throw new Error(`${description} was not created as a private regular file`);
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function requireCdpCredentialResponse(
  response: unknown,
  rpId: "localhost",
): readonly CdpCredentialRecord[] {
  if (
    !isRecord(response) ||
    Object.keys(response).length !== 1 ||
    !Array.isArray(response.credentials)
  ) {
    throw new Error("CDP returned an invalid WebAuthn credential response");
  }
  if (response.credentials.length !== 1) {
    throw new Error("CDP must export exactly one WebAuthn credential");
  }
  return response.credentials.map((value) => requireCdpCredential(value, rpId));
}

function requireCdpCredential(value: unknown, rpId: "localhost"): CdpCredentialRecord {
  if (!isRecord(value)) throw new Error("CDP returned an invalid WebAuthn credential");
  const expectedKeys = [
    "backupEligibility",
    "backupState",
    "credentialId",
    "isResidentCredential",
    "privateKey",
    "rpId",
    "signCount",
    "userDisplayName",
    "userHandle",
    "userName",
  ];
  if (Object.keys(value).sort().join(",") !== expectedKeys.join(",")) {
    throw new Error("CDP returned an unexpected WebAuthn credential record");
  }
  if (
    typeof value.credentialId !== "string" ||
    value.isResidentCredential !== true ||
    value.rpId !== rpId ||
    typeof value.privateKey !== "string" ||
    !isCount(value.signCount) ||
    typeof value.userHandle !== "string" ||
    typeof value.backupEligibility !== "boolean" ||
    typeof value.backupState !== "boolean" ||
    !isBoundedText(value.userDisplayName) ||
    !isBoundedText(value.userName) ||
    (value.backupState && !value.backupEligibility)
  ) {
    throw new Error("CDP returned an invalid WebAuthn credential record");
  }
  requireCdpBinary(value.credentialId, 1024);
  requireCdpBinary(value.privateKey, 4096);
  requireCdpBinary(value.userHandle, 64);
  return {
    credentialId: value.credentialId,
    isResidentCredential: true,
    rpId,
    privateKey: value.privateKey,
    signCount: value.signCount,
    userHandle: value.userHandle,
    backupEligibility: value.backupEligibility,
    backupState: value.backupState,
    userDisplayName: value.userDisplayName,
    userName: value.userName,
  };
}

function continuationCredentialFromCdp(credential: CdpCredentialRecord): WebAuthnCredential {
  // Chrome exposes user labels on export, but addCredential accepts this narrower credential-only form.
  return {
    credentialId: canonicalBase64urlFromCdp(credential.credentialId, 1024),
    isResidentCredential: true,
    rpId: credential.rpId,
    privateKey: canonicalBase64urlFromCdp(credential.privateKey, 4096),
    signCount: credential.signCount,
    userHandle: canonicalBase64urlFromCdp(credential.userHandle, 64),
    backupEligibility: credential.backupEligibility,
    backupState: credential.backupState,
  };
}

function cdpCredentialFromContinuation(credential: WebAuthnCredential): CdpCredentialInput {
  return {
    credentialId: canonicalCdpBase64FromContinuation(credential.credentialId),
    isResidentCredential: true,
    rpId: credential.rpId,
    privateKey: canonicalCdpBase64FromContinuation(credential.privateKey),
    signCount: credential.signCount,
    userHandle: canonicalCdpBase64FromContinuation(credential.userHandle),
    backupEligibility: credential.backupEligibility,
    backupState: credential.backupState,
  };
}

function isBoundedText(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && value.length <= 256;
}

function requireCdpBinary(value: string, maximumBytes: number): void {
  if (value === "") throw new Error("CDP returned an empty WebAuthn binary field");
  const decoded = Buffer.from(value, "base64");
  const standard = decoded.toString("base64");
  const url = decoded.toString("base64url");
  if (
    decoded.length === 0 ||
    decoded.length > maximumBytes ||
    (value !== standard && value !== url)
  ) {
    throw new Error("CDP returned a noncanonical WebAuthn binary field");
  }
}

function canonicalBase64urlFromCdp(value: string, maximumBytes: number): string {
  requireCdpBinary(value, maximumBytes);
  return Buffer.from(value, "base64").toString("base64url");
}

function canonicalCdpBase64FromContinuation(value: string): string {
  return Buffer.from(value, "base64url").toString("base64");
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= 0xffffffff;
}
