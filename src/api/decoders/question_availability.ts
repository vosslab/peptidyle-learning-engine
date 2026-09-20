// Runtime validation for Question lineage availability transport values.

import {
  DecodeError,
  decodeBoolean,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import type { QuestionLineageView } from "../../../generated/api/QuestionLineageView";
import { field, requireOnlyFields } from "./shared";
import type { QuestionAvailabilityTransition } from "../question_availability";
import { decodeQuestionSummary } from "./question_library";

/** Strict current lineage plus its session-derived Archive affordance. */
export function decodeQuestionLineageView(value: unknown, path = "response"): QuestionLineageView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["summary", "viewerMayArchive"]);
  return {
    summary: decodeQuestionSummary(field(record, "summary", path), `${path}.summary`, true),
    viewerMayArchive: decodeBoolean(
      field(record, "viewerMayArchive", path),
      `${path}.viewerMayArchive`,
    ),
  };
}

/** Decodes the small database-owned current lineage transition receipt. */
export function decodeQuestionAvailabilityTransition(
  value: unknown,
  path = "response",
): QuestionAvailabilityTransition {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["availability", "editNumber"]);
  const availabilityRecord = decodeRecord(
    field(record, "availability", path),
    `${path}.availability`,
  );
  requireOnlyFields(availabilityRecord, `${path}.availability`, ["availability"]);
  const availability = decodeStringEnum(
    field(availabilityRecord, "availability", `${path}.availability`),
    `${path}.availability.availability`,
    ["available", "archived"],
  );
  const editNumber = decodeString(field(record, "editNumber", path), `${path}.editNumber`);
  if (!/^[1-9][0-9]*$/u.test(editNumber)) {
    throw new DecodeError(`${path}.editNumber`, "a canonical positive Edit Number");
  }
  return { availability, questionAvailabilityEditNumber: editNumber };
}
