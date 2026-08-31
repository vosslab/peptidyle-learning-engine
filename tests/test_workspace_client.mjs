// test_workspace_client.mjs - behavior contracts for private workspace CRUD transport.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "./fixtures/published_problem.ts";
import {
  createHttpApiClient,
  ApiProtocolError,
  ApiRequestError,
  WorkspaceConflictError,
  PublicationValidationError,
} from "../src/api/http_client.ts";
import { decodePublicationDiff, decodeWorkspaceDraftPage } from "../src/api/decoders.ts";
import { decodePublicByline, parseReviewedPublicByline } from "../src/api/public_byline.ts";
import { createWorkspaceEditorRepository } from "../src/pages/editor_workspace_repository.ts";

const draft = publishedProblemFixture.draft;
const workspace = draft.workspace;

function jsonResponse(value, options = {}) {
  return new Response(JSON.stringify(value), {
    status: options.status ?? 200,
    headers: { "content-type": "application/json", ...options.headers },
  });
}

function semanticProjection(definition) {
  const response = definition.response;
  const optionCount =
    response.kind === "multipleChoice"
      ? response.choices.length
      : response.kind === "ordering"
        ? response.items.length
        : null;
  return {
    sourceBackend: definition.source.backend,
    title: definition.metadata.title,
    prompt: { blocks: definition.prompt.map((block) => block.kind) },
    response: { kind: response.kind, optionCount },
    questionAttemptLimit: definition.questionAttemptLimit,
    questionAttemptTimeLimit: definition.questionAttemptTimeLimit,
    randomization: { kind: definition.randomization.kind },
    metadata: {
      tags: definition.metadata.tags,
      taxonomy: definition.metadata.taxonomy,
      license: definition.metadata.license,
      language: definition.metadata.language,
    },
  };
}

test("publication diff recursively admits only semantic projections and consistent baselines", () => {
  const current = semanticProjection(draft);
  const first = {
    draftRevision: 1,
    baseline: "newQuestion",
    current,
    changed: [],
  };
  assert.deepEqual(decodePublicationDiff(first), { ...first, revision: '"1"' });
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
    assert.throws(() => decodePublicationDiff(contaminated));
  }
});

test("publication transport uses a bodyless validation request and an explicit reviewed byline", async () => {
  const calls = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      calls.push({ input: String(input), init });
      if (String(input).endsWith("publication-validation")) {
        return jsonResponse({ violations: [] }, { headers: { etag: '"1"' } });
      }
      if (String(input).endsWith("publication-diff")) {
        return jsonResponse(
          {
            draftRevision: 1,
            baseline: "newQuestion",
            current: semanticProjection(draft),
            changed: [],
          },
          { headers: { etag: '"1"' } },
        );
      }
      return jsonResponse(publishedProblemFixture.publishedQuestion);
    },
  });
  await client.validateWorkspacePublication(workspace);
  await client.getWorkspacePublicationDiff(workspace);
  const request = { scope: "public", byline: { names: ["Fixture Instructor"] } };
  await client.publishWorkspace(workspace, request, '"1"');
  assert.equal(calls[0].init.method, "POST");
  assert.equal(calls[0].init.body, undefined);
  assert.equal(calls[0].init.headers["content-type"], undefined);
  assert.equal(calls[2].init.body, JSON.stringify(request));
  assert.equal(calls[2].init.headers["content-type"], "application/json");
});

test("reviewed bylines share strict line-based author input and wire decoding", async () => {
  assert.deepEqual(parseReviewedPublicByline("  Ada Lovelace  \nGrace Hopper"), {
    names: ["Ada Lovelace", "Grace Hopper"],
  });
  for (const text of ["", "Ada\nAda", "Ada\u0007", "😀".repeat(121)]) {
    assert.equal(parseReviewedPublicByline(text), null);
  }
  assert.deepEqual(decodePublicByline({ names: ["Ada Lovelace"] }, "response.byline"), {
    names: ["Ada Lovelace"],
  });
  for (const value of [
    { names: [] },
    { names: ["Ada Lovelace", "Ada Lovelace"] },
    { names: ["Ada\u0007"] },
    { names: ["Ada Lovelace"], extra: true },
  ]) {
    assert.throws(() => decodePublicByline(value, "response.byline"));
  }

  const client = createHttpApiClient({
    fetch: async () => {
      throw new Error("invalid bylines must not reach fetch");
    },
  });
  await assert.rejects(
    client.publishWorkspace(
      workspace,
      { scope: "public", byline: { names: ["Ada\u0007"] } },
      '"1"',
    ),
    ApiProtocolError,
  );
});

test("publication 422 shapes keep readiness distinct from complete capability violations", async () => {
  const readiness = createHttpApiClient({
    fetch: async () =>
      jsonResponse(
        { error: "question backend is not registered" },
        { status: 422, headers: { etag: '"1"' } },
      ),
  });
  assert.deepEqual(await readiness.validateWorkspacePublication(workspace), {
    kind: "readinessFailure",
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
      { scope: "public", byline: { names: ["Fixture Instructor"] } },
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
          draftRevision: 2,
          baseline: "newQuestion",
          current: semanticProjection(draft),
          changed: [],
        },
        { headers: { etag: '"1"' } },
      ),
  });
  await assert.rejects(badDiff.getWorkspacePublicationDiff(workspace), ApiProtocolError);
  const publish = createHttpApiClient({
    fetch: async () => jsonResponse({ error: "required" }, { status: 428 }),
  });
  for (const revision of [undefined, '"0"', '"9223372036854775808"']) {
    await assert.rejects(
      () =>
        publish.publishWorkspace(
          workspace,
          { scope: "public", byline: { names: ["Fixture Instructor"] } },
          revision,
        ),
      /revision|arguments/u,
    );
  }
  await assert.rejects(
    publish.publishWorkspace(
      workspace,
      { scope: "public", byline: { names: ["Fixture Instructor"] } },
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
              workspace,
              reference: "W-1",
              title: draft.metadata.title,
              sourceBackend: draft.source.backend,
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
      decodeWorkspaceDraftPage({
        items: [
          { workspace, title: draft.metadata.title, sourceBackend: "native", version: "forbidden" },
        ],
        nextCursor: null,
      }),
    /allowed by this response contract/u,
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
