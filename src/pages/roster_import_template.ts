import type { CourseRosterImportEntry } from "../api/course_roster";
import { decodeCourseRosterImportInput, trimRosterName } from "../api/decoders/course_roster";

/** Synthetic template intentionally containing no real Course or Account data. */
export function rosterImportTemplateCsv(): string {
  return 'email,roster_id,roster_name\nsynthetic-student@example.edu,synthetic-001,"Example, Synthetic Student"\n';
}

/** Parses reviewed pasted CSV, including quoted commas and doubled quotation marks. */
export function parseRosterImportRows(value: string): ReadonlyArray<CourseRosterImportEntry> {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let quoted = false;
  let closedQuote = false;
  const malformed = (): never => {
    throw new Error("Use email,roster_id,roster_name rows with correctly quoted CSV fields.");
  };
  function finishField(): void {
    row.push(trimRosterName(field));
    field = "";
    closedQuote = false;
  }
  function finishRow(): void {
    finishField();
    if (row.some((item) => item !== "")) rows.push(row);
    row = [];
  }
  for (let index = 0; index < value.length; index += 1) {
    const character = value[index];
    if (quoted) {
      if (character === '"') {
        if (value[index + 1] === '"') {
          field += '"';
          index += 1;
        } else {
          quoted = false;
          closedQuote = true;
        }
      } else field += character;
    } else if (character === ",") finishField();
    else if (character === "\n" || character === "\r") {
      finishRow();
      if (character === "\r" && value[index + 1] === "\n") index += 1;
    } else if (character === '"') {
      if (closedQuote || trimRosterName(field) !== "") malformed();
      field = "";
      quoted = true;
    } else {
      if (closedQuote && trimRosterName(character ?? "") !== "") malformed();
      if (!closedQuote) field += character;
    }
  }
  if (quoted) malformed();
  finishRow();
  if (rows[0]?.join(",") === "email,roster_id,roster_name") rows.shift();
  const entries = rows.map((fields) => {
    if (fields.length !== 3) malformed();
    return { email: fields[0], rosterId: fields[1], rosterName: fields[2] };
  });
  // ASVS 2.2.1: reject malformed input before transport; the server remains authoritative.
  try {
    return decodeCourseRosterImportInput({ entries }).entries;
  } catch {
    throw new Error(
      "Use 1-50 valid email,roster_id,roster_name rows. Names need 1-200 characters without controls; email and roster ID must be unique.",
    );
  }
}
