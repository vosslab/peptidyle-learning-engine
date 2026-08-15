import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import { createMockApiClient } from "../src/api/mock/client.ts";
import { AssignmentConflictError } from "../src/api/http_client/error.ts";

test("revisioned assignment commands carry QIDs and return fresh safe summaries", async () => {
  const client = createMockApiClient({ assignmentAuthoring: true });
  const assignment = await client.getAssignmentEditor(publishedProblemFixture.assignment.id);
  const item = assignment.items[0];
  assert.ok(item);
  const saved = await client.saveAssignment(
    assignment.courseId,
    assignment.id,
    {
      title: "Renamed without changing Question ID",
      items: assignment.items.map(
        ({ id, questionId, position, pointsPossible, deliveryState, scoringMode }) => ({
          id,
          questionId,
          position,
          pointsPossible,
          deliveryState,
          scoringMode,
        }),
      ),
      policies: assignment.policies,
      assignmentTiming: { timeLimitSeconds: null },
    },
    assignment.revision,
  );
  assert.equal(saved.title, "Renamed without changing Question ID");
  assert.equal(saved.items[0]?.questionId, item.questionId);
  const replaced = await client.replaceAssignmentItemQuestion(
    saved.courseId,
    saved.id,
    item.id,
    { questionId: publishedProblemFixture.catalogProblem.questionId },
    saved.revision,
  );
  assert.equal(replaced.items[0]?.questionId, publishedProblemFixture.catalogProblem.questionId);
  assert.equal(replaced.items[0]?.id, item.id);
  await assert.rejects(
    client.removeAssignmentItem(saved.courseId, saved.id, item.id, saved.revision),
    AssignmentConflictError,
  );
});

test("focused assignment commands reject extra request fields before transport", async () => {
  const client = createMockApiClient({ assignmentAuthoring: true });
  const assignment = await client.getAssignmentEditor(publishedProblemFixture.assignment.id);
  const item = assignment.items[0];
  assert.ok(item);
  await assert.rejects(
    client.addAssignmentItem(
      assignment.courseId,
      assignment.id,
      { questionId: item.questionId, position: 1, extra: true },
      assignment.revision,
    ),
    /allowed by this response contract/u,
  );
  await assert.rejects(
    client.replaceAssignmentItemQuestion(
      assignment.courseId,
      assignment.id,
      item.id,
      { questionId: item.questionId, extra: true },
      assignment.revision,
    ),
    /allowed by this response contract/u,
  );
});
