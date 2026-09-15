// Same-origin browser transport for the deployment-gated live-demo entry seams.

import type { LiveDemoClient } from "../live_demo";
import {
  decodeLiveDemoSelection,
  decodeLiveDemoSelectedAccount,
  decodeSeededDemoAccounts,
  decodeSeededDemoPersona,
} from "../live_demo";
import { encodedId, requestJson, type ApiFetch } from "./request";

const ACCOUNTS_PATH = "/api/auth/live-demo/accounts";

export function createLiveDemoClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): LiveDemoClient {
  return {
    listSeededDemoAccounts: () =>
      requestJson(fetchImplementation, basePath, ACCOUNTS_PATH, decodeSeededDemoAccounts),
    selectSeededDemoAccount: (persona): ReturnType<LiveDemoClient["selectSeededDemoAccount"]> => {
      const selectedPersona = decodeSeededDemoPersona(persona);
      return requestJson(fetchImplementation, basePath, ACCOUNTS_PATH, decodeLiveDemoSelection, {
        method: "POST",
        body: { persona: selectedPersona },
      });
    },
    completeSeededDemoSysadminTotp: (attestationId, code) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/auth/sysadmin-totp/complete/${encodedId(attestationId)}`,
        decodeLiveDemoSelectedAccount,
        { method: "POST", body: { code } },
      ),
  };
}
