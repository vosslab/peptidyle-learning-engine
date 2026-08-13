//! Explicitly local-only credential exchange. This module is reachable only
//! through the local browser build boundary, never the ordinary client bundle.

import type { AuthSession } from "../contracts";
import { decodeAuthSession } from "../decoders";
import { browserFetch, normalizeBasePath, requestJson, type HttpApiClientConfig } from "./request";

export type LocalCredentialLogin = (credential: string) => Promise<AuthSession>;

/** Builds the one local-file credential exchange without retaining its input. */
export function createHttpLocalCredentialLogin(
  config: HttpApiClientConfig = {},
): LocalCredentialLogin {
  const basePath = normalizeBasePath(config.basePath);
  const fetchImplementation = config.fetch ?? browserFetch;
  return (credential: string) =>
    requestJson(fetchImplementation, basePath, "/api/auth/login", decodeAuthSession, {
      method: "POST",
      body: { credential },
    });
}
