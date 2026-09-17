// library_page_helpers.tsx - small, presentation-neutral helpers for the Question Library page.

import { Show, type JSX } from "solid-js";

import { decodeQuestionId } from "../api/decoders/shared";
import {
  QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER,
  type QuestionLibraryBrowseQuery,
  type QuestionLibraryBrowseRow,
} from "./library_page_model";

export function questionLink(row: QuestionLibraryBrowseRow, returnToken: string): string {
  return `/library/${encodeURIComponent(row.displayId)}?${new URLSearchParams({
    [QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER]: returnToken,
  }).toString()}`;
}

export function questionTypeLabel(value: string): string {
  const labels: Readonly<Record<string, string>> = {
    multipleChoice: "Multiple choice",
    multipleAnswer: "Multiple answer",
    fillInBlank: "Fill in the blank",
    multipleFillInBlank: "Multiple fill in the blank",
    numeric: "Numeric",
    matching: "Matching",
    ordering: "Ordering",
    hotspot: "Hotspot",
  };
  return labels[value] ?? value;
}

export function backendLabel(value: string): string {
  const labels: Readonly<Record<string, string>> = {
    ple: "PLE",
    webwork: "WeBWorK",
    imathas: "IMathAS",
  };
  return labels[value] ?? value;
}

export function selectedQuestionLibrarySort(value: string): QuestionLibraryBrowseQuery["sort"] {
  // ASVS 2.2.1: retain only the closed server-supported sort values at the UI boundary.
  if (value === "titleAscending" || value === "publishedNewest") return value;
  throw new Error("Question Library sort selection is invalid");
}

export function hasCanonicalPoolDeepLink(search: string): boolean {
  const values = new URLSearchParams(search).getAll("pool");
  if (values.length !== 1) return false;
  try {
    decodeQuestionId(values[0], "pool");
    return true;
  } catch {
    return false;
  }
}

export function RetainedSelectOption(props: {
  readonly value: string | null | undefined;
  readonly label: (value: string) => string;
}): JSX.Element {
  return (
    <Show when={props.value}>
      {(value) => (
        <option value={value()} selected>
          {props.label(value())}
        </option>
      )}
    </Show>
  );
}

export function webworkFormatLabel(
  value: QuestionLibraryBrowseRow["questionFormat"],
): string | null {
  if (value === "webworkPg") return "PG";
  if (value === "webworkPgml") return "PGML";
  return null;
}
