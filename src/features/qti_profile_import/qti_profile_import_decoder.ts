// qti_profile_import_decoder.ts - strict runtime boundary for safe QTI report JSON.

import type { WorkspaceImportId } from "../../../generated/api/WorkspaceImportId";
import {
  DecodeError,
  decodeArray,
  decodeField,
  decodeNullable,
  decodeRecord,
  decodeString,
  decodeStringEnum,
  decodeUuid,
} from "../../api/decoder";
import type {
  QtiProfileDiagnostic,
  QtiProfileImportFailure,
  QtiProfileImportProgress,
  QtiProfileImportReadyReport,
  QtiProfileImportResponse,
  QtiProfileItemReport,
} from "./qti_profile_import_contract";

const PROFILE_IDS = [
  "canvas-qti-1.2-static-single-choice/v1",
  "blackboard-qti-2.1-static-single-choice-pool/v1",
] as const;
const MAX_ITEMS = 1_000;
const MAX_DIAGNOSTICS = 32;

function onlyFields(
  record: Record<string, unknown>,
  path: string,
  allowed: ReadonlyArray<string>,
): void {
  for (const key of Object.keys(record)) {
    if (!allowed.includes(key)) {
      throw new DecodeError(`${path}.${key}`, "a field allowed by the QTI report contract");
    }
  }
}

function boundedString(value: unknown, path: string, maximum: number, allowEmpty = false): string {
  const decoded = decodeString(value, path);
  if ((!allowEmpty && decoded.trim().length === 0) || Array.from(decoded).length > maximum) {
    throw new DecodeError(path, `a bounded${allowEmpty ? "" : " nonblank"} string`);
  }
  for (const character of decoded) {
    const point = character.codePointAt(0);
    if (point !== undefined && (point <= 0x1f || point === 0x7f)) {
      throw new DecodeError(path, "a string without control characters");
    }
  }
  return decoded;
}

function digest(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!/^[0-9a-f]{64}$/u.test(decoded)) {
    throw new DecodeError(path, "a lowercase SHA-256 acknowledgement");
  }
  return decoded;
}

function diagnostic(value: unknown, path: string): QtiProfileDiagnostic {
  const record = decodeRecord(value, path);
  onlyFields(record, path, ["code", "location", "detail"]);
  return {
    code: boundedString(decodeField(record, "code", path), `${path}.code`, 160),
    location: boundedString(decodeField(record, "location", path), `${path}.location`, 1_024, true),
    detail: boundedString(decodeField(record, "detail", path), `${path}.detail`, 2_048),
  };
}

function diagnostics(value: unknown, path: string): ReadonlyArray<QtiProfileDiagnostic> {
  const decoded = decodeArray(value, path, diagnostic);
  if (decoded.length > MAX_DIAGNOSTICS) {
    throw new DecodeError(path, `at most ${MAX_DIAGNOSTICS} entries`);
  }
  return decoded;
}

function item(value: unknown, path: string): QtiProfileItemReport {
  const record = decodeRecord(value, path);
  onlyFields(record, path, [
    "sourceIdentifier",
    "title",
    "status",
    "diagnostics",
    "defaults",
    "warnings",
  ]);
  return {
    sourceIdentifier: boundedString(
      decodeField(record, "sourceIdentifier", path),
      `${path}.sourceIdentifier`,
      1_024,
    ),
    title: decodeNullable(decodeField(record, "title", path), `${path}.title`, (entry, entryPath) =>
      boundedString(entry, entryPath, 512),
    ),
    status: decodeStringEnum(decodeField(record, "status", path), `${path}.status`, [
      "accepted",
      "rejected",
    ]),
    diagnostics: diagnostics(decodeField(record, "diagnostics", path), `${path}.diagnostics`),
    defaults: diagnostics(decodeField(record, "defaults", path), `${path}.defaults`),
    warnings: diagnostics(decodeField(record, "warnings", path), `${path}.warnings`),
  };
}

function importId(record: Record<string, unknown>, path: string): WorkspaceImportId {
  return decodeUuid(decodeField(record, "importId", path), `${path}.importId`);
}

function progress(
  record: Record<string, unknown>,
  path: string,
  state: "queued" | "processing",
): QtiProfileImportProgress {
  onlyFields(record, path, ["importId", "state"]);
  return { importId: importId(record, path), state };
}

function failure(
  record: Record<string, unknown>,
  path: string,
  state: "failed" | "unsupportedProfile",
): QtiProfileImportFailure {
  onlyFields(record, path, ["importId", "state", "error"]);
  return {
    importId: importId(record, path),
    state,
    error: boundedString(decodeField(record, "error", path), `${path}.error`, 512),
  };
}

function ready(record: Record<string, unknown>, path: string): QtiProfileImportReadyReport {
  onlyFields(record, path, [
    "importId",
    "state",
    "profileId",
    "profileLabel",
    "profileVersion",
    "reportRevision",
    "items",
    "pleDefaults",
    "reviewToken",
  ]);
  const decodedItems = decodeArray(decodeField(record, "items", path), `${path}.items`, item);
  if (decodedItems.length === 0 || decodedItems.length > MAX_ITEMS) {
    throw new DecodeError(`${path}.items`, `between 1 and ${MAX_ITEMS} items`);
  }
  return {
    importId: importId(record, path),
    state: "ready",
    profileId: decodeStringEnum(
      decodeField(record, "profileId", path),
      `${path}.profileId`,
      PROFILE_IDS,
    ),
    profileLabel: boundedString(
      decodeField(record, "profileLabel", path),
      `${path}.profileLabel`,
      200,
    ),
    profileVersion: boundedString(
      decodeField(record, "profileVersion", path),
      `${path}.profileVersion`,
      80,
    ),
    reportRevision: digest(decodeField(record, "reportRevision", path), `${path}.reportRevision`),
    items: decodedItems,
    pleDefaults: diagnostics(decodeField(record, "pleDefaults", path), `${path}.pleDefaults`),
    reviewToken: digest(decodeField(record, "reviewToken", path), `${path}.reviewToken`),
  };
}

export function decodeQtiProfileImportResponse(
  value: unknown,
  path = "response",
): QtiProfileImportResponse {
  const record = decodeRecord(value, path);
  const state = decodeStringEnum(decodeField(record, "state", path), `${path}.state`, [
    "queued",
    "processing",
    "ready",
    "failed",
    "unsupportedProfile",
  ]);
  if (state === "queued" || state === "processing") return progress(record, path, state);
  if (state === "failed" || state === "unsupportedProfile") return failure(record, path, state);
  return ready(record, path);
}
