// Strict, bounded decoding for immutable Blueprint history and recorded metadata.
import type { BlueprintHistoryEntryView } from "../../../generated/api/BlueprintHistoryEntryView";
import type { BlueprintHistoryPageView } from "../../../generated/api/BlueprintHistoryPageView";
import { DecodeError, decodeRecord, decodeStringEnum } from "../decoder";
import { decodeBlueprintRevision, text } from "./blueprint_course";
import { decodeCursorPage, decodeTimestamp, field, requireOnlyFields } from "./shared";

function entry(value: unknown, path: string): BlueprintHistoryEntryView {
  // ASVS 2.2.1: allowlist every tagged entry before it reaches browser state.
  const record = decodeRecord(value, path);
  const kind = field(record, "kind", path);
  if (kind === "savedRevision") {
    requireOnlyFields(record, path, ["kind", "revision", "savedAt"]);
    return {
      kind,
      revision: decodeBlueprintRevision(field(record, "revision", path), `${path}.revision`),
      savedAt: decodeTimestamp(field(record, "savedAt", path), `${path}.savedAt`),
    };
  }
  if (kind === "metadataChange") {
    requireOnlyFields(record, path, [
      "kind",
      "shortName",
      "longName",
      "availability",
      "recordedAt",
    ]);
    return {
      kind,
      shortName: text(field(record, "shortName", path), `${path}.shortName`),
      longName: text(field(record, "longName", path), `${path}.longName`),
      availability: decodeStringEnum(field(record, "availability", path), `${path}.availability`, [
        "private",
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
): BlueprintHistoryPageView {
  const page = decodeCursorPage(value, path, entry);
  return { items: [...page.items], nextCursor: page.nextCursor };
}
