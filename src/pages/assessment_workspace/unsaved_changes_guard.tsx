import {
  UnsavedChangesGuard as SharedUnsavedChangesGuard,
  type UnsavedChangesGuardProps as SharedUnsavedChangesGuardProps,
} from "../../components/unsaved_changes_guard";
import type { JSX } from "solid-js";

type UnsavedChangesGuardProps = Omit<SharedUnsavedChangesGuardProps, "copy">;

/** Guards browser navigation while the Assessment Question Editor has local structural edits. */
export function UnsavedChangesGuard(props: UnsavedChangesGuardProps): JSX.Element {
  return (
    <SharedUnsavedChangesGuard
      {...props}
      copy={{
        heading: "Save Assessment Question changes?",
        description:
          "Your Question order, additions, removals, or title changes have not been saved.",
        saveActionLabel: "Save and continue",
        savingActionLabel: "Saving Questions...",
        saveFailureMessage:
          "Questions were not saved. Resolve the save error shown on the page, then try again or stay here.",
      }}
    />
  );
}
