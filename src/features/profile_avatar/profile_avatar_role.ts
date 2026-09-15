// profile_avatar_role.ts - browser presentation gate for self-avatar image controls.

import type { ProductRole } from "../../../generated/api/ProductRole";

/** Browser presentation gate only; the self-avatar server route remains authoritative. */
export function profileRoleMayManageImage(role: ProductRole): boolean {
  return role === "instructor" || role === "sysadmin";
}
