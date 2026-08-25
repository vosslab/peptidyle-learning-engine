// CDP-only WebAuthn support for the private live-demo browser journey.

import type { Page } from "@playwright/test";

export interface CdpProtocolSession {
  send(method: string, params?: object): Promise<unknown>;
}

export interface VirtualAuthenticator {
  readonly session: CdpProtocolSession;
  readonly id: string;
}

const VIRTUAL_AUTHENTICATOR_OPTIONS = {
  protocol: "ctap2",
  transport: "internal",
  hasResidentKey: true,
  hasUserVerification: true,
  isUserVerified: true,
  automaticPresenceSimulation: true,
} as const;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

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
