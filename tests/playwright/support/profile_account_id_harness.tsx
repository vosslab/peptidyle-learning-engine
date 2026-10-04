import { render } from "solid-js/web";

import { ProfileAccountId } from "../../../src/pages/profile_account_id";

export function mountProfileAccountId(target: HTMLElement, accountId: string): void {
  render(() => <ProfileAccountId accountId={accountId} />, target);
}
