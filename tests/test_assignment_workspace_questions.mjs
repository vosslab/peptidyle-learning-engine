// Stable Questions-workspace model contracts; connected browser journeys cover visible controls.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { createMasteryAssignmentEditorState } from "./support/assignment_editor_test_support.ts";
import { assignmentContentInput } from "../src/pages/assignment_editor_model.ts";
import {
  assignmentWorkspaceCreateErrorMessage,
  createdAssignmentQuestionsPath,
} from "../src/pages/assignment_workspace/assignment_workspace_create_model.ts";
import {
  assignmentWorkspaceCreatePath,
  assignmentWorkspacePath,
} from "../src/pages/assignment_workspace/assignment_workspace_paths.ts";
import {
  parseAssignmentReference,
  parseCourseInstanceReference,
} from "../src/navigation/public_route.ts";

test("legacy assignment content model keeps title with ordered public Assignment Content", () => {
  const draft = {
    ...createMasteryAssignmentEditorState("course-1"),
    title: "Protein bonds",
    entries: [
      {
        kind: "fixedQuestion",
        id: "item-1",
        questionId: "7K3-M9QP",
        title: "Peptide bond resonance",
        backend: "ple",
        capabilities: [],
        pointsPossible: "1",
        availability: "available",
        scoringRule: "normal",
        questionAttemptLimit: { maxAttempts: null },
        questionAttemptTimeLimit: { kind: "unlimited" },
      },
    ],
  };

  assert.deepEqual(assignmentContentInput(draft), {
    title: "Protein bonds",
    entries: [
      {
        kind: "fixedQuestion",
        questionId: "7K3-M9QP",
        pointsPossible: "1",
        availability: "available",
        scoringRule: "normal",
        questionAttemptLimit: { maxAttempts: null },
        questionAttemptTimeLimit: { kind: "unlimited" },
      },
    ],
  });
});

test("the rendered Questions surface owns Assignment title and Policies keeps delivery fields", () => {
  const questionsPage = readFileSync(
    "src/pages/assignment_workspace/assignment_workspace_questions_page.tsx",
    "utf8",
  );
  const policiesPage = readFileSync(
    "src/pages/assignment_workspace/assignment_workspace_policies_page.tsx",
    "utf8",
  );

  assert.match(
    questionsPage,
    /const \[title, setTitle\] = createSignal\(workspace\.assignment\(\)\.workspace\.title\);/u,
  );
  assert.match(
    questionsPage,
    /Assignment title\s*<input value=\{title\(\)\} onInput=\{\(event\) => setTitle\(event\.currentTarget\.value\)\}/u,
  );
  assert.match(
    questionsPage,
    /withQuestionIds\(workspace\.assignment\(\)\.workspace, title\(\), selected\(\)\)/u,
  );
  assert.doesNotMatch(policiesPage, /Assignment title/u);
  assert.match(policiesPage, /Student instructions/u);
});

test("persisted draft creation enters the canonical Questions route", () => {
  const course = parseCourseInstanceReference("C-8");
  const assignment = parseAssignmentReference("A-15");
  assert.ok(course);
  assert.ok(assignment);
  assert.equal(
    createdAssignmentQuestionsPath(course, assignment),
    "/instructor/courses/C-8/assignments/A-15/questions",
  );
  assert.equal(assignmentWorkspaceCreatePath(course), "/instructor/courses/C-8/assignments/new");
});

test("draft creation recovery gives one safe actionable message", () => {
  const message = assignmentWorkspaceCreateErrorMessage();
  assert.equal(
    message,
    "The Assignment could not be created. Your title is still here. Try again.",
  );
  assert.equal(message.includes("/api/"), false);
});

test("Questions composition provides a named removal control", () => {
  const page = readFileSync(
    "src/pages/assignment_workspace/assignment_workspace_questions_page.tsx",
    "utf8",
  );

  assert.match(
    page,
    /function remove\(index: number\): void \{\s+setSelected\(\(current\) => removeSelectedQuestionAt\(current, index\)\);/u,
  );
  assert.match(
    page,
    /aria-label=\{`Remove Question \$\{entry\.questionId\}`\}[\s\S]*?onClick=\{\(\) => remove\(index\(\)\)\}/u,
  );
});

test("initial and reloaded Questions always link to the contract Policies route", () => {
  const page = readFileSync(
    "src/pages/assignment_workspace/assignment_workspace_questions_page.tsx",
    "utf8",
  );
  const course = parseCourseInstanceReference("C-8");
  const assignment = parseAssignmentReference("A-15");

  assert.ok(course);
  assert.ok(assignment);
  assert.equal(
    assignmentWorkspacePath(course, assignment, "policies"),
    "/instructor/courses/C-8/assignments/A-15/policies",
  );
  assert.match(
    page,
    /<p class="assignment-workspace-next-actions">\s*<A\s+class="quiet-link"\s+href=\{assignmentWorkspacePath\(\s*workspace\.courseReference,\s*workspace\.assignmentReference,\s*"policies",/u,
  );
  assert.doesNotMatch(
    page,
    /<Show when=\{saved\(\)\}>\s*<p class="assignment-workspace-next-actions">/u,
  );
  assert.match(
    page,
    /setMessage\("Questions and order saved\. Review assignment policies when you are ready\."\);/u,
  );
});
