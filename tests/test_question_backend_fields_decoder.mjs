import assert from "node:assert/strict";
import test from "node:test";

import { publishedQuestionFixture } from "./fixtures/published_question.ts";
import {
  decodeDraftQuestionContent,
  decodeQuestionRevision,
} from "../src/api/decoders/question_delivery.ts";

const published = publishedQuestionFixture.publishedQuestionRevision;
const draft = publishedQuestionFixture.draft;

function publishedFields(questionBackend, fields = {}) {
  return {
    ...published,
    questionBackend,
    webworkPgPath: null,
    qtiPackageItemIdentifier: null,
    imathasQuestionBackendBinding: null,
    ...fields,
  };
}

function draftFields(questionBackend, fields = {}) {
  return {
    ...draft,
    questionBackend,
    webworkPgPath: null,
    qtiPackageItemIdentifier: null,
    workspaceImportId: null,
    draftImathasQuestionBackendBinding: null,
    ...fields,
  };
}

test("published Question Revision decoding accepts only its four exact Question Backend rows", () => {
  const rows = [
    publishedFields("ple"),
    publishedFields("webwork", { webworkPgPath: "Library/Algebra/item.pg" }),
    publishedFields("qti", { qtiPackageItemIdentifier: "choice-17" }),
    publishedFields("imathas", {
      imathasQuestionBackendBinding: {
        deploymentReference: "recorded-imathas",
        itemReference: "item-17",
        profile: "recorded-v1",
      },
    }),
  ];
  for (const row of rows) {
    assert.equal(decodeQuestionRevision(row).questionBackend, row.questionBackend);
  }
  assert.throws(() =>
    decodeQuestionRevision(publishedFields("webwork", { qtiPackageItemIdentifier: "choice-17" })),
  );
  assert.throws(() =>
    decodeQuestionRevision(
      publishedFields("qti", {
        qtiPackageItemIdentifier: "choice-17",
        workspaceImportId: "0198e000-0000-7000-8000-000000000017",
      }),
    ),
  );
});

test("Draft Question Content decoding accepts only its four exact Question Backend rows", () => {
  const rows = [
    draftFields("ple"),
    draftFields("webwork", { webworkPgPath: "Library/Algebra/item.pg" }),
    draftFields("qti", {
      qtiPackageItemIdentifier: "choice-17",
      workspaceImportId: "0198e000-0000-7000-8000-000000000017",
    }),
    draftFields("imathas", {
      draftImathasQuestionBackendBinding: {
        deploymentReference: "recorded-imathas",
        itemReference: "item-17",
      },
    }),
  ];
  for (const row of rows) {
    assert.equal(decodeDraftQuestionContent(row).questionBackend, row.questionBackend);
  }
  assert.throws(() =>
    decodeDraftQuestionContent(
      draftFields("imathas", {
        draftImathasQuestionBackendBinding: {
          deploymentReference: "recorded-imathas",
          itemReference: "item-17",
          profile: "published-only",
        },
      }),
    ),
  );
});

test("Question Backend decoders reject retired backendLocator and source-bearing fields", () => {
  assert.throws(() =>
    decodeQuestionRevision({
      ...published,
      backendLocator: { backend: "ple" },
    }),
  );
  assert.throws(() =>
    decodeDraftQuestionContent({
      ...draft,
      backendLocator: { backend: "ple" },
    }),
  );
  assert.throws(() =>
    decodeQuestionRevision(
      publishedFields("imathas", {
        imathasQuestionBackendBinding: {
          deploymentReference: "recorded-imathas",
          itemReference: "item-17",
          profile: "recorded-v1",
          sourceObjectReference: "00000000-0000-0000-0000-000000000017",
        },
      }),
    ),
  );
});
