import assert from "node:assert/strict";
import test from "node:test";

import {
  folderDeletionFromObserved,
  questionCurationConfirmationPresentation,
} from "../src/features/question_curation/question_curation_model.ts";

test("curation deletion presents one named dialog with Cancel as the initial action", () => {
  const deletion = folderDeletionFromObserved({
    reference: "QC-7",
    kind: "named",
    title: "Exam candidates",
    visibility: "private",
    editNumber: "7",
    access: "owner",
  });
  const presentation = questionCurationConfirmationPresentation(deletion);

  assert.deepEqual(
    {
      labelledBy: presentation.labelledBy,
      describedBy: presentation.describedBy,
      heading: presentation.heading,
      objectConsequence: presentation.consequence,
      actions: presentation.actions,
    },
    {
      labelledBy: "curation-delete-heading",
      describedBy: "curation-delete-consequence",
      heading: 'Delete Question Folder "Exam candidates"?',
      objectConsequence:
        "Deleting this Question Folder removes its saved ordered Question list. Published Questions remain available in the Question Library.",
      actions: [
        { kind: "cancel", label: "Cancel", initial: true },
        { kind: "confirm", label: "Delete Question Folder", initial: false },
      ],
    },
  );
});
