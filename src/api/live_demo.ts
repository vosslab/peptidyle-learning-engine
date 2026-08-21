// Strict browser wire contracts for deployment-gated live-demo entry.

import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeField,
  decodeNonemptyString,
  decodeRecord,
  decodeStringEnum,
  decodeTrue,
  decodeUuid,
} from "./decoder";
import { requireOnlyFields } from "./decoders/shared";

const SEEDED_DEMO_PERSONAS = [
  "elenaInstructor",
  "maryStudent",
  "jackStudent",
  "averyStudent",
] as const;
const MAX_SEEDED_DEMO_ACCOUNTS = SEEDED_DEMO_PERSONAS.length;
const MAX_ACCOUNT_DISPLAY_NAME_CHARACTERS = 200;
const MAX_PASSKEY_LABEL_CHARACTERS = 80;
const OWNERSHIP_PROOF_PATTERN = /^[A-Za-z0-9_-]{43}$/u;

export type SeededDemoPersona = (typeof SEEDED_DEMO_PERSONAS)[number];

export interface SeededDemoAccount {
  readonly persona: SeededDemoPersona;
  readonly displayName: string;
}

export interface SeededDemoAccounts {
  readonly accounts: ReadonlyArray<SeededDemoAccount>;
}

export interface LiveDemoSelectedAccount {
  readonly authenticated: true;
}

export interface LiveDemoOwnershipStatus {
  readonly available: boolean;
}

export interface LiveDemoOwnershipStart {
  readonly ceremonyId: string;
  readonly options: Readonly<Record<string, unknown>>;
}

export interface LiveDemoOwnershipComplete {
  readonly authenticated: true;
}

/** Deployment-only authentication convenience; it is not a product identity model. */
export interface LiveDemoClient {
  readonly listSeededDemoAccounts: () => Promise<SeededDemoAccounts>;
  readonly selectSeededDemoAccount: (
    persona: SeededDemoPersona,
  ) => Promise<LiveDemoSelectedAccount>;
  readonly getLiveDemoSysadminOwnershipStatus: () => Promise<LiveDemoOwnershipStatus>;
  readonly startLiveDemoSysadminOwnership: (
    ownershipProof: string,
  ) => Promise<LiveDemoOwnershipStart>;
  readonly completeLiveDemoSysadminOwnership: (
    ownershipProof: string,
    ceremonyId: string,
    label: string,
    credential: RegistrationResponseJSON,
  ) => Promise<LiveDemoOwnershipComplete>;
}

function closedRecord(
  value: unknown,
  path: string,
  fields: ReadonlyArray<string>,
): Record<string, unknown> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, fields);
  for (const field of fields) decodeField(record, field, path);
  return record;
}

function boundedNonblankText(value: unknown, path: string, maximum: number): string {
  const text = decodeNonemptyString(value, path);
  if (text.trim().length === 0 || Array.from(text).length > maximum) {
    throw new DecodeError(path, `nonblank text no longer than ${maximum} Unicode scalar values`);
  }
  return text;
}

function decodeSeededDemoAccount(value: unknown, path: string): SeededDemoAccount {
  const record = closedRecord(value, path, ["persona", "displayName"]);
  return {
    persona: decodeStringEnum(record.persona, `${path}.persona`, SEEDED_DEMO_PERSONAS),
    displayName: boundedNonblankText(
      record.displayName,
      `${path}.displayName`,
      MAX_ACCOUNT_DISPLAY_NAME_CHARACTERS,
    ),
  };
}

export function decodeSeededDemoAccounts(value: unknown, path = "response"): SeededDemoAccounts {
  const record = closedRecord(value, path, ["accounts"]);
  const accounts = decodeArray(record.accounts, `${path}.accounts`, decodeSeededDemoAccount);
  if (accounts.length > MAX_SEEDED_DEMO_ACCOUNTS) {
    throw new DecodeError(`${path}.accounts`, "an array with at most four entries");
  }
  const personas = accounts.map((account) => account.persona);
  if (new Set(personas).size !== personas.length) {
    throw new DecodeError(`${path}.accounts`, "accounts with unique personas");
  }
  return { accounts };
}

function decodeAuthenticated(value: unknown, path: string): LiveDemoSelectedAccount {
  const record = closedRecord(value, path, ["authenticated"]);
  return { authenticated: decodeTrue(record.authenticated, `${path}.authenticated`) };
}

export function decodeLiveDemoSelectedAccount(
  value: unknown,
  path = "response",
): LiveDemoSelectedAccount {
  return decodeAuthenticated(value, path);
}

export function decodeLiveDemoOwnershipStatus(
  value: unknown,
  path = "response",
): LiveDemoOwnershipStatus {
  const record = closedRecord(value, path, ["available"]);
  return { available: decodeBoolean(record.available, `${path}.available`) };
}

export function decodeLiveDemoOwnershipStart(
  value: unknown,
  path = "response",
): LiveDemoOwnershipStart {
  const record = closedRecord(value, path, ["ceremonyId", "options"]);
  return {
    ceremonyId: decodeUuid(record.ceremonyId, `${path}.ceremonyId`),
    options: decodeRecord(record.options, `${path}.options`),
  };
}

export function decodeLiveDemoCeremonyId(value: unknown, path = "ceremonyId"): string {
  return decodeUuid(value, path);
}

export function decodeLiveDemoOwnershipComplete(
  value: unknown,
  path = "response",
): LiveDemoOwnershipComplete {
  return decodeAuthenticated(value, path);
}

export function decodeSeededDemoPersona(value: unknown, path = "persona"): SeededDemoPersona {
  return decodeStringEnum(value, path, SEEDED_DEMO_PERSONAS);
}

export function decodeLiveDemoOwnershipProof(value: unknown, path = "ownershipProof"): string {
  const proof = decodeNonemptyString(value, path);
  if (!OWNERSHIP_PROOF_PATTERN.test(proof)) {
    throw new DecodeError(path, "one canonical 32-byte URL-safe ownership proof");
  }
  return proof;
}

export function decodeLiveDemoPasskeyLabel(value: unknown, path = "label"): string {
  const label = decodeNonemptyString(value, path);
  const trimmed = label.trim();
  if (trimmed.length === 0 || Array.from(trimmed).length > MAX_PASSKEY_LABEL_CHARACTERS) {
    throw new DecodeError(
      path,
      "nonblank passkey-label text no longer than 80 Unicode scalar values",
    );
  }
  return trimmed;
}
