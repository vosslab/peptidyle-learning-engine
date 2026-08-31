// Guarded Blueprint Course creation from a complete local draft.

import type { CreateBlueprintCourseDefinitionInput } from "../../../generated/api/CreateBlueprintCourseDefinitionInput";
import type { ReusableCurriculumClient } from "../../api/reusable_curriculum";
import { validateBlueprintCourseDefinition } from "./reusable_curriculum_model";

export type BlueprintCourseCreationResult<Created> =
  | { readonly kind: "invalid"; readonly message: string }
  | { readonly kind: "created"; readonly value: Created };

/** Validates a complete Blueprint Course draft before its one live create request. */
export async function createBlueprintCourseWhenReady(
  client: ReusableCurriculumClient,
  definition: CreateBlueprintCourseDefinitionInput,
): Promise<
  BlueprintCourseCreationResult<
    Awaited<ReturnType<ReusableCurriculumClient["createBlueprintCourse"]>>
  >
> {
  const validation = validateBlueprintCourseDefinition(definition);
  if (!validation.valid) {
    return {
      kind: "invalid",
      message: validation.message ?? "Complete the Blueprint Course before creating it.",
    };
  }
  return { kind: "created", value: await client.createBlueprintCourse(definition) };
}
