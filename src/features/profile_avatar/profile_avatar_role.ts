// profile_avatar_role.ts - browser presentation gate for self-avatar image controls.

import type { UserRole } from "../../../generated/api/UserRole";

/** Browser presentation gate only; the self-avatar server route remains authoritative. */
export function profileRoleMayManageImage(role: UserRole): boolean {
  return role === "instructor" || role === "sysadmin";
}
