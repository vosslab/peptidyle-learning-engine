// reusable_curriculum_model.ts - immutable authoring operations for reusable curricula.

import type { AlphaCourseAccess } from "../../../generated/api/AlphaCourseAccess";
import type { AlphaCourseDefinitionInput } from "../../../generated/api/AlphaCourseDefinitionInput";
import type { AlphaCourseModuleInput } from "../../../generated/api/AlphaCourseModuleInput";
import type { AlphaCourseView } from "../../../generated/api/AlphaCourseView";
import type { BlueprintDefinitionInput } from "../../../generated/api/BlueprintDefinitionInput";
import type { BlueprintView } from "../../../generated/api/BlueprintView";
import type { ReusableAssignmentDefinitionInput } from "../../../generated/api/ReusableAssignmentDefinitionInput";
import type { ReusableAssignmentDefinitionView } from "../../../generated/api/ReusableAssignmentDefinitionView";
import type { ReusableAssignmentEntryInput } from "../../../generated/api/ReusableAssignmentEntryInput";
import type { ReusableAssignmentEntryView } from "../../../generated/api/ReusableAssignmentEntryView";
import type { ReusableAssignmentDefaults } from "../../../generated/api/ReusableAssignmentDefaults";
import type { RelativeAssignmentSchedule } from "../../../generated/api/RelativeAssignmentSchedule";
import type { RelativeScheduleMoment } from "../../../generated/api/RelativeScheduleMoment";
import type { ProblemPickerSelection, ProblemPickerSource } from "../problem_picker";

export const MAX_REUSABLE_ENTRIES = 1024;
export const MAX_POOL_CANDIDATES = 1024;
export const MAX_REUSABLE_TITLE_LENGTH = 200;

export type ReusableScheduleField = "availableAt" | "dueAt" | "closesAt";
export type ReusableEntryDirection = -1 | 1;

export interface CurriculumValidation {
  readonly valid: boolean;
  readonly message: string | null;
}

export interface CurriculumActionPresentation {
  readonly editable: boolean;
  readonly primaryAction: string;
  readonly guidance: string;
}

/** Shared curricula choose from the one global catalog. */
export function alphaProblemPickerSources(): ReadonlyArray<ProblemPickerSource> {
  return [{ kind: "sharedCatalog", label: "Shared catalog" }];
}

export interface CurriculumContinuationPresentation {
  readonly visible: boolean;
  readonly action: string | null;
}

/** Appends a live cursor page without duplicating an already visible curriculum record. */
export function appendCurriculumPage<Record extends { readonly reference: string }>(
  current: ReadonlyArray<Record>,
  incoming: ReadonlyArray<Record>,
): ReadonlyArray<Record> {
  const known = new Set(current.map((record) => record.reference));
  const appended = incoming.filter((record) => !known.has(record.reference));
  return [...current, ...appended];
}

/** Gives each visible cursor continuation an explicit next action. */
export function curriculumContinuationPresentation(
  kind: "blueprint" | "alpha",
  hasMore: boolean,
  retry: boolean,
): CurriculumContinuationPresentation {
  if (!hasMore) return { visible: false, action: null };
  const noun = kind === "blueprint" ? "blueprints" : "Alpha curricula";
  return { visible: true, action: retry ? `Retry loading ${noun}` : `Load more ${noun}` };
}

function defaultDefaults(): ReusableAssignmentDefaults {
  return {
    timeLimitSeconds: null,
    attemptLimit: null,
    lateSubmission: "markLate",
    deadlineBehavior: "autoSubmit",
    runPolicies: {
      completion: { kind: "answerAll" },
      grade: "highest",
      continuedPractice: { kind: "unlimited" },
      variation: "newSeeds",
    },
    student_disclosure: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      feedback_text: "after_submit",
      solution: "after_close",
      class_statistics: "never",
    },
  };
}

function emptySchedule(): RelativeAssignmentSchedule {
  return { availableAt: null, dueAt: null, closesAt: null };
}

/** Builds an editable starting definition with transparent, conservative teaching defaults. */
export function emptyReusableDefinition(
  title = "Untitled reusable assignment",
): ReusableAssignmentDefinitionInput {
  return {
    title,
    instructions: "",
    entries: [],
    defaults: defaultDefaults(),
    schedule: emptySchedule(),
  };
}

export function emptyBlueprintDefinition(): BlueprintDefinitionInput {
  return { definition: emptyReusableDefinition() };
}

export function emptyAlphaDefinition(): AlphaCourseDefinitionInput {
  return {
    title: "Untitled Alpha curriculum",
    modules: [{ label: "Module 1", definitions: [emptyReusableDefinition()] }],
  };
}

function questionIdsFromSelection(selection: ProblemPickerSelection): ReadonlyArray<string> {
  return selection.questionIds.filter(
    (questionId, index, all) => all.indexOf(questionId) === index,
  );
}

function fixedEntry(questionId: string): ReusableAssignmentEntryInput {
  return { kind: "fixed", questionId, pointsPossible: "1", scoringMode: "normal" };
}

function poolEntry(questionIds: ReadonlyArray<string>): ReusableAssignmentEntryInput {
  return {
    kind: "pool",
    candidates: [...questionIds],
    drawCount: 1,
    pointsPerItem: "1",
    ordering: "candidateOrder",
    algorithm: "v1",
  };
}

/** Appends each chosen question as its own fixed entry and retains picker order. */
export function appendPickedFixedEntries(
  definition: ReusableAssignmentDefinitionInput,
  selection: ProblemPickerSelection,
): ReusableAssignmentDefinitionInput {
  const entries = questionIdsFromSelection(selection).map(fixedEntry);
  return { ...definition, entries: [...definition.entries, ...entries] };
}

/** Appends one pool whose candidate order is the instructor's picker order. */
export function appendPickedPool(
  definition: ReusableAssignmentDefinitionInput,
  selection: ProblemPickerSelection,
): ReusableAssignmentDefinitionInput {
  const candidates = questionIdsFromSelection(selection);
  if (candidates.length === 0) return definition;
  const entry = poolEntry(candidates);
  return { ...definition, entries: [...definition.entries, entry] };
}

export function moveReusableEntry(
  definition: ReusableAssignmentDefinitionInput,
  index: number,
  direction: ReusableEntryDirection,
): ReusableAssignmentDefinitionInput {
  const destination = index + direction;
  if (index < 0 || destination < 0 || destination >= definition.entries.length) return definition;
  const entries = [...definition.entries];
  const current = entries[index];
  const adjacent = entries[destination];
  if (current === undefined || adjacent === undefined) return definition;
  entries[index] = adjacent;
  entries[destination] = current;
  return { ...definition, entries };
}

export function removeReusableEntry(
  definition: ReusableAssignmentDefinitionInput,
  index: number,
): ReusableAssignmentDefinitionInput {
  if (index < 0 || index >= definition.entries.length) return definition;
  return {
    ...definition,
    entries: definition.entries.filter((_, entryIndex) => entryIndex !== index),
  };
}

export function updateReusablePoolDrawCount(
  definition: ReusableAssignmentDefinitionInput,
  index: number,
  drawCount: number,
): ReusableAssignmentDefinitionInput {
  const entry = definition.entries[index];
  if (entry === undefined || entry.kind !== "pool") return definition;
  const entries = [...definition.entries];
  entries[index] = { ...entry, drawCount };
  return { ...definition, entries };
}

export function updateReusableSchedule(
  definition: ReusableAssignmentDefinitionInput,
  field: ReusableScheduleField,
  moment: RelativeScheduleMoment | null,
): ReusableAssignmentDefinitionInput {
  return { ...definition, schedule: { ...definition.schedule, [field]: moment } };
}

export function updateReusableDefaults(
  definition: ReusableAssignmentDefinitionInput,
  defaults: ReusableAssignmentDefaults,
): ReusableAssignmentDefinitionInput {
  return { ...definition, defaults };
}

export function updateReusableText(
  definition: ReusableAssignmentDefinitionInput,
  change: Partial<Pick<ReusableAssignmentDefinitionInput, "title" | "instructions">>,
): ReusableAssignmentDefinitionInput {
  return { ...definition, ...change };
}

function momentValue(moment: RelativeScheduleMoment | null): number | null {
  if (moment === null) return null;
  const time = /^([01]\d|2[0-3]):[0-5]\d:[0-5]\d\.\d{3}$/.exec(moment.localTime);
  if (time === null) return null;
  const [hours, minutes, seconds, milliseconds] = moment.localTime.split(/[:.]/).map(Number);
  if (
    hours === undefined ||
    minutes === undefined ||
    seconds === undefined ||
    milliseconds === undefined
  ) {
    return null;
  }
  return (
    moment.dayOffset * 86_400_000 +
    hours * 3_600_000 +
    minutes * 60_000 +
    seconds * 1_000 +
    milliseconds
  );
}

function validateSchedule(schedule: RelativeAssignmentSchedule): CurriculumValidation {
  const available = momentValue(schedule.availableAt);
  const due = momentValue(schedule.dueAt);
  const closes = momentValue(schedule.closesAt);
  if (
    [schedule.availableAt, schedule.dueAt, schedule.closesAt].some(
      (moment) => moment !== null && momentValue(moment) === null,
    )
  ) {
    return { valid: false, message: "Use local times in HH:MM:SS.sss format." };
  }
  if (
    (available !== null && due !== null && available > due) ||
    (due !== null && closes !== null && due > closes) ||
    (available !== null && closes !== null && available > closes)
  ) {
    return {
      valid: false,
      message: "Available, due, and close moments must remain in calendar order.",
    };
  }
  return { valid: true, message: null };
}

/** Client-side guidance mirrors durable meaning before the server performs authoritative validation. */
export function validateReusableDefinition(
  definition: ReusableAssignmentDefinitionInput,
): CurriculumValidation {
  const title = definition.title.trim();
  if (title.length === 0 || title.length > MAX_REUSABLE_TITLE_LENGTH) {
    return {
      valid: false,
      message: "Give this reusable assignment a title of up to 200 characters.",
    };
  }
  if (definition.entries.length === 0 || definition.entries.length > MAX_REUSABLE_ENTRIES) {
    return { valid: false, message: "Add at least one fixed question or pool before saving." };
  }
  for (const entry of definition.entries) {
    if (entry.kind !== "pool") continue;
    const unique = new Set(entry.candidates);
    if (entry.candidates.length === 0 || entry.candidates.length > MAX_POOL_CANDIDATES) {
      return { valid: false, message: "Each pool needs from 1 through 1024 candidate questions." };
    }
    if (unique.size !== entry.candidates.length) {
      return { valid: false, message: "Each pool candidate must appear only once." };
    }
    if (
      !Number.isSafeInteger(entry.drawCount) ||
      entry.drawCount < 1 ||
      entry.drawCount > entry.candidates.length
    ) {
      return {
        valid: false,
        message: "Choose a whole draw count between 1 and this pool's candidate count.",
      };
    }
  }
  return validateSchedule(definition.schedule);
}

export function validateAlphaDefinition(
  definition: AlphaCourseDefinitionInput,
): CurriculumValidation {
  const title = definition.title.trim();
  if (title.length === 0 || title.length > MAX_REUSABLE_TITLE_LENGTH) {
    return { valid: false, message: "Give this Alpha curriculum a title of up to 200 characters." };
  }
  if (definition.modules.length === 0 || definition.modules.length > MAX_REUSABLE_ENTRIES) {
    return { valid: false, message: "Add at least one labelled module before saving." };
  }
  for (const module of definition.modules) {
    if (module.label.trim().length === 0 || module.label.length > MAX_REUSABLE_TITLE_LENGTH) {
      return { valid: false, message: "Give each module a label of up to 200 characters." };
    }
    if (module.definitions.length === 0 || module.definitions.length > MAX_REUSABLE_ENTRIES) {
      return { valid: false, message: "Each module needs at least one reusable assignment." };
    }
    for (const reusableDefinition of module.definitions) {
      const validation = validateReusableDefinition(reusableDefinition);
      if (!validation.valid) return validation;
    }
  }
  return { valid: true, message: null };
}

/** Keeps access state visible and task-oriented for creators and approved readers. */
export function alphaActionPresentation(access: AlphaCourseAccess): CurriculumActionPresentation {
  if (access === "creator") {
    return {
      editable: true,
      primaryAction: "Save curriculum",
      guidance: "Edit modules and reusable assignments, then save the current curriculum.",
    };
  }
  return {
    editable: false,
    primaryAction: "Inspect and reuse question set",
    guidance:
      "Review the answer-free modules, then choose published questions for your own course.",
  };
}

export function appendAlphaModule(
  definition: AlphaCourseDefinitionInput,
): AlphaCourseDefinitionInput {
  const module: AlphaCourseModuleInput = {
    label: `Module ${definition.modules.length + 1}`,
    definitions: [emptyReusableDefinition()],
  };
  return { ...definition, modules: [...definition.modules, module] };
}

export function moveAlphaModule(
  definition: AlphaCourseDefinitionInput,
  index: number,
  direction: ReusableEntryDirection,
): AlphaCourseDefinitionInput {
  const destination = index + direction;
  if (index < 0 || destination < 0 || destination >= definition.modules.length) return definition;
  const modules = [...definition.modules];
  const current = modules[index];
  const adjacent = modules[destination];
  if (current === undefined || adjacent === undefined) return definition;
  modules[index] = adjacent;
  modules[destination] = current;
  return { ...definition, modules };
}

export function removeAlphaModule(
  definition: AlphaCourseDefinitionInput,
  index: number,
): AlphaCourseDefinitionInput {
  if (index < 0 || index >= definition.modules.length) return definition;
  return {
    ...definition,
    modules: definition.modules.filter((_, moduleIndex) => moduleIndex !== index),
  };
}

export function appendAlphaDefinition(
  definition: AlphaCourseDefinitionInput,
  moduleIndex: number,
): AlphaCourseDefinitionInput {
  const module = definition.modules[moduleIndex];
  if (module === undefined) return definition;
  const modules = [...definition.modules];
  modules[moduleIndex] = {
    ...module,
    definitions: [...module.definitions, emptyReusableDefinition()],
  };
  return { ...definition, modules };
}

export function moveAlphaDefinition(
  definition: AlphaCourseDefinitionInput,
  moduleIndex: number,
  definitionIndex: number,
  direction: ReusableEntryDirection,
): AlphaCourseDefinitionInput {
  const module = definition.modules[moduleIndex];
  const destination = definitionIndex + direction;
  if (
    module === undefined ||
    definitionIndex < 0 ||
    destination < 0 ||
    destination >= module.definitions.length
  ) {
    return definition;
  }
  const definitions = [...module.definitions];
  const current = definitions[definitionIndex];
  const adjacent = definitions[destination];
  if (current === undefined || adjacent === undefined) return definition;
  definitions[definitionIndex] = adjacent;
  definitions[destination] = current;
  return updateAlphaModule(definition, moduleIndex, { definitions });
}

export function removeAlphaDefinition(
  definition: AlphaCourseDefinitionInput,
  moduleIndex: number,
  definitionIndex: number,
): AlphaCourseDefinitionInput {
  const module = definition.modules[moduleIndex];
  if (module === undefined || definitionIndex < 0 || definitionIndex >= module.definitions.length) {
    return definition;
  }
  const definitions = module.definitions.filter((_, index) => index !== definitionIndex);
  return updateAlphaModule(definition, moduleIndex, { definitions });
}

export function updateAlphaModule(
  definition: AlphaCourseDefinitionInput,
  moduleIndex: number,
  change: Partial<AlphaCourseModuleInput>,
): AlphaCourseDefinitionInput {
  const module = definition.modules[moduleIndex];
  if (module === undefined) return definition;
  const modules = [...definition.modules];
  modules[moduleIndex] = { ...module, ...change };
  return { ...definition, modules };
}

export function updateAlphaDefinition(
  definition: AlphaCourseDefinitionInput,
  moduleIndex: number,
  definitionIndex: number,
  nextDefinition: ReusableAssignmentDefinitionInput,
): AlphaCourseDefinitionInput {
  const module = definition.modules[moduleIndex];
  if (module === undefined || module.definitions[definitionIndex] === undefined) return definition;
  const definitions = [...module.definitions];
  definitions[definitionIndex] = nextDefinition;
  return updateAlphaModule(definition, moduleIndex, { definitions });
}

function entryInputFromView(entry: ReusableAssignmentEntryView): ReusableAssignmentEntryInput {
  if (entry.kind === "pool") {
    return {
      kind: "pool",
      candidates: entry.candidates.map((candidate) => candidate.catalog.summary.questionId),
      drawCount: entry.drawCount,
      pointsPerItem: entry.pointsPerItem,
      ordering: entry.ordering,
      algorithm: entry.algorithm,
    };
  }
  return {
    kind: "fixed",
    questionId: entry.question.catalog.summary.questionId,
    pointsPossible: entry.points_possible,
    scoringMode: entry.scoring_mode,
  };
}

export function reusableDefinitionInputFromView(
  definition: ReusableAssignmentDefinitionView,
): ReusableAssignmentDefinitionInput {
  return {
    title: definition.title,
    instructions: definition.instructions,
    entries: definition.entries.map(entryInputFromView),
    defaults: definition.defaults,
    schedule: definition.schedule,
  };
}

export function blueprintInputFromView(view: BlueprintView): BlueprintDefinitionInput {
  return { definition: reusableDefinitionInputFromView(view.definition) };
}

export function alphaInputFromView(view: AlphaCourseView): AlphaCourseDefinitionInput {
  return {
    title: view.title,
    modules: view.modules.map((module) => ({
      label: module.label,
      definitions: module.definitions.map(reusableDefinitionInputFromView),
    })),
  };
}
