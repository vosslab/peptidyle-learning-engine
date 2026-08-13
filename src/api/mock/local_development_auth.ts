//! Local-preview-only credential exchange. The production browser bundle never
//! imports this test fixture transport.

import type { AuthSession } from "../contracts";
import { publishedProblemFixture } from "../../../generated/fixtures/published_problem";

export type LocalCredentialLogin = (credential: string) => Promise<AuthSession>;

/** Returns the deterministic preview login used only by the local browser build. */
export function createMockLocalCredentialLogin(): LocalCredentialLogin {
  return (credential: string) => {
    if (credential.length === 0) return Promise.reject(new Error("credential is required"));
    return Promise.resolve({
      authenticated: true,
      tenant: publishedProblemFixture.enrollment.tenant,
      user: {
        id: publishedProblemFixture.enrollment.user,
        displayName: "Fixture Student",
        roles: ["student"],
      },
    });
  };
}
