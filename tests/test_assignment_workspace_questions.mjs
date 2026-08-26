// Stable Questions-workspace model contracts; connected browser journeys cover visible controls.

import assert from "node:assert/strict";
import test from "node:test";

import {
  assignmentContentInput,
  createMasteryAssignmentDraft,
} from "../src/pages/assignment_editor_model.ts";
import { createdAssignmentQuestionsPath } from "../src/pages/assignment_workspace/assignment_workspace_create_model.ts";
import { parseAssignmentReference, parseCourseReference } from "../src/navigation/public_route.ts";

test("Questions save owns only the title and ordered public definition", () => {
  const draft = {
    ...createMasteryAssignmentDraft("course-1"),
    title: "Protein bonds",
    entries: [
      {
        kind: "fixed",
        id: "item-1",
        questionId: "7K3-M9QP",
        title: "Peptide bond resonance",
        backend: "native",
        capabilities: [],
        position: 0,
        pointsPossible: "1",
        deliveryState: "active",
        scoringMode: "normal",
      },
    ],
  };

  assert.deepEqual(assignmentContentInput(draft), {
    title: "Protein bonds",
    entries: [
      {
        kind: "fixed",
        questionId: "7K3-M9QP",
        pointsPossible: "1",
        deliveryState: "active",
        scoringMode: "normal",
        position: 0,
      },
    ],
  });
});

test("persisted draft creation enters the canonical Questions route", () => {
  const course = parseCourseReference("C-8");
  const assignment = parseAssignmentReference("A-15");
  assert.ok(course);
  assert.ok(assignment);
  assert.equal(
    createdAssignmentQuestionsPath(course, assignment),
    "/instructor/courses/C-8/assignments/A-15/questions",
  );
});
