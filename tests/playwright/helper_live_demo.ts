// CDP-only WebAuthn support for the private live-demo browser journey.

import type { CDPSession, Page } from "@playwright/test";

export interface VirtualAuthenticator {
  readonly session: CDPSession;
  readonly id: string;
}

/** Creates the browser's normal virtual security key; no application transport is replaced. */
export async function installVirtualAuthenticator(page: Page): Promise<VirtualAuthenticator> {
  const session = await page.context().newCDPSession(page);
  await session.send("WebAuthn.enable");
  const result = await session.send("WebAuthn.addVirtualAuthenticator", {
    options: {
      protocol: "ctap2",
      transport: "internal",
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });
  return { session, id: String(result.authenticatorId) };
}
