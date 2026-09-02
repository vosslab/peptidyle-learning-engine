// test_workspace_client.mjs - behavior contracts for private workspace CRUD transport.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedQuestionFixture } from "./fixtures/published_question.ts";
import {
  createHttpApiClient,
  ApiProtocolError,
  ApiRequestError,
  WorkspaceConflictError,
  PublicationValidationError,
} from "../src/api/http_client.ts";
import { decodeQuestionPublicationReview, decodeDraftQuestionPage } from "../src/api/decoders.ts";
import {
  decodeQuestionAuthorship,
  parseReviewedQuestionAuthorship,
} from "../src/api/question_authorship.ts";
import { createWorkspaceEditorRepository } from "../src/pages/editor_workspace_repository.ts";

const draft = publishedQuestionFixture.draft;
const workspace = draft.workspace;
const draftQuestion = "D-901";

function jsonResponse(value, options = {}) {
  return new Response(JSON.stringify(value), {
    status: options.status ?? 200,
    headers: { "content-type": "application/json", ...options.headers },
  });
}

function questionPublicationReviewCurrent(content) {
  const response = content.response;
  const optionCount =
    response.kind === "multipleChoice"
      ? response.choices.length
      : response.kind === "ordering"
        ? response.items.length
        : null;
  return {
    questionBackend: content.questionBackend,
    title: content.metadata.title,
    prompt: { blocks: content.prompt.map((block) => block.kind) },
    response: { kind: response.kind, optionCount },
    questionAttemptLimit: content.questionAttemptLimit,
    questionAttemptTimeLimit: content.questionAttemptTimeLimit,
    questionVariationRule: { kind: content.questionVariationRule.kind },
    metadata: {
      questionDescription: content.metadata.questionDescription,
      tags: content.metadata.tags,
      classifications: content.metadata.classifications,
      questionLicense: content.metadata.questionLicense,
      questionCitation: content.metadata.questionCitation,
      language: content.metadata.language,
    },
  };
}

test("Question Publication Review admits only safe review summaries and a consistent base", () => {
  const current = questionPublicationReviewCurrent(draft);
  const first = {
    draftQuestionRevisionNumber: 1,
    baseQuestion: "newQuestion",
    current,
    changed: [],
  };
  assert.deepEqual(decodeQuestionPublicationReview(first), { ...first, revision: '"1"' });
  for (const contaminated of [
    { ...first, current: { ...current, source: { path: "private.pg" } } },
    { ...first, current: { ...current, prompt: { blocks: ["text", "secret"] } } },
    { ...first, current: { ...current, response: { kind: "numeric", optionCount: 1 } } },
    { ...first, changed: ["title"] },
    {
      ...first,
      prior: "hidden",
    },
  ]) {
    assert.throws(() => decodeQuestionPublicationReview(contaminated));
  }
});

test("publication transport uses a bodyless validation request and explicit Question Authorship", async () => {
  const calls = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      calls.push({ input: String(input), init });
      if (String(input).endsWith("publication-validation")) {
        return jsonResponse({ violations: [] }, { headers: { etag: '"1"' } });
      }
      if (String(input).endsWith("question-publication-review")) {
        return jsonResponse(
          {
            draftQuestionRevisionNumber: 1,
            baseQuestion: "newQuestion",
            current: questionPublicationReviewCurrent(draft),
            changed: [],
          },
          { headers: { etag: '"1"' } },
        );
      }
      return jsonResponse(publishedQuestionFixture.publishedQuestion);
    },
  });
  await client.validateWorkspacePublication(workspace);
  await client.getQuestionPublicationReview(workspace);
  const request = {
    scope: "public",
    authorship: { authors: [{ displayName: "Fixture Instructor" }] },
  };
  await client.publishWorkspace(workspace, request, '"1"');
  assert.equal(calls[0].init.method, "POST");
  assert.equal(calls[0].init.body, undefined);
  assert.equal(calls[0].init.headers["content-type"], undefined);
  assert.equal(calls[2].init.body, JSON.stringify(request));
  assert.equal(calls[2].init.headers["content-type"], "application/json");
});

test("Question Authorship shares strict line-based author input and wire decoding", async () => {
  assert.deepEqual(parseReviewedQuestionAuthorship("  Ada Lovelace  \nGrace Hopper"), {
    authors: [{ displayName: "Ada Lovelace" }, { displayName: "Grace Hopper" }],
  });
  for (const text of ["", "Ada\nAda", "Ada\u0007", "😀".repeat(121)]) {
    assert.equal(parseReviewedQuestionAuthorship(text), null);
  }
  const sixteenAuthors = Array.from({ length: 16 }, (_, index) => `Question Author ${index + 1}`);
  assert.equal(parseReviewedQuestionAuthorship(sixteenAuthors.join("\n"))?.authors.length, 16);
  assert.equal(
    parseReviewedQuestionAuthorship([...sixteenAuthors, "Question Author 17"].join("\n")),
    null,
  );
  assert.deepEqual(
    decodeQuestionAuthorship({ authors: [{ displayName: "Ada Lovelace" }] }, "response.authorship"),
    { authors: [{ displayName: "Ada Lovelace" }] },
  );
  assert.equal(
    decodeQuestionAuthorship(
      { authors: sixteenAuthors.map((displayName) => ({ displayName })) },
      "response.authorship",
    ).authors.length,
    16,
  );
  for (const value of [
    { authors: [] },
    { authors: [{ displayName: "Ada Lovelace" }, { displayName: "Ada Lovelace" }] },
    { authors: [{ displayName: "Ada\u0007" }] },
    { authors: [{ displayName: "Ada Lovelace" }], extra: true },
  ]) {
    assert.throws(() => decodeQuestionAuthorship(value, "response.authorship"));
  }

  const client = createHttpApiClient({
    fetch: async () => {
      throw new Error("invalid Question Authorship must not reach fetch");
    },
  });
  await assert.rejects(
    client.publishWorkspace(
      workspace,
      { scope: "public", authorship: { authors: [{ displayName: "Ada\u0007" }] } },
      '"1"',
    ),
    ApiProtocolError,
  );
});

test("publication 422 shapes keep validation unavailability distinct from complete capability violations", async () => {
  const validationUnavailable = createHttpApiClient({
    fetch: async () =>
      jsonResponse(
        { error: "question backend is not registered" },
        { status: 422, headers: { etag: '"1"' } },
      ),
  });
  assert.deepEqual(await validationUnavailable.validateWorkspacePublication(workspace), {
    kind: "questionPublicationValidationUnavailable",
    message: "question backend is not registered",
  });

  const capabilityFailure = createHttpApiClient({
    fetch: async () =>
      jsonResponse(
        {
          error: "publication validation failed",
          violations: [
            {
              workspace,
              title: draft.metadata.title,
              capability: "questionAttemptTimeLimit",
            },
          ],
        },
        { status: 422 },
      ),
  });
  await assert.rejects(
    capabilityFailure.publishWorkspace(
      workspace,
      { scope: "public", authorship: { authors: [{ displayName: "Fixture Instructor" }] } },
      '"1"',
    ),
    (error) =>
      error instanceof PublicationValidationError &&
      error.messageForAuthor === "publication validation failed" &&
      error.violations.length === 1,
  );
});

test("publication revisions reject missing, mismatched, zero, and out-of-range evidence", async () => {
  const badDiff = createHttpApiClient({
    fetch: async () =>
      jsonResponse(
        {
          draftQuestionRevisionNumber: 2,
          baseQuestion: "newQuestion",
          current: questionPublicationReviewCurrent(draft),
          changed: [],
        },
        { headers: { etag: '"1"' } },
      ),
  });
  await assert.rejects(badDiff.getQuestionPublicationReview(workspace), ApiProtocolError);
  const publish = createHttpApiClient({
    fetch: async () => jsonResponse({ error: "required" }, { status: 428 }),
  });
  for (const revision of [undefined, '"0"', '"9223372036854775808"']) {
    await assert.rejects(
      () =>
        publish.publishWorkspace(
          workspace,
          { scope: "public", authorship: { authors: [{ displayName: "Fixture Instructor" }] } },
          revision,
        ),
      /revision|arguments/u,
    );
  }
  await assert.rejects(
    publish.publishWorkspace(
      workspace,
      { scope: "public", authorship: { authors: [{ displayName: "Fixture Instructor" }] } },
      '"1"',
    ),
    WorkspaceConflictError,
  );
});

test("workspace CRUD uses no-store, exact ETags, and never permits a path/body mismatch", async () => {
  const calls = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      calls.push({ input: String(input), init });
      if (init?.method === "PUT") return jsonResponse(draft, { headers: { etag: '"2"' } });
      if (init?.method === "DELETE") return new Response(null, { status: 204 });
      if (String(input).startsWith("/api/workspaces?")) {
        return jsonResponse({
          items: [
            {
              draftQuestion,
              workspace,
              authoringWorkspace: "W-1",
              title: draft.metadata.title,
              questionBackend: draft.questionBackend,
            },
          ],
          nextCursor: null,
        });
      }
      return jsonResponse(draft, { headers: { etag: '"1"' } });
    },
  });

  const page = await client.listWorkspaceDrafts("cursor-1");
  const loaded = await client.getWorkspaceDraft(workspace);
  const saved = await client.saveWorkspaceDraft(workspace, draft, loaded.revision);
  await client.deleteWorkspaceDraft(workspace, saved.revision);

  assert.equal(page.items[0].workspace, workspace);
  assert.equal(saved.revision, '"2"');
  assert.equal(calls[0].init.cache, "no-store");
  assert.equal(calls[2].init.headers["if-match"], '"1"');
  assert.equal(calls[3].init.cache, "no-store");
  assert.equal(calls[3].init.headers["if-match"], '"2"');
  await assert.rejects(
    client.saveWorkspaceDraft("00000000-0000-0000-0000-000000000099", draft),
    ApiProtocolError,
  );
});

test("workspace detail requires a strong ETag and stale updates preserve the editor's local state", async () => {
  const missingEtag = createHttpApiClient({ fetch: async () => jsonResponse(draft) });
  await assert.rejects(missingEtag.getWorkspaceDraft(workspace), ApiProtocolError);

  let saves = 0;
  const client = createHttpApiClient({
    fetch: async (_input, init) => {
      if (init?.method === "PUT") {
        saves += 1;
        return jsonResponse({ error: "changed" }, { status: 409 });
      }
      return jsonResponse(draft, { headers: { etag: '"9"' } });
    },
  });
  const repository = createWorkspaceEditorRepository(client);
  const local = await repository.getDraft(workspace);
  const edited = { ...local, title: "Unsaved local revision" };
  await assert.rejects(repository.saveDraft(edited), WorkspaceConflictError);
  assert.equal(edited.title, "Unsaved local revision");
  assert.equal(saves, 1);
});

test("workspace boundaries reject foreign failures, oversized errors, and contaminated draft records", async () => {
  const foreign = createHttpApiClient({
    fetch: async () => jsonResponse({ error: "not found" }, { status: 404 }),
  });
  await assert.rejects(foreign.getWorkspaceDraft(workspace), ApiRequestError);
  const oversized = createHttpApiClient({
    fetch: async () => jsonResponse({ error: "x".repeat(64 * 1024) }, { status: 413 }),
  });
  await assert.rejects(oversized.saveWorkspaceDraft(workspace, draft), ApiRequestError);

  assert.throws(
    () =>
      decodeDraftQuestionPage({
        items: [{ workspace, title: draft.metadata.title, sourceBackend: "ple" }],
        nextCursor: null,
      }),
    /allowed by this response contract/u,
  );
  assert.throws(
    () =>
      decodeDraftQuestionPage({
        items: [
          {
            draftQuestion: "00000000-0000-0000-0000-000000000901",
            workspace,
            authoringWorkspace: "W-1",
            title: draft.metadata.title,
            questionBackend: "ple",
          },
        ],
        nextCursor: null,
      }),
    /D- reference/u,
  );
  const contaminated = { ...draft, problem: "00000000-0000-0000-0000-000000000099" };
  const client = createHttpApiClient({
    fetch: async () => jsonResponse(contaminated, { headers: { etag: '"1"' } }),
  });
  await assert.rejects(client.getWorkspaceDraft(workspace), /allowed by this response contract/u);
});

test("stale delete after a collaborator save sends the old ETag and returns a reloadable conflict", async () => {
  let revision = 1;
  const calls = [];
  const client = createHttpApiClient({
    fetch: async (_input, init) => {
      calls.push(init);
      if (init?.method === "DELETE") {
        return jsonResponse({ error: "changed" }, { status: 409 });
      }
      return jsonResponse(draft, { headers: { etag: `"${revision}"` } });
    },
  });
  const repository = createWorkspaceEditorRepository(client);
  await repository.getDraft(workspace);
  assert.equal(repository.displayedRevision?.(workspace), '"1"');
  revision = 2;
  await assert.rejects(repository.deleteDraft(workspace), WorkspaceConflictError);
  assert.equal(calls.at(-1).headers["if-match"], '"1"');
  assert.equal(repository.displayedRevision?.(workspace), '"1"');
  const reloaded = await repository.reloadDraft(workspace);
  assert.equal(reloaded.workspace, workspace);
  assert.equal(repository.displayedRevision?.(workspace), '"2"');
});

test("delete refuses a missing precondition response without discarding the workspace", async () => {
  const client = createHttpApiClient({
    fetch: async (_input, init) => {
      if (init?.method === "DELETE") return jsonResponse({ error: "required" }, { status: 428 });
      return jsonResponse(draft, { headers: { etag: '"4"' } });
    },
  });
  const repository = createWorkspaceEditorRepository(client);
  const local = await repository.getDraft(workspace);
  await assert.rejects(repository.deleteDraft(workspace), WorkspaceConflictError);
  assert.equal(local.title, draft.metadata.title);
});
