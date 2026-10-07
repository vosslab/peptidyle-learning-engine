import assert from "node:assert/strict";
import test from "node:test";

import { createQuestionDraftAutosave } from "../src/features/ple_question_json_authoring/question_draft_autosave.ts";
import { createDraftQuestionPreviewClient } from "../src/features/question_draft_preview/draft_preview_client.ts";

const draftQuestion = "018f0000-0000-7000-8000-000000000001";
const delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

function draftSourceHeaders({ backend, format, mediaType, path, editNumber }) {
  return {
    "content-type": mediaType,
    etag: `"${editNumber}"`,
    "x-ple-question-backend": backend,
    "x-ple-question-format": format,
    "x-ple-webwork-pg-path": path === null ? "" : encodeURIComponent(path),
  };
}

function deferred() {
  let resolve;
  const promise = new Promise((finish) => {
    resolve = finish;
  });
  return { promise, resolve };
}

test("source GET and PUT preserve exact PGML text and registered binding", async () => {
  const source = "\n[% incomplete\t\n  $answer = 4;  \n";
  const requests = [];
  const client = createDraftQuestionPreviewClient({
    fetch: async (input, init) => {
      requests.push({ input, init });
      if (init?.method === "GET") {
        return new Response(source, {
          headers: draftSourceHeaders({
            backend: "webwork",
            format: "webworkPgml",
            mediaType: "text/x-wework-pg; charset=utf-8",
            path: "Library/Genetics/incomplete source.pg",
            editNumber: "17",
          }),
        });
      }
      return new Response(null, {
        status: 204,
        headers: { etag: '"18"' },
      });
    },
  });

  const loaded = await client.loadSource(draftQuestion);
  assert.equal(loaded.source, source);
  assert.equal(loaded.draftQuestionEditNumber, "17");
  assert.deepEqual(loaded.binding, {
    backend: "webwork",
    format: "webworkPgml",
    mediaType: "text/x-wework-pg",
    webworkPgPath: "Library/Genetics/incomplete source.pg",
  });
  assert.equal(requests[0].init.headers.accept, "*/*");

  const nextEditNumber = await client.saveSource(draftQuestion, loaded.binding, source, "17");
  assert.equal(nextEditNumber, "18");
  assert.equal(requests[1].init.body, source);
  assert.equal(requests[1].init.headers["content-type"], "text/x-wework-pg");
  assert.equal(requests[1].init.headers["if-match"], '"17"');
});

test("empty PG source stays empty through its registered media type", async () => {
  const requests = [];
  const client = createDraftQuestionPreviewClient({
    fetch: async (input, init) => {
      requests.push({ input, init });
      if (init?.method === "GET") {
        return new Response("", {
          headers: draftSourceHeaders({
            backend: "webwork",
            format: "webworkPg",
            mediaType: "text/x-wework-pg",
            path: "Library/Genetics/empty.pg",
            editNumber: "4",
          }),
        });
      }
      return new Response(null, { status: 204, headers: { etag: '"5"' } });
    },
  });

  const loaded = await client.loadSource(draftQuestion);
  assert.equal(loaded.source, "");
  assert.equal(await client.saveSource(draftQuestion, loaded.binding, loaded.source, "4"), "5");
  assert.equal(requests[1].init.body, "");
  assert.equal(requests[1].init.headers["content-type"], "text/x-wework-pg");
});

test("empty and malformed Native source loads without JSON parsing", async () => {
  const sources = ["", '{"format":"pleQuestionJson",'];
  let requestIndex = 0;
  const writes = [];
  const client = createDraftQuestionPreviewClient({
    fetch: async (_input, init) => {
      if (init?.method === "PUT") {
        writes.push(init);
        return new Response(null, { status: 204, headers: { etag: '"23"' } });
      }
      const source = sources[requestIndex] ?? "";
      requestIndex += 1;
      return new Response(source, {
        headers: draftSourceHeaders({
          backend: "ple",
          format: "pleQuestionJson",
          mediaType: "application/vnd.peptidyle.question+json",
          path: null,
          editNumber: String(20 + requestIndex),
        }),
      });
    },
  });

  const empty = await client.loadSource(draftQuestion);
  const malformed = await client.loadSource(draftQuestion);
  assert.equal(empty.source, "");
  assert.equal(malformed.source, '{"format":"pleQuestionJson",');
  assert.equal(
    await client.saveSource(
      draftQuestion,
      malformed.binding,
      malformed.source,
      malformed.draftQuestionEditNumber,
    ),
    "23",
  );
  assert.equal(writes[0].body, '{"format":"pleQuestionJson",');
  assert.equal(writes[0].headers["content-type"], "application/vnd.peptidyle.question+json");
});

test("queued source writes use the newest acknowledged Edit Number", async () => {
  const firstWriteGate = deferred();
  const firstStarted = deferred();
  const secondStarted = deferred();
  const requests = [];
  let activeWrites = 0;
  let maximumActiveWrites = 0;
  let editNumber = "16";
  const client = createDraftQuestionPreviewClient({
    fetch: async (_input, init) => {
      const writeNumber = requests.length;
      requests.push(init);
      activeWrites += 1;
      maximumActiveWrites = Math.max(maximumActiveWrites, activeWrites);
      if (writeNumber === 0) {
        firstStarted.resolve();
        await firstWriteGate.promise;
      } else {
        secondStarted.resolve();
      }
      activeWrites -= 1;
      return new Response(null, {
        status: 204,
        headers: { etag: `"${17 + writeNumber}"` },
      });
    },
  });
  const binding = {
    backend: "webwork",
    format: "webworkPg",
    mediaType: "text/x-wework-pg",
    webworkPgPath: "Library/Genetics/in-progress.pg",
  };
  const autosave = createQuestionDraftAutosave({
    debounceMs: 5,
    save: async (source) => {
      editNumber = await client.saveSource(draftQuestion, binding, source, editNumber);
    },
  });

  autosave.edit("first snapshot");
  await firstStarted.promise;
  autosave.edit("latest\nunfinished source  ");
  await delay(15);
  firstWriteGate.resolve();
  await secondStarted.promise;
  await autosave.flush();

  assert.equal(requests.length, 2);
  assert.equal(maximumActiveWrites, 1);
  assert.equal(requests[0].headers["if-match"], '"16"');
  assert.equal(requests[0].body, "first snapshot");
  assert.equal(requests[1].headers["if-match"], '"17"');
  assert.equal(requests[1].body, "latest\nunfinished source  ");
  assert.equal(editNumber, "18");
  assert.equal(autosave.state().status, "saved");
  assert.equal(autosave.state().generation, autosave.state().acknowledgedGeneration);
  autosave.dispose();
});

test("a failed source save retains the newest work for retry", async () => {
  const requests = [];
  const firstWriteGate = deferred();
  const firstStarted = deferred();
  const client = createDraftQuestionPreviewClient({
    fetch: async (_input, init) => {
      requests.push(init);
      if (requests.length === 1) {
        firstStarted.resolve();
        await firstWriteGate.promise;
        return new Response(null, { status: 412 });
      }
      return new Response(null, { status: 204, headers: { etag: '"20"' } });
    },
  });
  const binding = {
    backend: "webwork",
    format: "webworkPg",
    mediaType: "text/x-wework-pg",
    webworkPgPath: "Library/Genetics/broken.pg",
  };
  let localSource = "[% first unfinished source";
  let editNumber = "19";
  const autosave = createQuestionDraftAutosave({
    debounceMs: 5,
    save: async (source) => {
      editNumber = await client.saveSource(draftQuestion, binding, source, editNumber);
    },
  });

  autosave.edit(localSource);
  const failedWrite = autosave.flush();
  await firstStarted.promise;
  localSource = "[% newest text remains available after failure";
  autosave.edit(localSource);
  await delay(15);
  firstWriteGate.resolve();
  await assert.rejects(failedWrite);
  assert.equal(autosave.state().status, "error");
  assert.equal(autosave.state().generation, 2);
  assert.equal(autosave.state().acknowledgedGeneration, 0);

  await autosave.flush();
  assert.equal(requests.length, 2);
  assert.equal(requests[0].body, "[% first unfinished source");
  assert.equal(requests[1].body, localSource);
  assert.equal(requests[0].headers["if-match"], '"19"');
  assert.equal(requests[1].headers["if-match"], '"19"');
  assert.equal(editNumber, "20");
  assert.equal(autosave.state().status, "saved");
  autosave.dispose();
});
