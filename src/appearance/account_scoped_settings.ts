// account_scoped_settings.ts - Account identity boundary for appearance settings responses.

import type { AccountId } from "../../generated/api/AccountId";
import type { ProfileSettings } from "../api/profile_settings";

export interface AccountScopedSettings {
  readonly accountId: AccountId;
  readonly settings: ProfileSettings;
}

/** Returns settings only when their response belongs to the authenticated Account now in view. */
export function settingsForCurrentAccount(
  load: AccountScopedSettings | undefined,
  accountId: AccountId | undefined,
): ProfileSettings | undefined {
  return load !== undefined && load.accountId === accountId ? load.settings : undefined;
}

/** Admits an appearance mutation response only to the Account that started the mutation. */
export function currentAccountSettingsUpdate(
  accountId: AccountId,
  currentAccountId: AccountId | undefined,
  settings: ProfileSettings,
): AccountScopedSettings | undefined {
  return accountId === currentAccountId ? { accountId, settings } : undefined;
}
