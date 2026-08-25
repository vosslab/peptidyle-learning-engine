// reusable_curriculum_creation.ts - guarded live creation commands for locally authored drafts.

import type { AlphaCourseDefinitionInput } from "../../../generated/api/AlphaCourseDefinitionInput";
import type { BlueprintDefinitionInput } from "../../../generated/api/BlueprintDefinitionInput";
import type { ReusableCurriculumClient } from "../../api/reusable_curriculum";
import { validateAlphaDefinition, validateReusableDefinition } from "./reusable_curriculum_model";

export type CurriculumCreationResult<Created> =
  | { readonly kind: "invalid"; readonly message: string }
  | { readonly kind: "created"; readonly value: Created };

/** Validates a browser-local blueprint before making its one live mutation request. */
export async function createBlueprintWhenReady(
  client: ReusableCurriculumClient,
  definition: BlueprintDefinitionInput,
): Promise<
  CurriculumCreationResult<Awaited<ReturnType<ReusableCurriculumClient["createBlueprint"]>>>
> {
  const validation = validateReusableDefinition(definition.definition);
  if (!validation.valid) {
    return {
      kind: "invalid",
      message: validation.message ?? "Complete the blueprint before creating it.",
    };
  }
  const value = await client.createBlueprint(definition);
  return { kind: "created", value };
}

/** Validates a browser-local Alpha aggregate before making its one live mutation request. */
export async function createAlphaWhenReady(
  client: ReusableCurriculumClient,
  definition: AlphaCourseDefinitionInput,
): Promise<
  CurriculumCreationResult<Awaited<ReturnType<ReusableCurriculumClient["createAlphaCourse"]>>>
> {
  const validation = validateAlphaDefinition(definition);
  if (!validation.valid) {
    return {
      kind: "invalid",
      message: validation.message ?? "Complete the Alpha curriculum before creating it.",
    };
  }
  const value = await client.createAlphaCourse(definition);
  return { kind: "created", value };
}
