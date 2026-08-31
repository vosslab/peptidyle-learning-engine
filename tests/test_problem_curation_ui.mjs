import assert from "node:assert/strict";
import test from "node:test";

import {
  collectionDeletionFromObserved,
  problemCurationConfirmationPresentation,
} from "../src/features/problem_curation/problem_curation_model.ts";

test("curation deletion presents one named dialog with Cancel as the initial action", () => {
  const deletion = collectionDeletionFromObserved({
    reference: "PC-7",
    kind: "named",
    title: "Exam candidates",
    visibility: "private",
    editNumber: "7",
    access: "owner",
  });
  const presentation = problemCurationConfirmationPresentation(deletion);

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
      heading: 'Delete collection "Exam candidates"?',
      objectConsequence:
        "Deleting this collection removes its saved ordered question list. Published questions remain available in the Library.",
      actions: [
        { kind: "cancel", label: "Cancel", initial: true },
        { kind: "confirm", label: "Delete collection", initial: false },
      ],
    },
  );
});
