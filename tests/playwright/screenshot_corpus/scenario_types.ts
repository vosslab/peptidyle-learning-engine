// scenario_types.ts - executable scenario registry contract.

import { VIEWPORT_IDS, type PrivacyProfileId, type ViewportId } from "./manifest";
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

export type ScenarioViewportCoverage =
  | { readonly status: "captured" }
  | {
      readonly status: "covered_by";
      readonly target: string;
      readonly reason: string;
    };

export interface ScenarioViewportCoveredBy {
  readonly target: string;
  readonly reason: string;
}

/** Create four direct declarations for one responsive proof checkpoint. */
export function directViewportCaptures(
  capture: CaptureDeclaration,
): ReadonlyArray<CaptureDeclaration> {
  const checkpointStem = capture.checkpoint.replace(/_laptop$/u, "");
  return VIEWPORT_IDS.map((viewport) =>
    viewport === capture.viewport
      ? capture
      : {
          ...capture,
          checkpoint: `${checkpointStem}_${viewport}`,
          viewport,
          caption: `${capture.caption} on a ${viewport}`,
          featured: false,
        },
  );
}

/**
 * Declare direct captures or an honest representative substitution for every viewport.
 *
 * A direct capture takes precedence. `covered_by` records the existing checkpoint
 * used for review while the omitted viewport remains unverified pending its own capture.
 */
export function viewportCoverage(
  capturedViewports: ReadonlyArray<ViewportId>,
  coveredBy: Readonly<Partial<Record<ViewportId, ScenarioViewportCoveredBy>>>,
): Readonly<Record<ViewportId, ScenarioViewportCoverage>> {
  const captured = new Set(capturedViewports);
  return Object.fromEntries(
    VIEWPORT_IDS.map((viewport) => {
      if (captured.has(viewport)) return [viewport, { status: "captured" }];
      const coverage = coveredBy[viewport];
      if (coverage === undefined) {
        throw new Error(`scenario viewport ${viewport} needs a captured or covered_by declaration`);
      }
      return [viewport, { status: "covered_by", ...coverage }];
    }),
  ) as Readonly<Record<ViewportId, ScenarioViewportCoverage>>;
}

export interface ScenarioDefinition {
  readonly id: string;
  readonly role: "public" | "instructor" | "student" | "sysadmin";
  readonly captures: ReadonlyArray<CaptureDeclaration>;
  readonly viewportCoverage: Readonly<Record<ViewportId, ScenarioViewportCoverage>>;
  readonly run: (runtime: ScenarioRuntime) => Promise<void>;
}
