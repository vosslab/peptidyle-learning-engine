import type { AuthSession } from "../contracts";
import { decodeAuthSession, decodeSignedOutResponse } from "../decoders";
import type { ApiFetch } from "./request";
import { requestJson } from "./request";

export interface AuthClient {
  readonly getSession: () => Promise<AuthSession>;
  readonly logout: () => Promise<void>;
}

export function createAuthClient(fetchImplementation: ApiFetch, basePath: string): AuthClient {
  return {
    getSession: () =>
      requestJson(fetchImplementation, basePath, "/api/auth/session", decodeAuthSession),
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
