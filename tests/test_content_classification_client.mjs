// Stable browser contract for active authoring choices and retired discovery IDs.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeContentClassificationList } from "../src/api/decoders/content_classification.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { ApiProtocolError } from "../src/api/http_client/error.ts";

const active = {
  uuid: "00000000-0000-0000-0000-000000000001",
  name: "Biology",
  isRetired: false,
};
const retired = {
  uuid: "00000000-0000-0000-0000-000000000002",
  name: "Historical Biology",
  isRetired: true,
};

function noStoreJson(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("classification decoding makes a retired Discipline explicit", () => {
  assert.deepEqual(
    decodeContentClassificationList({ disciplines: [active, retired] }, "disciplines"),
    [active, retired],
  );
  assert.throws(
    () =>
      decodeContentClassificationList(
        { disciplines: [{ ...active, isRetired: undefined }] },
        "disciplines",
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeContentClassificationList(
        { disciplines: [{ ...active, lifecycle: "active" }] },
        "disciplines",
      ),
    DecodeError,
  );
});

test("Discipline discovery and Sysadmin lifecycle use their closed same-origin paths", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      const path = new URL(request.url).pathname;
      if (path.endsWith("/discovery")) return noStoreJson({ disciplines: [active, retired] });
      if (path.endsWith("/disciplines") && request.method === "POST") {
        return noStoreJson(active, 201);
      }
      if (path.endsWith("/rename")) return noStoreJson({ ...active, name: "Life Sciences" });
      if (path.endsWith("/retire")) return noStoreJson(retired);
      if (path.endsWith("/restore")) return noStoreJson(active);
      return noStoreJson({ disciplines: [active] });
    },
  });

  assert.deepEqual(await client.listDisciplinesIncludingRetired(), [active, retired]);
  await client.createDiscipline(" Biology ");
  await client.renameDiscipline(active.uuid, "Life Sciences");
  await client.retireDiscipline(active.uuid);
  await client.restoreDiscipline(active.uuid);

  assert.deepEqual(
    requests.map((request) => [request.method, new URL(request.url).pathname]),
    [
      ["GET", "/api/content-classification/disciplines/discovery"],
      ["POST", "/api/content-classification/disciplines"],
      ["POST", `/api/content-classification/disciplines/${active.uuid}/rename`],
      ["POST", `/api/content-classification/disciplines/${active.uuid}/retire`],
      ["POST", `/api/content-classification/disciplines/${active.uuid}/restore`],
    ],
  );
  assert.deepEqual(await requests[1].json(), { name: "Biology" });
  assert.deepEqual(await requests[2].json(), { name: "Life Sciences" });
});

const disciplineUuid = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const subjectUuid = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const topicUuid = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
const subtopicUuid = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";

test("createSubject posts a new Subject and accepts an existing Subject on the shipped client", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      const path = new URL(request.url).pathname;
      const body = request.method === "POST" ? await request.json() : null;
      if (path === "/api/content-classification/subjects") {
        const needsAcceptance = body.name === "Existing Biochemistry";
        return noStoreJson(
          { uuid: subjectUuid, name: body.name, needsAcceptance },
          needsAcceptance ? 200 : 201,
        );
      }
      if (path === `/api/content-classification/subjects/${subjectUuid}/disciplines`) {
        return noStoreJson(
          { uuid: subjectUuid, name: "Existing Biochemistry", isRetired: false },
          200,
        );
      }
      if (path === "/api/content-classification/topics") {
        return noStoreJson({ uuid: topicUuid, name: body.name, isRetired: false }, 201);
      }
      if (path === "/api/content-classification/subtopics") {
        return noStoreJson({ uuid: subtopicUuid, name: body.name, isRetired: false }, 201);
      }
      throw new Error(`unexpected classification request ${request.method} ${path}`);
    },
  });

  assert.deepEqual(await client.createSubject(" Peptides ", disciplineUuid), {
    uuid: subjectUuid,
    name: "Peptides",
    needsAcceptance: false,
  });
  assert.deepEqual(await client.createSubject("Existing Biochemistry", disciplineUuid), {
    uuid: subjectUuid,
    name: "Existing Biochemistry",
    needsAcceptance: true,
  });
  assert.deepEqual(await client.acceptSubjectDiscipline(subjectUuid, disciplineUuid), {
    uuid: subjectUuid,
    name: "Existing Biochemistry",
    isRetired: false,
  });
  assert.deepEqual(await client.createTopic(" Bonds ", subjectUuid), {
    uuid: topicUuid,
    name: "Bonds",
    isRetired: false,
  });
  assert.deepEqual(await client.createSubtopic(" Amide ", topicUuid), {
    uuid: subtopicUuid,
    name: "Amide",
    isRetired: false,
  });
  await assert.rejects(() => client.createSubject(" ", disciplineUuid), DecodeError);
  await assert.rejects(() => client.createSubject("x".repeat(121), disciplineUuid), DecodeError);
  assert.deepEqual(await Promise.all(requests.map((request) => request.json())), [
    { name: "Peptides", disciplineUuid },
    { name: "Existing Biochemistry", disciplineUuid },
    { disciplineUuid },
    { name: "Bonds", subjectUuid },
    { name: "Amide", topicUuid },
  ]);
  assert.deepEqual(
    requests.map((request) => [request.method, new URL(request.url).pathname]),
    [
      ["POST", "/api/content-classification/subjects"],
      ["POST", "/api/content-classification/subjects"],
      ["POST", `/api/content-classification/subjects/${subjectUuid}/disciplines`],
      ["POST", "/api/content-classification/topics"],
      ["POST", "/api/content-classification/subtopics"],
    ],
  );
});

test("createSubject rejects a created Subject reported with the acceptance status", async () => {
  const client = createHttpApiClient({
    fetch: async () =>
      noStoreJson({ uuid: subjectUuid, name: "Peptides", needsAcceptance: false }, 200),
  });
  await assert.rejects(() => client.createSubject("Peptides", disciplineUuid), ApiProtocolError);
});

test("fulfilling a Discipline request is one same-origin request and dismissal stays separate", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request);
      if (new URL(request.url).pathname.endsWith("/fulfill")) {
        return noStoreJson(active, 201);
      }
      return new Response(null, { status: 204, headers: { "cache-control": "no-store" } });
    },
  });

  assert.deepEqual(await client.fulfillDisciplineRequest(disciplineUuid), active);
  await client.resolveDisciplineRequest(disciplineUuid);
  assert.deepEqual(
    requests.map((request) => [request.method, new URL(request.url).pathname]),
    [
      ["POST", `/api/content-classification/discipline-requests/${disciplineUuid}/fulfill`],
      ["POST", `/api/content-classification/discipline-requests/${disciplineUuid}/resolve`],
    ],
  );
});
