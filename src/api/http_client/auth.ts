import type { AuthenticatedSession } from "../contracts";
import type { CompletedEmailCodeSignIn, EmailCodeClient } from "../email_code";
import {
  decodeAuthenticatedSession,
  decodeCompletedEmailCodeSignIn,
  decodeEmailCodeChallengeId,
  decodeSignedOutResponse,
  decodeStartedEmailCodeSignIn,
} from "../decoders";
import type { ApiFetch } from "./request";
import { encodedId, requestJson } from "./request";

export interface AuthClient extends EmailCodeClient {
  readonly getSession: () => Promise<AuthenticatedSession>;
  readonly logout: () => Promise<void>;
}

export function createAuthClient(fetchImplementation: ApiFetch, basePath: string): AuthClient {
  return {
    getSession: () =>
      requestJson(fetchImplementation, basePath, "/api/auth/session", decodeAuthenticatedSession),
    logout: async (): Promise<void> => {
      await requestJson(
        fetchImplementation,
        basePath,
        "/api/auth/logout",
        decodeSignedOutResponse,
        { method: "POST" },
      );
    },
    startEmailCodeSignIn: (email) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/auth/email-code/start",
        decodeStartedEmailCodeSignIn,
        { method: "POST", body: { email } },
      ),
    completeEmailCodeSignIn: (challengeId, code): Promise<CompletedEmailCodeSignIn> => {
      const decodedChallengeId = decodeEmailCodeChallengeId(challengeId);
      return requestJson(
        fetchImplementation,
        basePath,
        `/api/auth/email-code/complete/${encodedId(decodedChallengeId)}`,
        decodeCompletedEmailCodeSignIn,
        { method: "POST", body: { code } },
      );
    },
  };
}
