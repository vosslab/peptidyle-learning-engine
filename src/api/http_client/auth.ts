import type { AuthSession } from "../contracts";
import { decodeAuthSession } from "../decoders";
import type { ApiFetch } from "./request";
import { requestJson } from "./request";

export interface AuthClient {
  readonly getSession: () => Promise<AuthSession>;
  readonly loginWithLocalCredential: (credential: string) => Promise<AuthSession>;
}

export function createAuthClient(fetchImplementation: ApiFetch, basePath: string): AuthClient {
  return {
    getSession: () =>
      requestJson(fetchImplementation, basePath, "/api/auth/session", decodeAuthSession),
    loginWithLocalCredential: (credential: string) =>
      requestJson(fetchImplementation, basePath, "/api/auth/login", decodeAuthSession, {
        method: "POST",
        body: { credential },
      }),
  };
}
