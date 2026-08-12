// live_mode_activation.ts - pure fail-closed selection for private browser gates.

export type Environment = Readonly<Record<string, string | undefined>>;

export interface LiveModeActivation {
  readonly webwork: boolean;
}

function activationValue(environment: Environment, name: string): boolean {
  const value = environment[name];
  if (value === undefined || value === "" || value === "0") return false;
  if (value === "1") return true;
  throw new Error(`${name} must be exactly 1 when set`);
}

/** Validates the separate WebWork live switch before its input parser reads a credential. */
export function liveModeActivationFromEnvironment(environment: Environment): LiveModeActivation {
  const webwork = activationValue(environment, "PLE_WEBWORK_LIVE_REQUIRED");
  return { webwork };
}
