/** Browser-safe, role-neutral settings returned for the authenticated Account. */
export interface ProfileSettings {
  readonly timeZone: string;
}

/** Closed browser input for the authenticated Account's sole settings preference. */
export interface UpdateAccountSettingsInput {
  readonly timeZone: string;
}
