// flat_question_editor.spec.ts - visible acceptance for protected flat-question authoring.
// Selector contract: the native labels and buttons in flat_question_editor_page.tsx and its
// child fields are the instructor's actual keyboard-accessible editing surface.

import { expect, test } from "@playwright/test";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

let fixtureScript = "";

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
        import { FlatQuestionEditorPage } from "./src/features/flat_question_authoring/flat_question_editor_page.tsx";
        import { createFlatQuestionClient } from "./src/features/flat_question_authoring/flat_question_client.ts";
        import { createFlatQuestionRepository } from "./src/features/flat_question_authoring/flat_question_repository.ts";
        import { serializeFlatQuestionSource } from "./src/features/flat_question_authoring/flat_question_codec.ts";

        const workspace = "00000000-0000-4000-8000-000000000010";
        const problem = "00000000-0000-4000-8000-000000000011";
        const version = "00000000-0000-4000-8000-000000000012";
        const calls = [];
        const publicCalls = [];
        let revision = 1;
        let source = {
          format: "pleFlatQuestion", version: 2,
          title: "Favorite color", prompt: "What is my favorite color?",
          response: { kind: "singleChoice", choices: [
            { id: "blue", text: "Blue", feedback: null },
            { id: "red", text: "Red", feedback: null },
          ], correctChoice: "blue" }, feedback: { correct: null, incorrect: null }, points: 1,
          attemptPolicy: { maxAttempts: null, feedback: "immediateFull" },
          timingPolicy: { kind: "untimed" }, tags: ["example"], taxonomy: [],
          license: { kind: "ccBySa" }, language: "en-US",
        };
        let staleNextSave = false;
        let delaySave = false;
        let releaseSave = null;
        let delayValidation = false;
        let releaseValidation = null;
        let dispose = null;
        const json = (value, status = 200, headers = {}) => new Response(JSON.stringify(value), {
          status, headers: { "content-type": "application/json", ...headers },
        });
        const compiled = (candidate, published = false) => ({
          ...(published ? { problem, version } : {}), workspace,
          source: { backend: "native", family: "flat_single_choice_v2" },
          prompt: [{ kind: "text", markdown: candidate.prompt }],
          response: { kind: "multipleChoice", choices: candidate.response.choices.map((choice) => ({
            id: choice.id, body: [{ kind: "text", markdown: choice.text }],
          })), selection: { kind: "exactlyOne" } },
          attemptPolicy: candidate.attemptPolicy, timingPolicy: candidate.timingPolicy,
          randomization: { kind: "static" }, grading: { mode: "allOrNothing", points: candidate.points },
          metadata: { title: candidate.title, tags: candidate.tags, taxonomy: candidate.taxonomy,
            license: candidate.license, language: candidate.language },
        });
        const transport = async (input, init = {}) => {
          const url = new URL(String(input), window.location.origin);
          const method = init.method ?? "GET";
          const body = typeof init.body === "string" ? init.body : null;
          const headers = new Headers(init.headers);
          calls.push({ method, path: url.pathname, body, ifMatch: headers.get("if-match") });
          const sourcePath = "/api/workspaces/" + workspace + "/flat-question";
          if (url.pathname === sourcePath && method === "GET") {
            return new Response(serializeFlatQuestionSource(source), {
              headers: { "content-type": "application/vnd.peptidyle.flat-question+json", etag: '"' + revision + '"' },
            });
          }
          if (url.pathname === sourcePath && method === "PUT") {
            if (staleNextSave) {
              staleNextSave = false;
              source = { ...source, title: "Remote collaborator title" };
              revision += 1;
              return json({ error: "stale" }, 409);
            }
            const candidate = JSON.parse(body);
            const finish = () => {
              source = candidate;
              revision += 1;
              return json(compiled(source), 200, { etag: '"' + revision + '"' });
            };
            if (delaySave) return await new Promise((resolve) => { releaseSave = () => resolve(finish()); });
            return finish();
          }
          if (url.pathname === "/api/problems/" + workspace + "/flat-question-publish" && method === "POST") {
            return json(compiled(source, true));
          }
          return json({ error: "not found" }, 404);
        };
        const client = createFlatQuestionClient({ fetch: transport });
        const repository = createFlatQuestionRepository(client);
        const api = {
          validateWorkspacePublication: async () => {
            publicCalls.push({ kind: "validate", body: null });
            if (delayValidation) await new Promise((resolve) => { releaseValidation = resolve; });
            return { kind: "capabilityReport", revision: '"' + revision + '"', violations: [] };
          },
          getWorkspacePublicationDiff: async () => {
            publicCalls.push({ kind: "diff", body: JSON.stringify({ title: source.title, changed: ["title"] }) });
            return {
              draftRevision: revision, revision: '"' + revision + '"', baseline: "firstPublication", prior: null,
              previous: null, changed: ["title"], current: {
                sourceBackend: "native", title: source.title, prompt: { blocks: ["text"] },
                response: { kind: "multipleChoice", optionCount: source.response.choices.length },
                attemptPolicy: source.attemptPolicy, timingPolicy: source.timingPolicy,
                randomization: { kind: "static" }, metadata: { tags: source.tags, taxonomy: source.taxonomy,
                  license: source.license, language: source.language },
              },
            };
          },
        };
        async function mount() {
          dispose?.();
          document.getElementById("flat-fixture")?.remove();
          const host = document.createElement("div");
          host.id = "flat-fixture";
          document.body.appendChild(host);
          const initial = await repository.load(workspace);
          dispose = render(() => createComponent(FlatQuestionEditorPage, { workspace, initial, repository, api }), host);
        }
        window.__flatQuestionFixture = {
          calls, publicCalls, mount,
          delayNextSave() { delaySave = true; },
          releaseSave() { delaySave = false; releaseSave?.(); releaseSave = null; },
          staleNextSave() { staleNextSave = true; },
          delayValidation() { delayValidation = true; },
          releaseValidation() { delayValidation = false; releaseValidation?.(); releaseValidation = null; },
          source: () => source,
        };
        void mount();
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "flat_question_editor_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles[0];
  if (output === undefined)
    throw new Error("Flat-question editor fixture bundle was not produced.");
  fixtureScript = output.text;
});

async function fixture(page: import("@playwright/test").Page): Promise<void> {
  await page.goto("/");
  await page.addScriptTag({ content: fixtureScript });
  await expect(page.getByRole("heading", { name: "Flat single-choice question" })).toBeVisible();
}

async function fixtureValue<T>(
  page: import("@playwright/test").Page,
  expression: string,
): Promise<T> {
  return await page.evaluate<T>(expression);
}

test("instructor authors, resolves a stale draft, and publishes only the reviewed revision", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("pageerror", (error) => consoleErrors.push(error.message));
  await fixture(page);

  await expect(page.getByLabel("Question title")).toHaveValue("Favorite color");
  await page.getByLabel("Question title").fill("Favorite color revised");
  await page.getByLabel("Learner-facing prompt").fill("Which color do I prefer today?");
  await page.getByRole("button", { name: "Add choice" }).click();
  const choiceText = page.getByLabel("Choice text");
  await choiceText.nth(0).fill("Blue");
  await choiceText.nth(1).fill("Red");
  await choiceText.nth(2).fill("Yellow");
  await page.getByRole("radio", { name: /Mark choice 1 as correct:/u }).check();
  await page
    .getByLabel("Teaching feedback for this choice (optional)")
    .nth(0)
    .fill("Blue is correct.");
  await page
    .getByLabel("Correct-answer feedback (optional)", { exact: true })
    .fill("Exactly right.");
  await page.getByLabel("Incorrect-answer feedback (optional)", { exact: true }).fill("Try again.");

  const beforePreview = await fixtureValue<number>(
    page,
    "window.__flatQuestionFixture.calls.length",
  );
  const preview = page.getByRole("region", { name: "Student preview" });
  await preview.getByLabel(/Blue/).check();
  await expect(preview).not.toContainText("Exactly right.");
  await expect(preview).not.toContainText("correctChoice");
  expect(await fixtureValue<number>(page, "window.__flatQuestionFixture.calls.length")).toBe(
    beforePreview,
  );
  expect((await preview.innerText()).toLowerCase()).not.toMatch(
    /feedback|correctchoice|checksum|base64|private|key/,
  );

  await page.evaluate("window.__flatQuestionFixture.delayNextSave()");
  const save = page.getByRole("button", { name: "Save private draft" });
  await save.click();
  await save.click({ force: true });
  await expect(save).toBeDisabled();
  expect(
    await fixtureValue<number>(
      page,
      "window.__flatQuestionFixture.calls.filter((call) => call.method === 'PUT').length",
    ),
  ).toBe(1);
  await page.evaluate("window.__flatQuestionFixture.releaseSave()");
  await expect(page.getByRole("button", { name: "Check instructor answer" })).toBeEnabled();
  expect(
    await fixtureValue<string>(
      page,
      "window.__flatQuestionFixture.source().response.choices[2].text",
    ),
  ).toBe("Yellow");

  await page.evaluate("window.__flatQuestionFixture.mount()");
  await expect(page.getByLabel("Question title")).toHaveValue("Favorite color revised");
  await expect(page.getByLabel("Learner-facing prompt")).toHaveValue(
    "Which color do I prefer today?",
  );
  await expect(page.getByLabel("Teaching feedback for this choice (optional)").nth(0)).toHaveValue(
    "Blue is correct.",
  );
  await expect(page.getByLabel("Correct-answer feedback (optional)", { exact: true })).toHaveValue(
    "Exactly right.",
  );
  await expect(
    page.getByLabel("Incorrect-answer feedback (optional)", { exact: true }),
  ).toHaveValue("Try again.");

  await page.getByLabel("Question title").fill("My local conflict edit");
  await page.evaluate("window.__flatQuestionFixture.staleNextSave()");
  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(page.getByRole("alert")).toContainText("newer saved draft exists");
  await expect(page.getByLabel("Question title")).toHaveValue("My local conflict edit");
  await page.getByRole("button", { name: "Reload newest draft" }).click();
  await expect(page.getByLabel("Question title")).toHaveValue("Remote collaborator title");

  await page.getByLabel("Question title").fill("Ready after review race");
  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(page.getByRole("button", { name: "Check instructor answer" })).toBeEnabled();
  await page.evaluate("window.__flatQuestionFixture.delayValidation()");
  await page.getByRole("button", { name: "Review publication changes" }).click();
  await page.getByLabel("Question title").fill("Changed during review");
  await page.evaluate("window.__flatQuestionFixture.releaseValidation()");
  await expect(page.getByRole("button", { name: "Confirm and publish" })).toHaveCount(0);
  expect(
    await fixtureValue<number>(
      page,
      "window.__flatQuestionFixture.calls.filter((call) => call.method === 'POST').length",
    ),
  ).toBe(0);

  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(page.getByRole("button", { name: "Check instructor answer" })).toBeEnabled();
  await page.getByRole("button", { name: "Review publication changes" }).click();
  await expect(page.getByRole("button", { name: "Confirm and publish" })).toBeVisible();
  await page.getByLabel("Publication scope").selectOption("public");
  await page.getByRole("button", { name: "Confirm and publish" }).click();
  await expect(page.getByRole("link", { name: "Open published version" })).toHaveAttribute(
    "href",
    "/library/00000000-0000-4000-8000-000000000011/versions/00000000-0000-4000-8000-000000000012",
  );

  const protectedPut = await fixtureValue<{ readonly body: string; readonly ifMatch: string }[]>(
    page,
    "window.__flatQuestionFixture.calls.filter((call) => call.method === 'PUT')",
  );
  const lastProtectedPut = protectedPut[protectedPut.length - 1];
  if (lastProtectedPut === undefined) throw new Error("Expected a protected source save.");
  expect(lastProtectedPut.ifMatch).toMatch(/^"[1-9][0-9]*"$/u);
  expect(lastProtectedPut.body).toContain('"correctChoice"');
  const publish = await fixtureValue<{ readonly body: string; readonly ifMatch: string }[]>(
    page,
    "window.__flatQuestionFixture.calls.filter((call) => call.method === 'POST')",
  );
  expect(publish).toHaveLength(1);
  expect(publish[0]).toMatchObject({
    body: '{"scope":"public"}',
    ifMatch: expect.stringMatching(/^"[1-9][0-9]*"$/u),
  });
  const publicBodies = await fixtureValue<{ readonly body: string | null }[]>(
    page,
    "window.__flatQuestionFixture.publicCalls",
  );
  for (const call of [...publicBodies, ...publish]) {
    expect((call.body ?? "").toLowerCase()).not.toMatch(
      /correctchoice|private|checksum|base64|key|answer/,
    );
  }
  expect(consoleErrors).toEqual([]);
});

test("native controls remain keyboard reachable and narrow viewports do not overflow", async ({
  page,
}) => {
  await page.setViewportSize({ width: 375, height: 800 });
  await fixture(page);
  await page.getByLabel("Question title").focus();
  await page.keyboard.press("Meta+A");
  await page.keyboard.type("Keyboard title");
  await expect(page.getByLabel("Question title")).toHaveValue("Keyboard title");
  const secondCorrectChoice = page.getByRole("radio", { name: /Mark choice 2 as correct:/u });
  await secondCorrectChoice.focus();
  await page.keyboard.press("Space");
  await expect(secondCorrectChoice).toBeChecked();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
});
