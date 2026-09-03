import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeHypotheticalStudentViewScenarioRequest,
  decodeInstructorPreviewSchedulePage,
  decodePreviewPlaneResponse,
  decodeSelectedStudentViewScenarioRequest,
} from "../src/api/decoders.ts";
import {
  ApiProtocolError,
  PreviewPlaneConflictError,
  createHttpApiClient,
} from "../src/api/http_client.ts";

const course = "C-12";
const assignment = "A-34";
const revision = "7";
const selectedMoment = { value: "2026-08-25T09:00:00.000", time_zone: "America/Chicago" };
const inheritedAdjustment = {
  available_at: { kind: "inherit" },
  due_at: { kind: "inherit" },
  closes_at: { kind: "inherit" },
  assignment_attempt_time_limit_seconds: { kind: "inherit" },
  attempt_limit: { kind: "inherit" },
};

function jsonResponse(value, status = 200, cacheControl = "no-store") {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json", "cache-control": cacheControl },
  });
}

function effective_assignment_policy(source = "base") {
  return {
    available_at: { value: "2026-08-24T09:00:00.000", source },
    due_at: { value: "2026-08-26T09:00:00.000", source },
    closes_at: { value: "2026-08-27T09:00:00.000", source },
    assignment_attempt_time_limit_seconds: { value: 3600, source },
    attempt_limit: { value: 2, source },
    late_work_rule: { value: "accept", source },
    assignment_deadline_rule: { value: "auto_submit", source },
  };
}

function allowedEvaluation() {
  return {
    kind: "allowed",
    student_view_scenario: {
      origin: "selected_student",
      assignment,
      revision,
      selected_moment: selectedMoment,
      policy: effective_assignment_policy("accommodation"),
      prior_assignment_attempt_count: 0,
    },
    student_view_scenario_admission: "selected_student_active_student_course_membership",
    effective_assignment_policy: effective_assignment_policy("accommodation"),
    student_feedback_release: [
      {
        kind: "available",
        moment: "now",
        flags: {
          score_shown: false,
          correctness_shown: false,
          feedback_shown: false,
          question_answer_shown: false,
          question_answer_explanation_shown: false,
          statistics_shown: false,
        },
      },
      {
        kind: "available",
        moment: "due",
        flags: {
          score_shown: true,
          correctness_shown: true,
          feedback_shown: true,
          question_answer_shown: false,
          question_answer_explanation_shown: false,
          statistics_shown: false,
        },
      },
      { kind: "unavailable", moment: "close", reason: "boundary_missing" },
    ],
  };
}

function previewResponse() {
  return {
    evaluation: allowedEvaluation(),
    accommodation: {
      before: effective_assignment_policy("base"),
      after: effective_assignment_policy("accommodation"),
    },
  };
}

test("preview decoders accept the closed server projections", () => {
  const response = previewResponse();
  const decoded = decodePreviewPlaneResponse(response);
  assert.deepEqual(decoded, response);

  const schedulePage = {
    revision,
    rows: [
      {
        kind: "granted",
        membership: "M-9",
        display: "Mary Student",
        active_student_course_membership: "active_student_course_membership",
        effective_assignment_policy: effective_assignment_policy(),
      },
      {
        kind: "denied",
        membership: "M-10",
        display: "Jack Student",
        reason: "no_active_student_course_membership",
      },
    ],
    next_cursor: "next cursor",
  };
  assert.deepEqual(decodeInstructorPreviewSchedulePage(schedulePage), schedulePage);

  const hypotheticalRequest = {
    assignment,
    revision,
    selected_moment: selectedMoment,
    modifiers: { mode: "extend_only", adjustment: inheritedAdjustment },
  };
  assert.deepEqual(
    decodeHypotheticalStudentViewScenarioRequest(hypotheticalRequest),
    hypotheticalRequest,
  );
  assert.deepEqual(
    decodeSelectedStudentViewScenarioRequest({
      assignment,
      revision,
      selected_moment: selectedMoment,
      selected_student_membership: "M-9",
    }),
    { assignment, revision, selected_moment: selectedMoment, selected_student_membership: "M-9" },
  );
});

test("preview decoders reject unknown and protected fields at closed boundaries", () => {
  const denial = {
    evaluation: { kind: "denied", reason: "active_student_course_membership_required" },
    accommodation: null,
  };
  assert.deepEqual(decodePreviewPlaneResponse(denial), denial);
  assert.throws(
    () =>
      decodePreviewPlaneResponse({
        evaluation: {
          kind: "denied",
          reason: "active_student_course_membership_required",
          student_view_scenario: allowedEvaluation().student_view_scenario,
        },
        accommodation: null,
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodePreviewPlaneResponse({
        ...previewResponse(),
        evaluation: {
          ...allowedEvaluation(),
          effective_assignment_policy: {
            ...effective_assignment_policy(),
            assignmentDeadlineRule: { value: "auto_submit", source: "base" },
          },
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodePreviewPlaneResponse({
        ...previewResponse(),
        evaluation: {
          ...allowedEvaluation(),
          student_view_scenario_admission: "hypothetical_student_view_scenario_admission",
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodePreviewPlaneResponse({
        evaluation: { kind: "denied", reason: "active_student_course_membership_required" },
        accommodation: {
          before: effective_assignment_policy(),
          after: effective_assignment_policy(),
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodePreviewPlaneResponse({
        ...previewResponse(),
        evaluation: {
          ...allowedEvaluation(),
          student_view_scenario: {
            ...allowedEvaluation().student_view_scenario,
            membership: "M-9",
          },
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeInstructorPreviewSchedulePage({
        revision,
        rows: [
          {
            kind: "denied",
            membership: "M-9",
            display: "Mary",
            reason: "no_active_student_course_membership",
            effective_assignment_policy: effective_assignment_policy(),
          },
        ],
        next_cursor: null,
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodePreviewPlaneResponse({
        ...previewResponse(),
        evaluation: {
          ...allowedEvaluation(),
          student_feedback_release: [
            {
              ...allowedEvaluation().student_feedback_release[0],
              flags: { ...allowedEvaluation().student_feedback_release[0].flags, answer: "secret" },
            },
            ...allowedEvaluation().student_feedback_release.slice(1),
          ],
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeHypotheticalStudentViewScenarioRequest({
        assignment,
        revision,
        selected_moment: selectedMoment,
        unexpectedGroups: ["G-3"],
        modifiers: { mode: "extend_only", adjustment: inheritedAdjustment },
      }),
    DecodeError,
  );
});

test("preview transport uses the public C-/A- routes, headers, bodies, and canonical pagination", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      requests.push({ path: String(input), init });
      return String(input).includes("preview-schedule")
        ? jsonResponse({ revision, rows: [], next_cursor: null })
        : jsonResponse(previewResponse());
    },
  });

  await client.listPreviewSchedule(course, assignment, revision, "opaque +/", 25);
  await client.constructHypotheticalStudentViewScenario(course, assignment, revision, {
    selected_moment: selectedMoment,
    modifiers: { mode: "extend_only", adjustment: inheritedAdjustment },
  });
  await client.constructSelectedStudentViewScenario(course, assignment, revision, {
    selected_moment: selectedMoment,
    selected_student_membership: "M-9",
  });

  assert.equal(
    requests[0].path,
    "/api/courses/C-12/assignments/A-34/preview-schedule?after=opaque+%2B%2F&size=25",
  );
  for (const request of requests) {
    assert.equal(request.init.credentials, "same-origin");
    assert.equal(request.init.cache, "no-store");
    assert.equal(request.init.headers["if-match"], '"7"');
    assert.equal(request.init.headers.accept, "application/json");
  }
  assert.equal(requests[0].init.method, "GET");
  assert.equal(requests[1].init.method, "POST");
  assert.equal(requests[1].init.headers["content-type"], "application/json");
  assert.deepEqual(JSON.parse(requests[1].init.body), {
    selected_moment: selectedMoment,
    modifiers: { mode: "extend_only", adjustment: inheritedAdjustment },
  });
  assert.deepEqual(JSON.parse(requests[2].init.body), {
    selected_moment: selectedMoment,
    selected_student_membership: "M-9",
  });
});

test("preview transport maps stale revisions and cache violations safely", async () => {
  const staleClient = createHttpApiClient({
    fetch: async () => jsonResponse({ error: "assignment changed; reload it" }, 412),
  });
  await assert.rejects(
    staleClient.constructSelectedStudentViewScenario(course, assignment, revision, {
      selected_moment: selectedMoment,
      selected_student_membership: "M-9",
    }),
    PreviewPlaneConflictError,
  );

  const cacheableClient = createHttpApiClient({
    fetch: async () =>
      jsonResponse({ revision, rows: [], next_cursor: null }, 200, "private, max-age=1"),
  });
  await assert.rejects(
    cacheableClient.listPreviewSchedule(course, assignment, revision),
    ApiProtocolError,
  );
});
