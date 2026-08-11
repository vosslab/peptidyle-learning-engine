// failure_triage.ts - fixed, public-only advisory categories for walkthrough failures.

const STAGES = [
  "runner-preflight",
  "gateway-readiness",
  "visible-target",
  "keyboard-navigation",
  "visible-outcome",
] as const;
const DIAGNOSTICS = [
  "configuration-invalid",
  "gateway-unavailable",
  "visible-control-unavailable",
  "keyboard-target-unavailable",
  "visible-state-unavailable",
] as const;
export type FailureStage = (typeof STAGES)[number];
export type FailureDiagnostic = (typeof DIAGNOSTICS)[number];
export type FailureTriageCategory =
  | "configuration"
  | "gateway"
  | "selector"
  | "keyboard"
  | "visible-outcome-mismatch"
  | "unclassified";

export interface FailureTriageInput {
  readonly stage: FailureStage;
  readonly diagnostic: FailureDiagnostic;
}

export interface FailureTriageHint {
  readonly category: FailureTriageCategory;
}

interface TriageRule {
  readonly stage: FailureStage;
  readonly diagnostic: FailureDiagnostic;
  readonly category: Exclude<FailureTriageCategory, "unclassified">;
}

const RULES: readonly TriageRule[] = [
  {
    stage: "runner-preflight",
    diagnostic: "configuration-invalid",
    category: "configuration",
  },
  {
    stage: "gateway-readiness",
    diagnostic: "gateway-unavailable",
    category: "gateway",
  },
  {
    stage: "visible-target",
    diagnostic: "visible-control-unavailable",
    category: "selector",
  },
  {
    stage: "keyboard-navigation",
    diagnostic: "keyboard-target-unavailable",
    category: "keyboard",
  },
  {
    stage: "visible-outcome",
    diagnostic: "visible-state-unavailable",
    category: "visible-outcome-mismatch",
  },
];

/**
 * Returns a fixed advisory category only. It never receives or changes a journey or report outcome.
 */
export function classifyFailureHint(value: unknown): FailureTriageHint {
  const input = parseFailureTriageInput(value);
  const matchingRule = input === undefined ? undefined : findMatchingRule(input);
  const category = matchingRule?.category ?? "unclassified";
  return { category };
}

/** Validates the closed stage and diagnostic vocabulary before triage uses it. */
export function parseFailureTriageInput(value: unknown): FailureTriageInput | undefined {
  if (!isRecord(value) || !hasExactKeys(value, ["stage", "diagnostic"])) return undefined;
  if (!isStage(value.stage) || !isDiagnostic(value.diagnostic)) return undefined;
  return { stage: value.stage, diagnostic: value.diagnostic };
}

function findMatchingRule(input: FailureTriageInput): TriageRule | undefined {
  return RULES.find((rule) => rule.stage === input.stage && rule.diagnostic === input.diagnostic);
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(value: Readonly<Record<string, unknown>>, keys: readonly string[]): boolean {
  const actual = Reflect.ownKeys(value);
  return (
    actual.length === keys.length &&
    actual.every(
      (key) =>
        typeof key === "string" &&
        keys.includes(key) &&
        Object.prototype.propertyIsEnumerable.call(value, key),
    )
  );
}

function isStage(value: unknown): value is FailureStage {
  return typeof value === "string" && STAGES.includes(value as FailureStage);
}

function isDiagnostic(value: unknown): value is FailureDiagnostic {
  return typeof value === "string" && DIAGNOSTICS.includes(value as FailureDiagnostic);
}
