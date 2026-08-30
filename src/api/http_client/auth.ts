import type { AuthenticatedSession } from "../contracts";
import { decodeAuthenticatedSession, decodeSignedOutResponse } from "../decoders";
import type { ApiFetch } from "./request";
import { requestJson } from "./request";

export interface AuthClient {
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
  };
}
