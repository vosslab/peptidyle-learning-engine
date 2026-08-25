// Strict browser decoding and local command validation for reusable curricula.

import { MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP } from "../../../generated/api/MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP";
import { MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS } from "../../../generated/api/MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS";
import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import { MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES } from "../../../generated/api/MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES";
import { MAX_PROBLEM_CURATION_TITLE_UNICODE_SCALARS } from "../../../generated/api/MAX_PROBLEM_CURATION_TITLE_UNICODE_SCALARS";
import type { AlphaCourseDefinitionInput } from "../../../generated/api/AlphaCourseDefinitionInput";
import type { AlphaCourseReference } from "../../../generated/api/AlphaCourseReference";
import type { AlphaCourseSummaryView } from "../../../generated/api/AlphaCourseSummaryView";
import type { AlphaCourseView } from "../../../generated/api/AlphaCourseView";
import type { BlueprintDefinitionInput } from "../../../generated/api/BlueprintDefinitionInput";
import type { BlueprintReference } from "../../../generated/api/BlueprintReference";
import type { BlueprintSummaryView } from "../../../generated/api/BlueprintSummaryView";
import type { BlueprintView } from "../../../generated/api/BlueprintView";
import type { CatalogDiscoveryItem } from "../../../generated/api/CatalogDiscoveryItem";
import type { CompletionRequirement } from "../../../generated/api/CompletionRequirement";
import type { ContinuedPractice } from "../../../generated/api/ContinuedPractice";
import type { RunPolicies } from "../../../generated/api/RunPolicies";
import type { CursorPage } from "../contracts";
import {
  DecodeError,
  decodeFiniteNumber,
  decodeNonemptyString,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeCatalogProblemSummary } from "./catalog_course";
import { decodeLearnerDisclosurePolicy } from "./assignment_policy";
import { decodePublicByline } from "../public_byline";
import { decodeBoundedArray, decodeCursor, field, requireOnlyFields } from "./shared";

const MAX_PAGE_SIZE = 100;
const QUESTION_ID = /^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u;
const POSITIVE_REVISION = /^[1-9][0-9]*$/u;
const LOCAL_TIME = /^([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]\.[0-9]{3}$/u;

function text(value: unknown, path: string): string {
  const decoded = decodeNonemptyString(value, path);
  if (
    decoded !== decoded.trim() ||
    Array.from(decoded).length > MAX_PROBLEM_CURATION_TITLE_UNICODE_SCALARS
  ) {
    throw new DecodeError(path, "trimmed curriculum text within its shared bound");
  }
  return decoded;
}

function reference(value: unknown, path: string, prefix: "BP" | "AC"): string {
  const decoded = decodeString(value, path);
  const pattern = new RegExp(`^${prefix}-[1-9][0-9]{0,9}$`, "u");
  if (!pattern.test(decoded) || Number(decoded.slice(prefix.length + 1)) > 2_147_483_647) {
    throw new DecodeError(path, `a canonical ${prefix} public reference`);
  }
  return decoded;
}

function revision(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!POSITIVE_REVISION.test(decoded) || BigInt(decoded) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a canonical positive PostgreSQL bigint revision");
  }
  return decoded;
}

function questionId(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!QUESTION_ID.test(decoded)) throw new DecodeError(path, "a canonical public Question ID");
  return decoded;
}

function pointValue(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!/^(?:0|[1-9][0-9]*)(?:\.[0-9]{1,4})?$/u.test(decoded)) {
    throw new DecodeError(path, "a canonical nonnegative point decimal with at most four places");
  }
  return decoded;
}

function localTime(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!LOCAL_TIME.test(decoded)) throw new DecodeError(path, "exact HH:MM:SS.sss local time");
  return decoded;
}

function scheduleMoment(value: unknown, path: string): { dayOffset: number; localTime: string } {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["dayOffset", "localTime"]);
  return {
    dayOffset: decodeSafeInteger(field(record, "dayOffset", path), `${path}.dayOffset`),
    localTime: localTime(field(record, "localTime", path), `${path}.localTime`),
  };
}

function schedule(
  value: unknown,
  path: string,
): BlueprintDefinitionInput["definition"]["schedule"] {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["availableAt", "dueAt", "closesAt"]);
  const decoded = {
    availableAt: decodeNullable(
      field(record, "availableAt", path),
      `${path}.availableAt`,
      scheduleMoment,
    ),
    dueAt: decodeNullable(field(record, "dueAt", path), `${path}.dueAt`, scheduleMoment),
    closesAt: decodeNullable(field(record, "closesAt", path), `${path}.closesAt`, scheduleMoment),
  };
  const moments = [decoded.availableAt, decoded.dueAt, decoded.closesAt];
  for (let index = 1; index < moments.length; index += 1) {
    const prior = moments[index - 1];
    const current = moments[index];
    if (prior === undefined || current === undefined) continue;
    if (prior !== null && current !== null) {
      const priorAfterCurrent =
        prior.dayOffset > current.dayOffset ||
        (prior.dayOffset === current.dayOffset && prior.localTime > current.localTime);
      if (priorAfterCurrent)
        throw new DecodeError(path, "chronologically ordered schedule moments");
    }
  }
  return decoded;
}

function defaults(
  value: unknown,
  path: string,
): BlueprintDefinitionInput["definition"]["defaults"] {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "timeLimitSeconds",
    "attemptLimit",
    "lateSubmission",
    "deadlineBehavior",
    "runPolicies",
    "learnerDisclosure",
  ]);
  const timeLimitSeconds = decodeNullable(
    field(record, "timeLimitSeconds", path),
    `${path}.timeLimitSeconds`,
    decodePositiveInteger,
  );
  const attemptLimit = decodeNullable(
    field(record, "attemptLimit", path),
    `${path}.attemptLimit`,
    decodePositiveInteger,
  );
  const lateSubmission = decodeStringEnum(
    field(record, "lateSubmission", path),
    `${path}.lateSubmission`,
    ["accept", "markLate", "reject"],
  );
  const deadlineBehavior = decodeStringEnum(
    field(record, "deadlineBehavior", path),
    `${path}.deadlineBehavior`,
    ["autoSubmit"],
  );
  const runPolicies = decodeRunPolicies(field(record, "runPolicies", path), `${path}.runPolicies`);
  const learnerDisclosure = decodeLearnerDisclosurePolicy(
    field(record, "learnerDisclosure", path),
    `${path}.learnerDisclosure`,
  );
  return {
    timeLimitSeconds,
    attemptLimit,
    lateSubmission,
    deadlineBehavior,
    runPolicies,
    learnerDisclosure,
  };
}

function completionRequirement(value: unknown, path: string): CompletionRequirement {
  const record = decodeRecord(value, path);
  const requirement = decodeString(field(record, "kind", path), `${path}.kind`);
  if (requirement === "answerAll" || requirement === "allCorrect") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind: requirement };
  }
  if (requirement === "scoreAtLeast") {
    requireOnlyFields(record, path, ["kind", "fraction"]);
    const fraction = decodeFiniteNumber(field(record, "fraction", path), `${path}.fraction`);
    if (fraction < 0 || fraction > 1)
      throw new DecodeError(`${path}.fraction`, "a fraction from 0 through 1");
    return { kind: "scoreAtLeast", fraction };
  }
  throw new DecodeError(`${path}.kind`, "a known completion requirement");
}

function continuedPractice(value: unknown, path: string): ContinuedPractice {
  const record = decodeRecord(value, path);
  const policy = decodeString(field(record, "kind", path), `${path}.kind`);
  if (policy === "unlimited" || policy === "closed") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind: policy };
  }
  if (policy === "capped") {
    requireOnlyFields(record, path, ["kind", "maxAdditionalRuns"]);
    const maxAdditionalRuns = decodeSafeInteger(
      field(record, "maxAdditionalRuns", path),
      `${path}.maxAdditionalRuns`,
    );
    if (maxAdditionalRuns < 0)
      throw new DecodeError(`${path}.maxAdditionalRuns`, "a nonnegative safe integer");
    return { kind: "capped", maxAdditionalRuns };
  }
  throw new DecodeError(`${path}.kind`, "a known continued-practice policy");
}

function decodeRunPolicies(value: unknown, path: string): RunPolicies {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["completion", "grade", "continuedPractice", "variation"]);
  return {
    completion: completionRequirement(field(record, "completion", path), `${path}.completion`),
    grade: decodeStringEnum(field(record, "grade", path), `${path}.grade`, [
      "first",
      "latest",
      "highest",
      "instructorSelected",
    ]),
    continuedPractice: continuedPractice(
      field(record, "continuedPractice", path),
      `${path}.continuedPractice`,
    ),
    variation: decodeStringEnum(field(record, "variation", path), `${path}.variation`, [
      "newSeeds",
      "selectedProblemVariants",
      "fullRegeneration",
    ]),
  };
}

function entry(
  value: unknown,
  path: string,
): BlueprintDefinitionInput["definition"]["entries"][number] {
  const record = decodeRecord(value, path);
  const entryKind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "fixed",
    "pool",
  ]);
  if (entryKind === "fixed") {
    requireOnlyFields(record, path, ["kind", "questionId", "pointsPossible", "scoringMode"]);
    return {
      kind: "fixed",
      questionId: questionId(field(record, "questionId", path), `${path}.questionId`),
      pointsPossible: pointValue(field(record, "pointsPossible", path), `${path}.pointsPossible`),
      scoringMode: decodeStringEnum(field(record, "scoringMode", path), `${path}.scoringMode`, [
        "normal",
        "fullCredit",
        "extraCredit",
        "excluded",
      ]),
    };
  }
  requireOnlyFields(record, path, [
    "kind",
    "candidates",
    "drawCount",
    "pointsPerItem",
    "ordering",
    "algorithm",
  ]);
  const candidates = decodeBoundedArray(
    field(record, "candidates", path),
    `${path}.candidates`,
    MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP,
    questionId,
  );
  const drawCount = decodePositiveInteger(field(record, "drawCount", path), `${path}.drawCount`);
  if (
    candidates.length === 0 ||
    drawCount > candidates.length ||
    new Set(candidates).size !== candidates.length
  ) {
    throw new DecodeError(path, "a nonempty pool with distinct candidates and a valid draw count");
  }
  return {
    kind: "pool",
    candidates,
    drawCount,
    pointsPerItem: pointValue(field(record, "pointsPerItem", path), `${path}.pointsPerItem`),
    ordering: decodeStringEnum(field(record, "ordering", path), `${path}.ordering`, [
      "candidateOrder",
      "randomized",
    ]),
    algorithm: decodeStringEnum(field(record, "algorithm", path), `${path}.algorithm`, ["v1"]),
  };
}

function definition(value: unknown, path: string): BlueprintDefinitionInput["definition"] {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "instructions", "entries", "defaults", "schedule"]);
  const instructions = decodeString(field(record, "instructions", path), `${path}.instructions`);
  if (Array.from(instructions).length > MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS) {
    throw new DecodeError(`${path}.instructions`, "instructions within the shared bound");
  }
  const entries = decodeBoundedArray(
    field(record, "entries", path),
    `${path}.entries`,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    entry,
  );
  if (entries.length === 0) throw new DecodeError(`${path}.entries`, "at least one ordered entry");
  const candidates = entries.reduce(
    (total, current) => total + (current.kind === "pool" ? current.candidates.length : 0),
    0,
  );
  if (candidates > MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES)
    throw new DecodeError(`${path}.entries`, "pool candidates within the assignment total bound");
  return {
    title: text(field(record, "title", path), `${path}.title`),
    instructions,
    entries,
    defaults: defaults(field(record, "defaults", path), `${path}.defaults`),
    schedule: schedule(field(record, "schedule", path), `${path}.schedule`),
  };
}

export function decodeBlueprintDefinitionInput(
  value: unknown,
  path = "request",
): BlueprintDefinitionInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["definition"]);
  return { definition: definition(field(record, "definition", path), `${path}.definition`) };
}

export function decodeAlphaCourseDefinitionInput(
  value: unknown,
  path = "request",
): AlphaCourseDefinitionInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "modules"]);
  const modules = decodeBoundedArray(
    field(record, "modules", path),
    `${path}.modules`,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    (entryValue, entryPath) => {
      const module = decodeRecord(entryValue, entryPath);
      requireOnlyFields(module, entryPath, ["label", "definitions"]);
      const definitions = decodeBoundedArray(
        field(module, "definitions", entryPath),
        `${entryPath}.definitions`,
        MAX_ASSIGNMENT_ORDERED_ENTRIES,
        definition,
      );
      if (definitions.length === 0)
        throw new DecodeError(`${entryPath}.definitions`, "at least one ordered definition");
      return { label: text(field(module, "label", entryPath), `${entryPath}.label`), definitions };
    },
  );
  if (modules.length === 0) throw new DecodeError(`${path}.modules`, "at least one ordered module");
  return { title: text(field(record, "title", path), `${path}.title`), modules };
}

function discovery(value: unknown, path: string): CatalogDiscoveryItem {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["summary", "evidence"]);
  const evidenceRecord = decodeRecord(field(record, "evidence", path), `${path}.evidence`);
  const evidenceState = decodeStringEnum(
    field(evidenceRecord, "state", `${path}.evidence`),
    `${path}.evidence.state`,
    ["insufficientEvidence", "available"],
  );
  if (evidenceState === "insufficientEvidence")
    requireOnlyFields(evidenceRecord, `${path}.evidence`, ["state"]);
  else {
    requireOnlyFields(evidenceRecord, `${path}.evidence`, [
      "state",
      "formulaVersion",
      "observedCourseCount",
      "independentLearnerObservationCount",
      "difficultyIndex",
      "attemptsMean",
      "timeMedianSecondsEstimate",
      "discriminationIndex",
      "evidenceAt",
    ]);
    decodePositiveInteger(
      field(evidenceRecord, "formulaVersion", `${path}.evidence`),
      `${path}.evidence.formulaVersion`,
    );
    decodePositiveInteger(
      field(evidenceRecord, "observedCourseCount", `${path}.evidence`),
      `${path}.evidence.observedCourseCount`,
    );
    decodePositiveInteger(
      field(evidenceRecord, "independentLearnerObservationCount", `${path}.evidence`),
      `${path}.evidence.independentLearnerObservationCount`,
    );
    decodeFiniteNumber(
      field(evidenceRecord, "difficultyIndex", `${path}.evidence`),
      `${path}.evidence.difficultyIndex`,
    );
    decodeFiniteNumber(
      field(evidenceRecord, "attemptsMean", `${path}.evidence`),
      `${path}.evidence.attemptsMean`,
    );
    decodePositiveInteger(
      field(evidenceRecord, "timeMedianSecondsEstimate", `${path}.evidence`),
      `${path}.evidence.timeMedianSecondsEstimate`,
    );
    const discrimination = field(evidenceRecord, "discriminationIndex", `${path}.evidence`);
    if (discrimination !== undefined)
      decodeFiniteNumber(discrimination, `${path}.evidence.discriminationIndex`);
    decodeSafeInteger(
      field(evidenceRecord, "evidenceAt", `${path}.evidence`),
      `${path}.evidence.evidenceAt`,
    );
  }
  return {
    summary: decodeCatalogProblemSummary(field(record, "summary", path), `${path}.summary`, true),
    evidence: evidenceRecord as CatalogDiscoveryItem["evidence"],
  };
}

function definitionView(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "instructions", "entries", "defaults", "schedule"]);
  text(field(record, "title", path), `${path}.title`);
  decodeString(field(record, "instructions", path), `${path}.instructions`);
  defaults(field(record, "defaults", path), `${path}.defaults`);
  schedule(field(record, "schedule", path), `${path}.schedule`);
  decodeBoundedArray(
    field(record, "entries", path),
    `${path}.entries`,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    (entryValue, entryPath) => {
      const entryRecord = decodeRecord(entryValue, entryPath);
      const entryKind = decodeStringEnum(
        field(entryRecord, "kind", entryPath),
        `${entryPath}.kind`,
        ["fixed", "pool"],
      );
      if (entryKind === "fixed") {
        requireOnlyFields(entryRecord, entryPath, [
          "kind",
          "question",
          "points_possible",
          "scoring_mode",
        ]);
        const question = decodeRecord(
          field(entryRecord, "question", entryPath),
          `${entryPath}.question`,
        );
        requireOnlyFields(question, `${entryPath}.question`, ["catalog", "selectionAvailability"]);
        discovery(
          field(question, "catalog", `${entryPath}.question`),
          `${entryPath}.question.catalog`,
        );
        decodeStringEnum(
          field(question, "selectionAvailability", `${entryPath}.question`),
          `${entryPath}.question.selectionAvailability`,
          ["available", "retained"],
        );
      } else {
        requireOnlyFields(entryRecord, entryPath, [
          "kind",
          "candidates",
          "drawCount",
          "pointsPerItem",
          "ordering",
          "algorithm",
        ]);
        decodeBoundedArray(
          field(entryRecord, "candidates", entryPath),
          `${entryPath}.candidates`,
          MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP,
          (candidate, candidatePath) => {
            const candidateRecord = decodeRecord(candidate, candidatePath);
            requireOnlyFields(candidateRecord, candidatePath, ["catalog", "selectionAvailability"]);
            discovery(field(candidateRecord, "catalog", candidatePath), `${candidatePath}.catalog`);
            return candidate;
          },
        );
      }
      return entryValue;
    },
  );
  return value;
}

function blueprintView(value: unknown, path: string): BlueprintView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "revision", "access", "definition"]);
  definitionView(field(record, "definition", path), `${path}.definition`);
  return {
    reference: reference(field(record, "reference", path), `${path}.reference`, "BP"),
    revision: revision(field(record, "revision", path), `${path}.revision`),
    access: decodeStringEnum(field(record, "access", path), `${path}.access`, ["owner"]),
    definition: field(record, "definition", path) as BlueprintView["definition"],
  };
}

function alphaView(value: unknown, path: string): AlphaCourseView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "title",
    "revision",
    "creatorByline",
    "access",
    "modules",
  ]);
  const modules = decodeBoundedArray(
    field(record, "modules", path),
    `${path}.modules`,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    (moduleValue, modulePath) => {
      const module = decodeRecord(moduleValue, modulePath);
      requireOnlyFields(module, modulePath, ["label", "definitions"]);
      const definitions = decodeBoundedArray(
        field(module, "definitions", modulePath),
        `${modulePath}.definitions`,
        MAX_ASSIGNMENT_ORDERED_ENTRIES,
        definitionView,
      );
      return {
        label: text(field(module, "label", modulePath), `${modulePath}.label`),
        definitions: definitions as AlphaCourseView["modules"][number]["definitions"],
      };
    },
  );
  return {
    reference: reference(field(record, "reference", path), `${path}.reference`, "AC"),
    title: text(field(record, "title", path), `${path}.title`),
    revision: revision(field(record, "revision", path), `${path}.revision`),
    creatorByline: decodePublicByline(
      field(record, "creatorByline", path),
      `${path}.creatorByline`,
    ),
    access: decodeStringEnum(field(record, "access", path), `${path}.access`, [
      "creator",
      "approvedInstructor",
    ]),
    modules,
  };
}

export function decodeBlueprintView(value: unknown, path = "response"): BlueprintView {
  return blueprintView(value, path);
}
export function decodeAlphaCourseView(value: unknown, path = "response"): AlphaCourseView {
  return alphaView(value, path);
}
export function decodeBlueprintReference(value: unknown, path = "reference"): BlueprintReference {
  return reference(value, path, "BP");
}
export function decodeAlphaCourseReference(
  value: unknown,
  path = "reference",
): AlphaCourseReference {
  return reference(value, path, "AC");
}

export function decodeBlueprintPage(
  value: unknown,
  path = "response",
): CursorPage<BlueprintSummaryView> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor"]);
  return {
    items: decodeBoundedArray(
      field(record, "items", path),
      `${path}.items`,
      MAX_PAGE_SIZE,
      (item, itemPath) => {
        const itemRecord = decodeRecord(item, itemPath);
        requireOnlyFields(itemRecord, itemPath, ["reference", "title", "revision", "access"]);
        return {
          reference: reference(
            field(itemRecord, "reference", itemPath),
            `${itemPath}.reference`,
            "BP",
          ),
          title: text(field(itemRecord, "title", itemPath), `${itemPath}.title`),
          revision: revision(field(itemRecord, "revision", itemPath), `${itemPath}.revision`),
          access: decodeStringEnum(field(itemRecord, "access", itemPath), `${itemPath}.access`, [
            "owner",
          ]),
        };
      },
    ),
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeCursor,
    ),
  };
}

export function decodeAlphaCoursePage(
  value: unknown,
  path = "response",
): CursorPage<AlphaCourseSummaryView> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor"]);
  return {
    items: decodeBoundedArray(
      field(record, "items", path),
      `${path}.items`,
      MAX_PAGE_SIZE,
      (item, itemPath) => {
        const itemRecord = decodeRecord(item, itemPath);
        requireOnlyFields(itemRecord, itemPath, [
          "reference",
          "title",
          "revision",
          "creatorByline",
          "access",
        ]);
        return {
          reference: reference(
            field(itemRecord, "reference", itemPath),
            `${itemPath}.reference`,
            "AC",
          ),
          title: text(field(itemRecord, "title", itemPath), `${itemPath}.title`),
          revision: revision(field(itemRecord, "revision", itemPath), `${itemPath}.revision`),
          creatorByline: decodePublicByline(
            field(itemRecord, "creatorByline", itemPath),
            `${itemPath}.creatorByline`,
          ),
          access: decodeStringEnum(field(itemRecord, "access", itemPath), `${itemPath}.access`, [
            "creator",
            "approvedInstructor",
          ]),
        };
      },
    ),
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeCursor,
    ),
  };
}
