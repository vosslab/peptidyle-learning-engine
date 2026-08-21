// Same-origin browser transport for the deployment-gated live-demo entry seams.

import type { LiveDemoClient } from "../live_demo";
import {
  decodeLiveDemoOwnershipComplete,
  decodeLiveDemoCeremonyId,
  decodeLiveDemoOwnershipProof,
  decodeLiveDemoOwnershipStart,
  decodeLiveDemoOwnershipStatus,
  decodeLiveDemoPasskeyLabel,
  decodeLiveDemoSelectedAccount,
  decodeSeededDemoAccounts,
  decodeSeededDemoPersona,
} from "../live_demo";
import { registerWebauthnWithBrowser } from "./enrollment";
import { requestJson, type ApiFetch } from "./request";

const ACCOUNTS_PATH = "/api/auth/live-demo/accounts";
const OWNERSHIP_PATH = "/api/auth/live-demo/sysadmin-ownership";
const OWNERSHIP_COMPLETE_PATH = "/api/auth/live-demo/sysadmin-ownership/complete";

export function createLiveDemoClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): LiveDemoClient {
  return {
    listSeededDemoAccounts: () =>
      requestJson(fetchImplementation, basePath, ACCOUNTS_PATH, decodeSeededDemoAccounts),
    selectSeededDemoAccount: (persona): ReturnType<LiveDemoClient["selectSeededDemoAccount"]> => {
      const selectedPersona = decodeSeededDemoPersona(persona);
      return requestJson(
        fetchImplementation,
        basePath,
        ACCOUNTS_PATH,
        decodeLiveDemoSelectedAccount,
        {
          method: "POST",
          body: { persona: selectedPersona },
        },
      );
    },
    getLiveDemoSysadminOwnershipStatus: () =>
      requestJson(fetchImplementation, basePath, OWNERSHIP_PATH, decodeLiveDemoOwnershipStatus),
    startLiveDemoSysadminOwnership: (
      ownershipProof,
    ): ReturnType<LiveDemoClient["startLiveDemoSysadminOwnership"]> => {
      const proof = decodeLiveDemoOwnershipProof(ownershipProof);
      return requestJson(
        fetchImplementation,
        basePath,
        OWNERSHIP_PATH,
        decodeLiveDemoOwnershipStart,
        {
          method: "POST",
          body: { ownershipProof: proof },
        },
      );
    },
    completeLiveDemoSysadminOwnership: (
      ownershipProof,
      ceremonyId,
      label,
      credential,
    ): ReturnType<LiveDemoClient["completeLiveDemoSysadminOwnership"]> => {
      const proof = decodeLiveDemoOwnershipProof(ownershipProof);
      const decodedCeremonyId = decodeLiveDemoCeremonyId(ceremonyId);
      const decodedLabel = decodeLiveDemoPasskeyLabel(label);
      return requestJson(
        fetchImplementation,
        basePath,
        OWNERSHIP_COMPLETE_PATH,
        decodeLiveDemoOwnershipComplete,
        {
          method: "POST",
          body: {
            ownershipProof: proof,
            ceremonyId: decodedCeremonyId,
            label: decodedLabel,
            credential,
          },
        },
      );
    },
  };
}

/** Runs the ordinary browser registration ceremony without reaching account or course data. */
export async function registerLiveDemoSysadminWithBrowser(
  client: LiveDemoClient,
  ownershipProof: string,
  label: string,
): Promise<void> {
  const proof = decodeLiveDemoOwnershipProof(ownershipProof);
  const decodedLabel = decodeLiveDemoPasskeyLabel(label);
  await registerWebauthnWithBrowser(
    () => client.startLiveDemoSysadminOwnership(proof),
    (ceremonyId, registrationLabel, credential) =>
      client.completeLiveDemoSysadminOwnership(proof, ceremonyId, registrationLabel, credential),
    decodedLabel,
  );
}
