// Strict, bounded decoding for immutable Blueprint history and recorded metadata.
import type { BlueprintHistoryEntryView } from "../../../generated/api/BlueprintHistoryEntryView";
import type { BlueprintHistoryPage } from "../blueprint_course";
import { THEME_VALUES } from "../../../generated/api/Theme";
import { DecodeError, decodeRecord, decodeSafeInteger, decodeStringEnum } from "../decoder";
import { blueprintEditNumber, decodeBlueprintRevisionNumber, text } from "./blueprint_course";
import { decodeBoundedArray, decodeTimestamp, field, requireOnlyFields } from "./shared";
import { decodeCourseClassification } from "./course_classification";

function entry(value: unknown, path: string): BlueprintHistoryEntryView {
  // ASVS 2.2.1: allowlist every tagged entry before it reaches browser state.
  const record = decodeRecord(value, path);
  const kind = field(record, "kind", path);
  if (kind === "savedRevision") {
    requireOnlyFields(record, path, ["kind", "revisionNumber", "savedAt"]);
    return {
      kind,
      revisionNumber: decodeBlueprintRevisionNumber(
        field(record, "revisionNumber", path),
        `${path}.revisionNumber`,
      ),
      savedAt: decodeTimestamp(field(record, "savedAt", path), `${path}.savedAt`),
    };
  }
  if (kind === "metadataChange") {
    requireOnlyFields(record, path, [
      "kind",
      "blueprintEditNumber",
      "shortName",
      "longName",
      "availability",
      "recordedAt",
      "classification",
      "theme",
    ]);
    return {
      kind,
      blueprintEditNumber: blueprintEditNumber(
        field(record, "blueprintEditNumber", path),
        `${path}.blueprintEditNumber`,
      ),
      shortName: text(field(record, "shortName", path), `${path}.shortName`),
      classification: decodeCourseClassification(
        field(record, "classification", path),
        `${path}.classification`,
      ),
      theme: decodeStringEnum(field(record, "theme", path), `${path}.theme`, THEME_VALUES),
      longName: text(field(record, "longName", path), `${path}.longName`),
      availability: decodeStringEnum(field(record, "availability", path), `${path}.availability`, [
        "public",
        "archived",
      ]),
      recordedAt: decodeTimestamp(field(record, "recordedAt", path), `${path}.recordedAt`),
    };
  }
  throw new DecodeError(`${path}.kind`, "savedRevision or metadataChange");
}

export function decodeBlueprintHistoryPageView(
  value: unknown,
  path = "response",
): BlueprintHistoryPage {
  const page = decodeRecord(value, path);
  requireOnlyFields(page, path, ["items", "nextCursor", "availabilityFacets"]);
  const cursor = field(page, "nextCursor", path);
  if (
    cursor !== null &&
    (typeof cursor !== "string" || cursor.length === 0 || cursor.length > 8192)
  )
    throw new DecodeError(`${path}.nextCursor`, "a bounded history cursor or null");
  const facets = decodeRecord(
    field(page, "availabilityFacets", path),
    `${path}.availabilityFacets`,
  );
  requireOnlyFields(facets, `${path}.availabilityFacets`, ["publicCount", "archivedCount"]);
  const publicCount = decodeSafeInteger(
    field(facets, "publicCount", path),
    `${path}.availabilityFacets.publicCount`,
  );
  const archivedCount = decodeSafeInteger(
    field(facets, "archivedCount", path),
    `${path}.availabilityFacets.archivedCount`,
  );
  if (publicCount < 0 || archivedCount < 0)
    throw new DecodeError(`${path}.availabilityFacets`, "nonnegative counts");
  return {
    items: [...decodeBoundedArray(field(page, "items", path), `${path}.items`, 100, entry)],
    nextCursor: cursor,
    availabilityFacets: { publicCount, archivedCount },
  };
}
