// Browser contract for explicitly confirmed Sysadmin access to one Student roster record.

import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { AccountId } from "../../generated/api/AccountId";

/** One audited Student roster record read by a Sysadmin for administrative work. */
export interface SysadminStudentDataAccess {
  readonly courseInstanceId: CourseInstanceId;
  readonly studentAccountId: AccountId;
  readonly rosterId: string;
  readonly rosterName: string;
  readonly state: "invitationPending" | "activeStudent" | "removed";
  readonly audit: {
    readonly eventId: string;
    readonly occurredAt: number;
  };
}

/** Same-origin capability for one confirmed Sysadmin Student-record read. */
export interface SysadminStudentAccessClient {
  readonly accessSysadminStudentData: (
    courseInstanceId: CourseInstanceId,
    rosterId: string,
  ) => Promise<SysadminStudentDataAccess>;
}
