// student_response.ts - key-free display projection of a Student's submitted response.

import type { ContentBlock } from "../../../generated/api/ContentBlock";
import type { QuestionEnvelope } from "../../../generated/api/QuestionEnvelope";
import type { StudentResponse } from "../../../generated/api/StudentResponse";

function text(markdown: string): ContentBlock {
  return { kind: "text", markdown };
}

function blockText(blocks: ReadonlyArray<ContentBlock>): string {
  return blocks
    .map((block) => {
      switch (block.kind) {
        case "text":
          return block.markdown;
        case "math":
        case "image":
        case "table":
          return block.description;
        case "code":
          return block.source;
      }
    })
    .join(" ");
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
 * Student response instead of guessing at a result.
 */
export function projectStudentResponse(
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
    case "multiBlank": {
      if (definition.kind !== "multiBlank") return [];
      const labels = new Map(definition.blanks.map((blank) => [blank.id, blockText(blank.label)]));
      return [
        {
          kind: "table",
          headers: ["Blank", "Your response"],
          rows: response.answers.map((answer) => [
            labels.get(answer.slot) ?? "Unknown blank",
            answer.text,
          ]),
          description: "Your responses for each blank",
        },
      ];
    }
    case "matching": {
      if (definition.kind !== "matching") return [];
      const prompts = new Map(
        definition.prompts.map((prompt) => [prompt.id, blockText(prompt.body)]),
      );
      const choices = new Map(
        definition.choices.map((choice) => [choice.id, blockText(choice.body)]),
      );
      return [
        {
          kind: "table",
          headers: ["Prompt", "Your match"],
          rows: response.matches.map((pair) => [
            prompts.get(pair.prompt) ?? "Unknown prompt",
            choices.get(pair.choice) ?? "Unknown choice",
          ]),
          description: "Your matching response",
        },
      ];
    }
    case "hotspot": {
      if (definition.kind !== "hotspot") return [];
      const selected = response.points.flatMap((point) =>
        definition.regions.filter(
          (region) =>
            point.x >= region.x &&
            point.x <= region.x + region.width &&
            point.y >= region.y &&
            point.y <= region.y + region.height,
        ),
      );
      return selected.flatMap((region) => region.label);
    }
    case "fileUpload":
      return [text("A file was submitted.")];
    case "externalTool":
      return [text("Your external-tool response was recorded.")];
  }
}
