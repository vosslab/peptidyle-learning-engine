// Guarded Blueprint Course creation from complete local working state.

import type { CreateBlueprintCourseInput } from "../../../generated/api/CreateBlueprintCourseInput";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { validateBlueprintCourseContent } from "./blueprint_course_model";
import { decodeCourseClassification } from "../../api/decoders/course_classification";

export type BlueprintCourseCreationResult<Created> =
  | { readonly kind: "invalid"; readonly message: string }
  | { readonly kind: "created"; readonly value: Created };

/** Validates complete Blueprint Course working state before its one live create request. */
export async function createBlueprintCourseWhenReady(
  client: BlueprintCourseClient,
  content: CreateBlueprintCourseInput,
  idempotencyKey: string,
): Promise<
  BlueprintCourseCreationResult<Awaited<ReturnType<BlueprintCourseClient["createBlueprintCourse"]>>>
> {
  try {
    decodeCourseClassification(content.classification);
  } catch {
    return {
      kind: "invalid",
      message:
        "Choose a Discipline and valid Course classification before creating the Blueprint Course.",
    };
  }
  const validation = validateBlueprintCourseContent(content);
  if (!validation.valid) {
    return {
      kind: "invalid",
      message: validation.message ?? "Complete the Blueprint Course before creating it.",
    };
  }
  return { kind: "created", value: await client.createBlueprintCourse(content, idempotencyKey) };
}
