// Strict browser wire contracts for deployment-gated live-demo entry.

import {
  DecodeError,
  decodeArray,
  decodeField,
  decodeNonemptyString,
  decodeRecord,
  decodeStringEnum,
  decodeTrue,
} from "./decoder";
import { requireOnlyFields } from "./decoders/shared";

const SEEDED_DEMO_PERSONAS = [
  "elenaInstructor",
  "maryStudent",
  "jackStudent",
  "averyStudent",
  "morganSysadmin",
] as const;
const MAX_SEEDED_DEMO_ACCOUNTS = SEEDED_DEMO_PERSONAS.length;
const MAX_ACCOUNT_DISPLAY_NAME_CHARACTERS = 200;

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

/** Deployment-only authentication convenience; it is not a product identity model. */
export interface LiveDemoClient {
  readonly listSeededDemoAccounts: () => Promise<SeededDemoAccounts>;
  readonly selectSeededDemoAccount: (
    persona: SeededDemoPersona,
  ) => Promise<LiveDemoSelectedAccount>;
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
    throw new DecodeError(`${path}.accounts`, "an array with at most five entries");
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

export function decodeSeededDemoPersona(value: unknown, path = "persona"): SeededDemoPersona {
  return decodeStringEnum(value, path, SEEDED_DEMO_PERSONAS);
}
