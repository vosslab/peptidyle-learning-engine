// scenario_types.ts - executable scenario registry contract.

import type { PrivacyProfileId, ViewportId } from "./manifest";
import type { ScenarioRuntime } from "./runtime";

export interface CaptureDeclaration {
  readonly checkpoint: string;
  readonly area: string;
  readonly workflow: string;
  readonly state: string;
  readonly viewport: ViewportId;
  readonly privacyProfile: PrivacyProfileId;
  readonly caption: string;
  readonly featured?: boolean;
}

export interface ScenarioDefinition {
  readonly id: string;
  readonly role: "public" | "instructor" | "student" | "sysadmin";
  readonly captures: ReadonlyArray<CaptureDeclaration>;
  readonly run: (runtime: ScenarioRuntime) => Promise<void>;
}
