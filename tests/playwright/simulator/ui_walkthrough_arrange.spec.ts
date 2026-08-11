// ui_walkthrough_arrange.spec.ts - direct private-arranger boundary tests.

import { expect, test } from "@playwright/test";

import {
  arrangementOutputFor,
  authenticatedInstructorContextWithRequest,
  instructorSetupArrangementOutput,
  instructorCredential,
  launcherManifest,
} from "../../walkthrough/children/arrange";

const INSTRUCTOR =
  "instructor=fixture_instructor_credential_that_is_long_enough_for_local_provider";
const SENTINEL = "private-credential-must-not-appear";

test("private parser accepts only the instructor and the launcher baseline reference", () => {
  expect(instructorCredential(`${INSTRUCTOR}\nstudent=${SENTINEL}`)).toBe(
    "fixture_instructor_credential_that_is_long_enough_for_local_provider",
  );
  const manifest =
    '{"assignmentId":"123e4567-e89b-12d3-a456-426614174000","enrollmentId":"123e4567-e89b-12d3-a456-426614174001","problemId":"123e4567-e89b-12d3-a456-426614174002","versionId":"123e4567-e89b-12d3-a456-426614174003"}';
  expect(launcherManifest(manifest)).toEqual({
    assignmentId: "123e4567-e89b-12d3-a456-426614174000",
  });
  expect(() => launcherManifest('{"assignmentId":"123e4567-e89b-12d3-a456-426614174000"}')).toThrow(
    "arrangement-input",
  );
  expect(() => instructorCredential(`instructor=${SENTINEL}!`)).toThrow("arrangement-input");
});

test("instructor setup retains only the supported retry-corpus publication arrangement", () => {
  const output = instructorSetupArrangementOutput({
    problem: "123e4567-e89b-12d3-a456-426614174001",
    version: "123e4567-e89b-12d3-a456-426614174002",
    catalogSearchTitle: "Pilot retry corpus pilotref123e4567e89b12d3a456426614174000",
  });
  expect(output).toEqual({
    arrangements: [
      {
        label: "api-retry-corpus-publication",
        problemId: "123e4567-e89b-12d3-a456-426614174001",
        versionId: "123e4567-e89b-12d3-a456-426614174002",
        catalogSearchTitle: "Pilot retry corpus pilotref123e4567e89b12d3a456426614174000",
      },
    ],
  });
  expect(JSON.stringify(output)).not.toContain("courseId");
});

test("arrangement output separates every setup boundary into fixed public records", () => {
  const output = arrangementOutputFor(
    "123e4567-e89b-12d3-a456-426614174000",
    {
      problem: "123e4567-e89b-12d3-a456-426614174001",
      version: "123e4567-e89b-12d3-a456-426614174002",
    },
    {
      arrangement: "seeded-course-assignments",
      baselineAssignmentId: "123e4567-e89b-12d3-a456-426614174000",
      courseId: "123e4567-e89b-12d3-a456-426614174003",
      masteryAssignmentId: "123e4567-e89b-12d3-a456-426614174004",
      examAssignmentId: "123e4567-e89b-12d3-a456-426614174005",
    },
  );
  expect(output.arrangements.map(({ label }) => label)).toEqual([
    "launcher-seeded-enrollment",
    "launcher-baseline-assignment",
    "api-retry-corpus-publication",
    "api-mastery-assignment",
    "api-exam-assignment",
  ]);
  expect(JSON.stringify(output)).not.toContain(SENTINEL);
});

test("failed instructor login disposes its isolated context without echoing private transport data", async () => {
  let disposed = false;
  const context = {
    post(): Promise<{ status(): number }> {
      return Promise.reject(new Error(SENTINEL));
    },
    dispose(): Promise<void> {
      disposed = true;
      return Promise.resolve();
    },
  };
  const factory = {
    newContext(): Promise<typeof context> {
      return Promise.resolve(context);
    },
  };

  let thrown: unknown;
  try {
    await authenticatedInstructorContextWithRequest(
      factory,
      "http://127.0.0.1:3000",
      "fixture_instructor_credential_that_is_long_enough_for_local_provider",
    );
  } catch (error: unknown) {
    thrown = error;
  }
  expect(disposed).toBe(true);
  expect(String(thrown)).toBe("Error: assignment-login");
  expect(String(thrown)).not.toContain(SENTINEL);
});
