// editor_mock_surface.spec.ts - mounted resilience proof for the injected editor boundary.

import { expect, test } from "@playwright/test";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

let fixtureScript = "";
let liveFixtureScript = "";
const LIVE_WORKSPACE_ID = "0198e000-0000-7000-8000-000000000002";

interface EditorTransportCall {
  readonly method: string;
  readonly path: string;
  readonly body: string | null;
  readonly ifMatch: string | null;
}

interface EditorFixtureWindow {
  readonly __editorLiveFixture: {
    readonly calls: ReadonlyArray<EditorTransportCall>;
    readonly probeForeign: () => Promise<string>;
  };
}

function nestedKeys(value: unknown): ReadonlyArray<string> {
  if (Array.isArray(value)) return value.flatMap(nestedKeys);
  if (typeof value !== "object" || value === null) return [];
  return Object.entries(value).flatMap(([key, child]) => [key, ...nestedKeys(child)]);
}

test.beforeAll(async () => {
  const result = await build({
    bundle: true,
    format: "iife",
    minify: false,
    plugins: [solidPlugin()],
    platform: "browser",
    stdin: {
      contents: `
        import { createComponent } from "solid-js";
        import { render } from "solid-js/web";
        import { EditorPage } from "./src/pages/editor_page.tsx";
        import { WorkspaceConflictError } from "./src/api/http_client.ts";
        import { InstructorPreviewConflictError } from "./src/pages/editor_instructor_preview.ts";

        const workspace = "00000000-0000-0000-0000-000000000010";
        const draft = {
          workspace,
          title: "Peptide-bond geometry",
          source: { backend: "native", family: "peptide-bond" },
          prompt: [
            { kind: "text", markdown: "Estimate the omega angle." },
            { kind: "math", latex: "\\\\omega", description: "omega" },
          ],
          response: { kind: "numeric", tolerance: { kind: "exact" }, unit: null },
          attemptPolicy: { maxAttempts: null, feedback: "immediateCorrectness" },
          timingPolicy: { kind: "untimed" },
          randomization: { kind: "static" },
        };
        let validationCalls = 0;
        let instructorPreviewChecks = 0;
        let mostRecentlySavedTitle = draft.title;
        const repository = {
          listDrafts: async () => ({
            items: [{ workspace, title: draft.title, sourceBackend: "native" }],
            nextCursor: null,
          }),
          getDraft: async () => draft,
          saveDraft: async (candidate) => {
            if (candidate.title === "Conflicting instructor revision") {
              throw new WorkspaceConflictError(409, "/api/workspaces/fixture");
            }
            mostRecentlySavedTitle = candidate.title;
            return candidate;
          },
          validateCapabilities: async () => {
            validationCalls += 1;
            if (validationCalls === 1) return [{ workspace, title: draft.title, capability: "hints" }];
            if (validationCalls === 2) throw new Error("Capability service temporarily unavailable");
            return [];
          },
          getPublishDiff: async () => ({
            revision: '"1"',
            baseline: "newQuestion",
            proposedTitle: draft.title,
            sections: [{ label: "Prompt", before: "Old", after: "New" }],
          }),
          publish: async () => { throw new Error("Publication service temporarily unavailable"); },
          deleteDraft: async () => { throw new WorkspaceConflictError(409, "/api/workspaces/fixture"); },
          reloadDraft: async () => ({ ...draft, title: "Collaborator revision" }),
          instructorPreview: {
            requestPresentation: async (_draft, _seed) => {
              if (
                mostRecentlySavedTitle !== "Retained during preview checks" &&
                mostRecentlySavedTitle !== "Author preview race"
              ) {
                throw new Error("Instructor preview was requested before saving the current draft");
              }
              instructorPreviewChecks += 1;
              if (mostRecentlySavedTitle === "Author preview race") {
                if (instructorPreviewChecks === 1) {
                  return {
                    kind: "available",
                    revision: '"1"',
                    presentation: {
                      title: draft.title,
                      prompt: draft.prompt,
                      response: draft.response,
                      seed: 101,
                      correctResponse: [{ kind: "text", markdown: "A planar peptide bond." }],
                    },
                  };
                }
                throw new InstructorPreviewConflictError(409);
              }
              if (instructorPreviewChecks === 1) {
                return { kind: "unavailable", revision: '"1"', backend: "webwork", reason: "No safe server derivation is available." };
              }
              throw new Error("Protected preview availability check failed");
            },
          },
        };
        const previewFacade = {
          preview: async (candidate, seed) => ({
            workspace: candidate.workspace,
            seed,
            title: candidate.title,
            prompt: candidate.prompt,
            response: candidate.response,
          }),
        };
        const responseValidator = {
          mode: "serverFallback",
          degradedReason: "fixture",
          validateResponseFormat: async () => ({ violations: [] }),
          timerVerdict: async () => "untimed",
          validateAssignmentConfig: async () => [],
        };
        const fixture = document.createElement("div");
        fixture.id = "editor-fixture";
        document.body.appendChild(fixture);
        render(() => createComponent(EditorPage, { repository, previewFacade, responseValidator }), fixture);
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "editor_mock_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles[0];
  if (output === undefined) throw new Error("Editor fixture bundle was not produced.");
  fixtureScript = output.text;

  const live = await build({
    bundle: true,
    format: "iife",
    minify: false,
    plugins: [solidPlugin()],
    platform: "browser",
    outdir: "/tmp/ple-editor-live-fixture",
    stdin: {
      contents: `
        import { createComponent } from "solid-js";
        import { render } from "solid-js/web";
        import { publishedProblemFixture } from "./generated/fixtures/published_problem.ts";
        import { createHttpApiClient } from "./src/api/http_client.ts";
        import { createInstructorPreviewClient } from "./src/pages/editor_instructor_preview.ts";
        import { createWorkspaceEditorRepository } from "./src/pages/editor_workspace_repository.ts";
        import { EditorPage } from "./src/pages/editor_page.tsx";

        const workspace = publishedProblemFixture.draft.workspace;
        const calls = [];
        let validationCount = 0;
        let diffCount = 0;
        let publishCount = 0;
        let saveCount = 0;
        let authorPreviewCount = 0;
        const definition = (title) => ({ ...publishedProblemFixture.draft, metadata: { ...publishedProblemFixture.draft.metadata, title } });
        const semantic = (draft) => ({
          sourceBackend: draft.source.backend,
          title: draft.metadata.title,
          prompt: { blocks: draft.prompt.map((block) => block.kind) },
          response: { kind: draft.response.kind, optionCount: draft.response.kind === "multipleChoice" ? draft.response.choices.length : null },
          attemptPolicy: draft.attemptPolicy,
          timingPolicy: draft.timingPolicy,
          randomization: { kind: draft.randomization.kind },
          metadata: { tags: draft.metadata.tags, taxonomy: draft.metadata.taxonomy, license: draft.metadata.license, language: draft.metadata.language },
        });
        const violation = (capability) => ({ workspace, title: "Saved workspace draft", capability });
        const json = (value, status = 200, headers = {}) => new Response(JSON.stringify(value), { status, headers: { "content-type": "application/json", ...headers } });
        const transport = async (input, init) => {
          const url = new URL(String(input), window.location.origin);
          const method = init?.method ?? "GET";
          const body = typeof init?.body === "string" ? init.body : null;
          calls.push({ method, path: url.pathname + url.search, body, ifMatch: init?.headers?.["if-match"] ?? null });
          if (url.pathname === "/api/workspaces" && method === "GET") return json({ items: [{ workspace, publicId: 1, title: "Saved workspace draft", sourceBackend: "native" }], nextCursor: null });
          if (url.pathname === "/api/workspaces/foreign" && method === "GET") return json({ error: "not found" }, 404);
          if (url.pathname === "/api/workspaces/" + workspace && method === "GET") {
            return json(
              definition(saveCount > 1 ? "Collaborator saved title" : "Saved workspace draft"),
              200,
              { etag: saveCount > 1 ? '"4"' : '"1"' },
            );
          }
          if (url.pathname === "/api/workspaces/" + workspace && method === "PUT") {
            const candidate = body === null ? null : JSON.parse(body);
            if (candidate?.metadata?.title === "Saved workspace draft") {
              return json(definition("Saved workspace draft"), 200, { etag: '"2"' });
            }
            saveCount += 1;
            if (saveCount === 1) return json(definition("Edited title survives readiness refusal"), 200, { etag: '"2"' });
            if (saveCount === 2) return json(definition("Post-diff local revision"), 200, { etag: '"3"' });
            if (saveCount === 3) return json(definition("Recovered intended title"), 200, { etag: '"5"' });
            return json(definition("Recovered intended title"), 200, { etag: '"6"' });
          }
          if (url.pathname === "/api/workspaces/" + workspace + "/author-preview" && method === "GET") {
            authorPreviewCount += 1;
            const etag = authorPreviewCount === 1 ? '"2"' : '"3"';
            return json(
              {
                kind: "available",
                title: "Saved workspace draft",
                prompt: publishedProblemFixture.draft.prompt,
                response: publishedProblemFixture.draft.response,
                seed: Number(url.searchParams.get("seed")),
                correctResponse: [{ kind: "text", markdown: "A planar peptide bond." }],
                rationale: [{ kind: "text", markdown: "Resonance constrains omega." }],
              },
              200,
              { etag },
            );
          }
          if (url.pathname.endsWith("/publication-validation")) {
            validationCount += 1;
            const etag = '"' + (saveCount === 0 ? 1 : saveCount + 1) + '"';
            if (validationCount === 2) return json({ error: "native backend readiness is unavailable" }, 422);
            return json({ violations: validationCount === 1 ? [violation("hints"), violation("perQuestionTiming")] : [] }, 200, { etag });
          }
          if (url.pathname.endsWith("/publication-diff")) {
            diffCount += 1;
            const current = semantic(definition(
              diffCount === 1 ? "Edited title survives readiness refusal" : diffCount === 2 ? "Post-diff local revision" : "Recovered intended title",
            ));
            return json(
              {
                draftRevision: diffCount === 1 ? 2 : diffCount === 2 ? 3 : 6,
                baseline: "newQuestion",
                current,
                changed: [],
              },
              200,
              { etag: diffCount === 1 ? '"2"' : diffCount === 2 ? '"3"' : '"6"' },
            );
          }
          if (url.pathname === "/api/problems/" + workspace + "/publish" && method === "POST") {
            publishCount += 1;
            if (publishCount === 2) return json({ error: "stale" }, 409);
            return json(publishedProblemFixture.catalogProblem);
          }
          return json({ error: "unexpected fixture request" }, 404);
        };
        const client = createHttpApiClient({ fetch: transport });
        window.__editorLiveFixture = {
          calls,
          probeForeign: async () => {
            try { await client.getWorkspaceDraft("foreign"); return "unexpected"; }
            catch (error) { return error instanceof Error ? error.name : "unknown"; }
          },
        };
        const previewFacade = { preview: async (draft, seed) => ({ workspace: draft.workspace, seed, title: draft.title, prompt: draft.prompt, response: draft.response }) };
        const responseValidator = { mode: "serverFallback", degradedReason: "fixture", validateResponseFormat: async () => ({ violations: [] }), timerVerdict: async () => "untimed", validateAssignmentConfig: async () => [] };
        const mount = document.createElement("div"); mount.id = "editor-live-fixture"; document.body.appendChild(mount);
        const authorPreview = createInstructorPreviewClient({ fetch: transport });
        render(() => createComponent(EditorPage, { repository: createWorkspaceEditorRepository(client, authorPreview), previewFacade, responseValidator }), mount);
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "editor_live_transport_fixture.tsx",
    },
    write: false,
  });
  const liveOutput = live.outputFiles.find((candidate) => candidate.path.endsWith(".js"));
  if (liveOutput === undefined) throw new Error("Editor live fixture bundle was not produced.");
  liveFixtureScript = liveOutput.text;
});

test("editor retains authored input and visible capability guidance through recoverable failures", async ({
  page,
}) => {
  const requests: string[] = [];
  page.on("request", (request) => requests.push(request.url()));
  await page.goto("/");
  requests.length = 0;
  await page.addScriptTag({ content: fixtureScript });

  const fixture = page.locator("#editor-fixture");
  const title = fixture.getByLabel("Question title");
  await expect(title).toHaveValue("Peptide-bond geometry");
  await title.fill("Preserved instructor revision");

  const hints = fixture.getByLabel("Require hints");
  await hints.check();
  await expect(
    fixture.getByRole("alert").filter({ hasText: "cannot provide hints" }),
  ).toContainText("Peptide-bond geometry cannot provide hints");
  await hints.uncheck();
  await expect(
    fixture.getByRole("alert").filter({ hasText: "Publication readiness:" }),
  ).toContainText("Capability service temporarily unavailable");
  await expect(
    fixture.getByRole("alert").filter({ hasText: "cannot provide hints" }),
  ).toContainText("Peptide-bond geometry cannot provide hints");

  await fixture.getByRole("button", { name: "Review publication changes" }).click();
  await expect(fixture.getByRole("heading", { name: "Publication changes" })).toBeVisible();
  await fixture.getByRole("button", { name: "Confirm publication" }).click();
  await expect(fixture.getByText("Publication service temporarily unavailable")).toBeVisible();
  await expect(title).toHaveValue("Preserved instructor revision");
  expect(requests.filter((url) => new URL(url).pathname.startsWith("/api/"))).toEqual([]);
});

test("ordinary preview is local while explicit author preview reports protected states", async ({
  page,
}) => {
  const requests: string[] = [];
  page.on("request", (request) => requests.push(request.url()));
  await page.goto("/");
  requests.length = 0;
  await page.addScriptTag({ content: fixtureScript });

  const fixture = page.locator("#editor-fixture");
  const title = fixture.getByLabel("Question title");
  await title.fill("Retained during preview checks");
  await fixture.getByRole("button", { name: "Preview this variation" }).click();
  await expect(fixture.getByRole("heading", { name: "Question", exact: true })).toBeVisible();
  await expect(fixture.getByRole("button", { name: "Submit answer" })).toBeVisible();

  const authorPreview = fixture.getByRole("button", { name: "Load instructor answer preview" });
  await authorPreview.click();
  await expect(
    fixture.getByRole("status").filter({ hasText: "webwork author preview is unavailable" }),
  ).toContainText("No safe server derivation is available.");
  await authorPreview.click();
  await expect(
    fixture.getByRole("alert").filter({ hasText: "Protected preview availability check failed" }),
  ).toContainText("Try again.");
  await expect(title).toHaveValue("Retained during preview checks");
  expect(
    requests.filter((url) => {
      const path = new URL(url).pathname.toLowerCase();
      return path.startsWith("/api/") || path.includes("key");
    }),
  ).toEqual([]);
});

test("a stale author-preview save preserves local edits and does not render a prior answer", async ({
  page,
}) => {
  await page.goto("/");
  await page.addScriptTag({ content: fixtureScript });

  const fixture = page.locator("#editor-fixture");
  const title = fixture.getByLabel("Question title");
  await title.fill("Conflicting instructor revision");
  await fixture.getByRole("button", { name: "Load instructor answer preview" }).click();
  await expect(
    fixture.getByRole("alert").filter({ hasText: "Reload, save your edits, and try again." }),
  ).toContainText("Someone saved a newer revision.");
  await expect(title).toHaveValue("Conflicting instructor revision");
  await expect(fixture.getByRole("heading", { name: "Correct response" })).toHaveCount(0);
});

test("a protected preview conflict clears an old answer and offers reload recovery", async ({
  page,
}) => {
  await page.goto("/");
  await page.addScriptTag({ content: fixtureScript });

  const fixture = page.locator("#editor-fixture");
  const title = fixture.getByLabel("Question title");
  await title.fill("Author preview race");
  await fixture.getByRole("button", { name: "Load instructor answer preview" }).click();
  await expect(fixture.getByRole("heading", { name: "Correct response" })).toBeVisible();

  await fixture.getByRole("button", { name: "Load instructor answer preview" }).click();
  await expect(
    fixture.getByRole("alert").filter({ hasText: "Reload, save your edits, and try again." }),
  ).toBeVisible();
  await expect(fixture.getByRole("button", { name: "Reload newest draft" })).toBeVisible();
  await expect(title).toHaveValue("Author preview race");
  await expect(fixture.getByRole("heading", { name: "Correct response" })).toHaveCount(0);
});

test("a stale delete retains local edits and offers the same reload recovery as a stale save", async ({
  page,
}) => {
  await page.goto("/");
  await page.addScriptTag({ content: fixtureScript });

  const fixture = page.locator("#editor-fixture");
  const title = fixture.getByLabel("Question title");
  await title.fill("Unsaved before delete");
  await fixture.getByRole("button", { name: "Delete this draft" }).click();
  await expect(fixture.getByRole("alert")).toContainText("Someone saved a newer revision");
  await expect(title).toHaveValue("Unsaved before delete");
  await fixture.getByRole("button", { name: "Reload newest draft" }).click();
  await expect(title).toHaveValue("Collaborator revision");
});

test("live author preview saves first, binds its ETag, and refuses a stale presentation", async ({
  page,
}) => {
  await page.goto("/");
  await page.addScriptTag({ content: liveFixtureScript });
  const fixture = page.locator("#editor-live-fixture");
  const callCountBeforePreview = await page.evaluate(() => {
    const fixtureWindow = window as unknown as EditorFixtureWindow;
    return fixtureWindow.__editorLiveFixture.calls.length;
  });

  await fixture.getByRole("button", { name: "Preview this variation" }).click();
  await expect(fixture.getByRole("heading", { name: "Question", exact: true })).toBeVisible();
  const callCountAfterLocalPreview = await page.evaluate(() => {
    const fixtureWindow = window as unknown as EditorFixtureWindow;
    return fixtureWindow.__editorLiveFixture.calls.length;
  });
  expect(callCountAfterLocalPreview).toBe(callCountBeforePreview);

  await fixture.getByRole("button", { name: "Load instructor answer preview" }).click();
  await expect(fixture.getByRole("heading", { name: "Correct response" })).toBeVisible();
  await expect(fixture.getByRole("heading", { name: "Why this works" })).toHaveCount(1);
  await expect(
    fixture.locator(".instructor-preview__card").getByText("Resonance constrains omega."),
  ).toHaveCount(1);
  const afterAvailable = await page.evaluate(() => {
    const fixtureWindow = window as unknown as EditorFixtureWindow;
    return fixtureWindow.__editorLiveFixture.calls;
  });
  const authorCalls = afterAvailable.slice(callCountBeforePreview);
  expect(authorCalls).toHaveLength(2);
  expect(authorCalls[0]).toMatchObject({
    method: "PUT",
    path: `/api/workspaces/${LIVE_WORKSPACE_ID}`,
    ifMatch: '"1"',
  });
  expect(authorCalls[0]?.body).toContain('"title":"Saved workspace draft"');
  expect(authorCalls[1]).toEqual({
    method: "GET",
    path: `/api/workspaces/${LIVE_WORKSPACE_ID}/author-preview?seed=101`,
    body: null,
    ifMatch: '"2"',
  });

  await fixture.getByRole("button", { name: "Load instructor answer preview" }).click();
  await expect(
    fixture.getByRole("alert").filter({ hasText: "does not match the saved draft" }),
  ).toBeVisible();
  await expect(fixture.getByRole("heading", { name: "Correct response" })).toHaveCount(0);
});

test("live editor preserves drafts across publication refusals and publishes scope-only intent", async ({
  page,
}) => {
  await page.goto("/");
  await page.addScriptTag({ content: liveFixtureScript });
  const fixture = page.locator("#editor-live-fixture");
  const title = fixture.getByLabel("Question title");
  await expect(title).toHaveValue("Saved workspace draft");
  await fixture.getByLabel("Require hints").check();
  await expect(fixture.getByText("cannot provide hints")).toBeVisible();
  await expect(fixture.getByText("cannot provide per question timing")).toBeVisible();
  await expect(fixture.getByRole("heading", { name: "Publication changes" })).toHaveCount(0);

  await title.fill("Edited title survives readiness refusal");
  await fixture.getByLabel("Require hints").uncheck();
  await expect(
    fixture.getByRole("alert").filter({ hasText: "native backend readiness is unavailable" }),
  ).toBeVisible();
  await expect(title).toHaveValue("Edited title survives readiness refusal");

  await fixture.getByRole("button", { name: "Review publication changes" }).click();
  await expect(fixture.getByText("This publication creates a new Question ID.")).toBeVisible();
  await expect(fixture.getByText("Publishing saved title:")).toContainText(
    "Edited title survives readiness refusal",
  );
  await expect(fixture.getByLabel("Publication scope")).toHaveValue("institution");
  await fixture.getByLabel("Publication scope").selectOption("public");
  await fixture.getByRole("button", { name: "Confirm publication" }).click();
  await expect(
    fixture.getByRole("status").filter({
      hasText: "The new Question ID is now available in the library.",
    }),
  ).toBeVisible();

  await title.fill("Post-diff local revision");
  await fixture.getByRole("button", { name: "Review publication changes" }).click();
  await expect(fixture.getByText("This publication creates a new Question ID.")).toBeVisible();
  await fixture.getByRole("button", { name: "Confirm publication" }).click();
  await expect(
    fixture.getByRole("alert").filter({ hasText: "Reload, save your edits" }),
  ).toBeVisible();
  await expect(fixture.getByRole("button", { name: "Confirm publication" })).toHaveCount(0);
  await expect(title).toHaveValue("Post-diff local revision");
  await fixture.getByRole("button", { name: "Reload newest draft" }).click();
  await expect(title).toHaveValue("Collaborator saved title");

  await title.fill("Recovered intended title");
  await fixture.getByRole("button", { name: "Save draft" }).click();
  await expect(fixture.getByRole("status")).toContainText("Draft saved.");
  await fixture.getByRole("button", { name: "Review publication changes" }).click();
  await expect(fixture.getByText("Publishing saved title:")).toContainText(
    "Recovered intended title",
  );
  await fixture.getByRole("button", { name: "Confirm publication" }).click();
  await expect(
    fixture.getByRole("status").filter({
      hasText: "The new Question ID is now available in the library.",
    }),
  ).toBeVisible();

  const evidence = await page.evaluate(async () => {
    const fixtureWindow = window as unknown as EditorFixtureWindow;
    return {
      calls: fixtureWindow.__editorLiveFixture.calls,
      foreign: await fixtureWindow.__editorLiveFixture.probeForeign(),
    };
  });
  const publishes = evidence.calls.filter((call) => call.path.endsWith("/publish"));
  expect(publishes).toEqual([
    {
      method: "POST",
      path: "/api/problems/0198e000-0000-7000-8000-000000000002/publish",
      body: '{"scope":"public"}',
      ifMatch: '"2"',
    },
    {
      method: "POST",
      path: "/api/problems/0198e000-0000-7000-8000-000000000002/publish",
      body: '{"scope":"public"}',
      ifMatch: '"3"',
    },
    {
      method: "POST",
      path: "/api/problems/0198e000-0000-7000-8000-000000000002/publish",
      body: '{"scope":"public"}',
      ifMatch: '"6"',
    },
  ]);
  expect(evidence.foreign).toBe("ApiRequestError");
  expect(evidence.calls.some((call) => /preview|key|provider|answer|source/i.test(call.path))).toBe(
    false,
  );
  expect(
    evidence.calls
      .filter((call) => call.path.endsWith("/publication-validation"))
      .every((call) => call.body === null),
  ).toBe(true);
  expect(
    evidence.calls
      .filter((call) => call.path.endsWith("/publish"))
      .every((call) => call.body === '{"scope":"public"}'),
  ).toBe(true);
  const saves = evidence.calls.filter((call) => call.method === "PUT");
  expect(saves.map((call) => call.ifMatch)).toEqual(['"1"', '"2"', '"4"', '"5"']);
  const save = saves[0];
  const savedDraft = JSON.parse(save?.body ?? "null") as Readonly<Record<string, unknown>>;
  expect(Object.keys(savedDraft).sort()).toEqual([
    "attemptPolicy",
    "grading",
    "metadata",
    "prompt",
    "randomization",
    "response",
    "source",
    "timingPolicy",
    "workspace",
  ]);
  expect(nestedKeys(savedDraft).join("\n")).not.toMatch(
    /^(?:answer|answerKey|key|provider|sourceArtifact|problem)$/im,
  );
  expect((savedDraft.metadata as Readonly<Record<string, unknown>>).title).toBe(
    "Edited title survives readiness refusal",
  );
});
