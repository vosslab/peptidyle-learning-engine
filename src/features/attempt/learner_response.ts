// learner_response.ts - key-free display projection of a learner's submitted response.

import type { ContentBlock } from "../../../generated/api/ContentBlock";
import type { QuestionEnvelope } from "../../../generated/api/QuestionEnvelope";
import type { StudentResponse } from "../../../generated/api/StudentResponse";

function text(markdown: string): ContentBlock {
  return { kind: "text", markdown };
}

function selectedBodies(
  options: ReadonlyArray<{ readonly id: string; readonly body: Array<ContentBlock> }>,
  ids: ReadonlyArray<string>,
): ReadonlyArray<ContentBlock> | null {
  const byId = new Map(options.map((option) => [option.id, option.body]));
  const bodies: ContentBlock[] = [];
  for (const id of ids) {
    const body = byId.get(id);
    if (body === undefined) return null;
    bodies.push(...body);
  }
  return bodies;
}

/**
 * Makes an inert, public-only summary for the feedback screen. It has no key,
 * scoring, or object-storage access: malformed pairings deliberately render no
 * learner response instead of guessing at a result.
 */
export function projectLearnerResponse(
  envelope: QuestionEnvelope,
  response: StudentResponse | null,
): ReadonlyArray<ContentBlock> {
  if (response === null || envelope.response.kind !== response.kind) return [];
  const definition = envelope.response;
  switch (response.kind) {
    case "multipleChoice": {
      if (definition.kind !== "multipleChoice") return [];
      const bodies = selectedBodies(definition.choices, response.selected);
      return bodies ?? [];
    }
    case "ordering": {
      if (definition.kind !== "ordering") return [];
      const bodies = selectedBodies(definition.items, response.order);
      return bodies ?? [];
    }
    case "numeric":
      return Number.isFinite(response.value) ? [text(String(response.value))] : [];
    case "shortText":
      return [text(response.text)];
    case "fileUpload":
      return [text("A file was submitted.")];
    case "externalTool":
      return [text("Your external-tool response was recorded.")];
  }
}
