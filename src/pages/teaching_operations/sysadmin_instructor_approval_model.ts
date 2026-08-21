import type { SysadminInstructorCandidateView } from "../../../generated/api/SysadminInstructorCandidateView";
import type { TeachingOperationRevision } from "../../../generated/api/TeachingOperationRevision";

export type InstructorApprovalAction = "approve" | "revoke";

export function isCandidateQueryEligible(query: string): boolean {
  return Array.from(query.trim()).length >= 2;
}

export function appendInstructorCandidatePage(
  current: ReadonlyArray<SysadminInstructorCandidateView>,
  next: ReadonlyArray<SysadminInstructorCandidateView>,
): ReadonlyArray<SysadminInstructorCandidateView> {
  const references = new Set(current.map((candidate) => candidate.account.reference));
  const unseen = next.filter((candidate) => !references.has(candidate.account.reference));
  return [...current, ...unseen];
}

export function candidateAction(
  candidate: SysadminInstructorCandidateView,
): InstructorApprovalAction | undefined {
  return candidate.approval.state === "approved" ? "revoke" : "approve";
}

/** An unapproved account deliberately sends no revision; recorded states require their exact revision. */
export function candidateActionRevision(
  candidate: SysadminInstructorCandidateView,
): TeachingOperationRevision | undefined {
  return candidate.approval.state === "unapproved"
    ? undefined
    : (candidate.approval.revision ?? undefined);
}

export function candidateApprovalLabel(candidate: SysadminInstructorCandidateView): string {
  switch (candidate.approval.state) {
    case "unapproved":
      return "Not approved";
    case "approved":
      return "Approved for invitations";
    case "revoked":
      return "Approval revoked";
  }
}

export function candidateActionLabel(action: InstructorApprovalAction): string {
  return action === "approve" ? "Approve as instructor" : "Revoke approval";
}

export function approvalSuccessCopy(display: string, action: InstructorApprovalAction): string {
  if (action === "revoke") return `${display} is no longer eligible for course invitations.`;
  return `${display} is eligible for a course invitation. This did not add ${display} to a course.`;
}

export function approvalFailureCopy(status: number | undefined): string {
  if (status === 403) return "You do not have permission to change instructor approval.";
  if (status === 409 || status === 412)
    return "The approval state changed. Results were refreshed.";
  return "The approval change could not be completed. Check your connection and try again.";
}

export function approvalReloadRequired(status: number | undefined): boolean {
  return status === 409 || status === 412;
}
