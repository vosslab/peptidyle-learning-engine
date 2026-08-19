// capability_validation.ts - server-free MOD-CAP fallback for the reference UI.

import type { Capability } from "../../../generated/api/Capability";
import type { AssignmentConfig, CapabilityViolation, CapabilityValidator } from "../../wasm/index";

const CAPABILITIES: ReadonlyArray<Capability> = [
  "algorithmicGeneration",
  "clientRendering",
  "serverGrading",
  "partialCredit",
  "hints",
  "perQuestionTiming",
  "printExport",
  "offlinePreview",
];

function requiredByQuestion(selected: AssignmentConfig["questions"][number]): Set<Capability> {
  const required = new Set<Capability>();
  if (selected.question.randomization.kind === "seeded") {
    required.add("algorithmicGeneration");
  }
  switch (selected.question.grading.mode) {
    case "allOrNothing":
      required.add("serverGrading");
      break;
    case "partialCredit":
      required.add("serverGrading");
      required.add("partialCredit");
      break;
    case "ungraded":
      break;
  }
  if (selected.question.timingPolicy.kind === "perQuestion") {
    required.add("perQuestionTiming");
  }
  return required;
}

/** Mirrors Rust MOD-CAP only while the reference client has no API server. */
export const validateAssignmentConfigInMock: CapabilityValidator = (
  config,
): Promise<ReadonlyArray<CapabilityViolation>> => {
  const violations: CapabilityViolation[] = [];
  for (const selected of config.questions) {
    const required = requiredByQuestion(selected);
    for (const capability of config.requiredCapabilities) {
      required.add(capability);
    }
    const supported = new Set(selected.backendCapabilities);
    for (const capability of CAPABILITIES) {
      if (required.has(capability) && !supported.has(capability)) {
        violations.push({ question: selected.question.version, capability });
      }
    }
  }
  return Promise.resolve(violations);
};
