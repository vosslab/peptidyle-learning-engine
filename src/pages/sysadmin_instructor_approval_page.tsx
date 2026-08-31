// sysadmin_instructor_approval_page.tsx - platform-scoped Instructor approval workspace.

import type { JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { SysadminInstructorApprovalPanel } from "./teaching_operations/sysadmin_instructor_approval_panel";

/** Hosts Instructor approval outside every Course Instance and Student-record surface. */
export function SysadminInstructorApprovalPage(): JSX.Element {
  const runtime = useApplicationApi();

  return (
    <section class="page" data-route-surface="sysadminInstructorApproval">
      <p class="eyebrow">Sysadmin tools</p>
      <h1>Instructor approvals</h1>
      <p class="page-lede">Review Instructor approval without opening a Course Instance.</p>
      <SysadminInstructorApprovalPanel applicationApi={runtime} />
    </section>
  );
}
