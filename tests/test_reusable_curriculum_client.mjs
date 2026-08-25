import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "./fixtures/published_problem.ts";
import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeBlueprintView,
  decodeAlphaCourseView,
} from "../src/api/decoders/reusable_curriculum.ts";
import {
  ApiProtocolError,
  ReusableCurriculumConflictError,
  createHttpApiClient,
} from "../src/api/http_client.ts";

function definitionInput() {
  return {
    definition: {
      title: "Peptide fundamentals",
      instructions: "Use your course notes to explain each choice.",
      entries: [
        {
          kind: "fixed",
          questionId: publishedProblemFixture.catalogProblem.questionId,
          pointsPossible: "2",
          scoringMode: "normal",
        },
      ],
      defaults: {
        timeLimitSeconds: null,
        attemptLimit: 2,
        lateSubmission: "accept",
        deadlineBehavior: "autoSubmit",
        runPolicies: {
          completion: { kind: "answerAll" },
          grade: "highest",
          continuedPractice: { kind: "unlimited" },
          variation: "newSeeds",
        },
        learnerDisclosure: {
          score: "afterSubmit",
          perItemCorrectness: "afterSubmit",
          feedbackText: "afterSubmit",
          solution: "never",
          classStatistics: "never",
        },
      },
      schedule: { availableAt: null, dueAt: null, closesAt: null },
    },
  };
}

function definitionView() {
  const input = definitionInput().definition;
  return {
    ...input,
    entries: [
      {
        kind: "fixed",
        question: {
          catalog: {
            summary: publishedProblemFixture.catalogProblem,
            evidence: { state: "insufficientEvidence" },
          },
          selectionAvailability: "available",
        },
        points_possible: "2",
        scoring_mode: "normal",
      },
    ],
  };
}

function blueprint(revision = "7") {
  return {
    reference: "BP-7",
    revision,
    access: "owner",
    definition: definitionView(),
  };
}

function alpha(revision = "3") {
  return {
    reference: "AC-3",
    title: "Biochemistry sequence",
    revision,
    creatorByline: { names: ["Fixture Instructor"] },
    access: "creator",
    modules: [{ label: "Week one", definitions: [definitionView()] }],
  };
}

function noStoreJson(value, etag, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: {
      "cache-control": "no-store",
      "content-type": "application/json",
      ...(etag === undefined ? {} : { etag }),
    },
  });
}

test("B1 curriculum decoders keep views answer-free and reject hostile fields", () => {
  assert.equal(decodeBlueprintView(blueprint()).reference, "BP-7");
  assert.equal(decodeAlphaCourseView(alpha()).reference, "AC-3");
  const hostile = structuredClone(blueprint());
  hostile.definition.entries[0].question.answerKey = "secret";
  assert.throws(() => decodeBlueprintView(hostile), DecodeError);
  const badByline = structuredClone(alpha());
  badByline.creatorByline.account = "private account";
  assert.throws(() => decodeAlphaCourseView(badByline), DecodeError);
});

test("B1 client uses closed local commands, matching ETags, and same-origin no-store transport", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      const path = new URL(request.url).pathname;
      if (request.method === "GET" && path.endsWith("BP-7")) return noStoreJson(blueprint(), '"7"');
      if (request.method === "POST" && path.endsWith("course-blueprints"))
        return noStoreJson(blueprint(), '"7"', 201);
      if (request.method === "PUT" && path.endsWith("BP-7"))
        return noStoreJson(blueprint("8"), '"8"');
      if (request.method === "GET" && path.endsWith("AC-3")) return noStoreJson(alpha(), '"3"');
      if (request.method === "POST" && path.endsWith("alpha-courses"))
        return noStoreJson(alpha(), '"3"', 201);
      if (request.method === "PUT" && path.endsWith("AC-3")) return noStoreJson(alpha("4"), '"4"');
      if (request.method === "DELETE")
        return new Response(null, { status: 204, headers: { "cache-control": "no-store" } });
      return noStoreJson({ items: [], nextCursor: null });
    },
  });

  const currentBlueprint = await client.getBlueprint("BP-7");
  assert.equal(currentBlueprint.etag, '"7"');
  await client.createBlueprint(definitionInput());
  const revisedBlueprint = await client.replaceBlueprint(
    "BP-7",
    definitionInput(),
    currentBlueprint.etag,
  );
  assert.equal(revisedBlueprint.etag, '"8"');
  const currentAlpha = await client.getAlphaCourse("AC-3");
  await client.createAlphaCourse({
    title: "Biochemistry sequence",
    modules: [{ label: "Week one", definitions: [definitionInput().definition] }],
  });
  await client.replaceAlphaCourse(
    "AC-3",
    {
      title: "Biochemistry sequence",
      modules: [{ label: "Week one", definitions: [definitionInput().definition] }],
    },
    currentAlpha.etag,
  );
  await client.deleteBlueprint("BP-7", revisedBlueprint.etag);
  const update = requests.find(
    (request) => request.method === "PUT" && request.url.endsWith("BP-7"),
  );
  assert.equal(update.headers.get("if-match"), '"7"');
  assert.equal(update.cache, "no-store");
  assert.equal(update.credentials, "same-origin");
  await assert.rejects(
    client.replaceBlueprint("BP-7", definitionInput(), '"07"'),
    ApiProtocolError,
  );
  await assert.rejects(
    client.createBlueprint({ ...definitionInput(), creatorByline: { names: ["forged"] } }),
    DecodeError,
  );
});

test("B1 client gives a typed conflict for a 412 and validates local positions and references", async () => {
  const conflict = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        new Response(null, { status: 412, headers: { "cache-control": "no-store" } }),
      ),
  });
  await assert.rejects(
    conflict.replaceAlphaCourse(
      "AC-3",
      {
        title: "Biochemistry sequence",
        modules: [{ label: "Week one", definitions: [definitionInput().definition] }],
      },
      '"3"',
    ),
    ReusableCurriculumConflictError,
  );
  await assert.rejects(conflict.getBlueprint("BP-00"), DecodeError);
});
