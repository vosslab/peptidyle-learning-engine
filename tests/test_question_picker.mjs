import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

import { EMPTY_QUESTION_LIBRARY_BROWSE_QUERY } from "../src/pages/library_page_model.ts";

import {
  MAX_QUESTION_PICKER_SELECTION_CAP,
  QuestionPickerSession,
  inspectQuestionPickerRow,
  moveQuestionPickerSelection,
  questionPickerInspectionView,
  questionPickerSelection,
  toggleQuestionPickerSelection,
} from "../src/features/question_picker/question_picker_model.ts";

function row(displayId, questionTitle = "Question", revisionNumber = 1) {
  return {
    displayId,
    publishedQuestionRevisionTuple: { publishedQuestionId: displayId, revisionNumber },
    questionTitle,
    summary: "Answer-free summary.",
    bloom: {
      cognitiveProcess: "Understand",
      knowledgeDimension: "Conceptual Knowledge",
      classificationEditNumber: "1",
    },
    disciplineName: "Biology",
    disciplineIsRetired: false,
    questionFormat: "pleQuestionJson",
    authorNames: ["Published author"],
    capabilities: [],
    questionLicense: "CC-BY-4.0",
    evidence: { state: "unavailable" },
  };
}

function questionIdFor(index) {
  const alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
  const characters = Array.from({ length: 7 }, () => "0");
  let remaining = index;
  for (let position = characters.length - 1; position >= 0; position -= 1) {
    const digit = remaining % alphabet.length;
    characters[position] = alphabet[digit];
    remaining = Math.floor(remaining / alphabet.length);
  }
  const identifier = characters.join("");
  const checksum = alphabet[createHash("sha256").update(identifier, "ascii").digest()[0] >>> 3];
  return `${identifier.slice(0, 4)}-${checksum}${identifier.slice(4)}`;
}

test("Question Picker preserves public Question ID order and safe row metadata", () => {
  const selection = questionPickerSelection("many", 200, [
    row("7K3M-79QP", "First"),
    row("2R5X-E7YA", "Second"),
  ]);
  assert.deepEqual(selection.questionIds, ["7K3M-79QP", "2R5X-E7YA"]);
  assert.equal(selection.questions[1]?.row.questionTitle, "Second");
  assert.deepEqual(selection.questions[1]?.row.publishedQuestionRevisionTuple, {
    publishedQuestionId: "2R5X-E7YA",
    revisionNumber: 1,
  });
});

test("single-selection mode replaces the prior result", () => {
  const initial = questionPickerSelection("one", 1, [row("7K3M-79QP")]);
  const next = toggleQuestionPickerSelection("one", 1, initial, row("2R5X-E7YA"), true);
  assert.deepEqual(next.questionIds, ["2R5X-E7YA"]);
});

test("picker removes a selected row and preserves the other selected order", () => {
  const initial = questionPickerSelection("many", 200, [row("7K3M-79QP"), row("2R5X-E7YA")]);
  const next = toggleQuestionPickerSelection("many", 200, initial, row("7K3M-79QP"), false);
  assert.deepEqual(next.questionIds, ["2R5X-E7YA"]);
});

test("picker reorders the selected tray without changing membership", () => {
  const initial = questionPickerSelection("many", 200, [row("7K3M-79QP"), row("2R5X-E7YA")]);
  const next = moveQuestionPickerSelection("many", 200, initial, 1, -1);
  assert.deepEqual(next.questionIds, ["2R5X-E7YA", "7K3M-79QP"]);
});

test("picker enforces the shared bounded selection limit", () => {
  const rows = Array.from({ length: MAX_QUESTION_PICKER_SELECTION_CAP + 1 }, (_value, index) => {
    return row(questionIdFor(index));
  });
  assert.throws(
    () => questionPickerSelection("many", MAX_QUESTION_PICKER_SELECTION_CAP, rows),
    /at most/,
  );
});

test("picker session drops a stale source response before publishing it", async () => {
  let resolveQuestionLibrary;
  const questionLibrary = new Promise((resolve) => {
    resolveQuestionLibrary = resolve;
  });
  const states = [];
  const session = new QuestionPickerSession(
    {
      search: async (request) => {
        if (request.source.kind === "library") return await questionLibrary;
        return {
          items: [row("2R5X-E7YA", "Mine")],
          aggregates: [],
          nextCursor: null,
          facetTruncation: noTruncation(),
        };
      },
    },
    (state) => states.push(state),
  );
  const first = session.reset({ kind: "library", label: "Question Library" }, { ...emptyQuery() });
  const second = session.reset({ kind: "mine", label: "My questions" }, { ...emptyQuery() });
  resolveQuestionLibrary({
    items: [row("7K3M-79QP", "Stale")],
    aggregates: [],
    nextCursor: null,
    facetTruncation: noTruncation(),
  });
  await Promise.all([first, second]);
  assert.equal(states.at(-1)?.kind, "ready");
  assert.equal(states.at(-1)?.rows[0]?.questionTitle, "Mine");
});

test("picker excludes deferred iMathAS rows from new selections", async () => {
  const states = [];
  const deferred = row("7K3M-79QP", "Deferred backend");
  deferred.questionFormat = "imathas";
  const session = new QuestionPickerSession(
    {
      search: async () => ({
        items: [deferred],
        aggregates: [],
        nextCursor: null,
        facetTruncation: noTruncation(),
      }),
    },
    (state) => states.push(state),
  );

  await session.reset({ kind: "library", label: "Question Library" }, emptyQuery());
  assert.equal(states.at(-1)?.kind, "empty");
});

test("picker selection remains ordered while a source and query change", async () => {
  const selection = questionPickerSelection("many", 200, [
    row("7K3M-79QP", "Preserved first"),
    row("2R5X-E7YA", "Preserved second"),
  ]);
  const session = new QuestionPickerSession(
    {
      search: async (request) => ({
        items: [row(request.source.kind === "library" ? "3S8B-24DZ" : "4T9C-C5EW")],
        aggregates: [],
        nextCursor: null,
        facetTruncation: noTruncation(),
      }),
    },
    () => undefined,
  );
  await session.reset(
    { kind: "library", label: "Question Library" },
    { ...emptyQuery(), search: "first" },
  );
  await session.reset(
    { kind: "mine", label: "My questions" },
    { ...emptyQuery(), search: "second" },
  );
  assert.deepEqual(selection.questionIds, ["7K3M-79QP", "2R5X-E7YA"]);
});

test("pagination failure retains loaded rows while external selection remains usable", async () => {
  const states = [];
  const selection = questionPickerSelection("many", 200, [row("7K3M-79QP")]);
  const session = new QuestionPickerSession(
    {
      search: async (request) => {
        if (request.cursor === null) {
          return {
            items: [row("2R5X-E7YA")],
            aggregates: [],
            nextCursor: "next",
            facetTruncation: noTruncation(),
          };
        }
        throw new Error("temporary source failure");
      },
    },
    (state) => states.push(state),
  );
  await session.reset({ kind: "library", label: "Question Library" }, emptyQuery());
  await session.loadNext();
  assert.equal(states.at(-1)?.kind, "error");
  assert.equal(states.at(-1)?.rows[0]?.displayId, "2R5X-E7YA");
  assert.deepEqual(selection.questionIds, ["7K3M-79QP"]);
});

test("picker replaces one discovery page and forwards the Library page size", async () => {
  const requests = [];
  const states = [];
  const session = new QuestionPickerSession(
    {
      search: async (request) => {
        requests.push({ cursor: request.cursor, pageSize: request.pageSize });
        if (request.pageSize === 100) {
          return {
            items: [row("7K3M-79QP", "Sized")],
            aggregates: [],
            nextCursor: null,
            facetTruncation: noTruncation(),
          };
        }
        if (request.cursor === null) {
          return {
            items: [row("2R5X-E7YA", "First page"), row("3S8B-24DZ", "Also first")],
            aggregates: [],
            nextCursor: "next",
            facetTruncation: noTruncation(),
          };
        }
        return {
          items: [row("4T9C-C5EW", "Second page")],
          aggregates: [],
          nextCursor: null,
          facetTruncation: noTruncation(),
        };
      },
    },
    (state) => states.push(state),
  );
  await session.reset({ kind: "library", label: "Question Library" }, emptyQuery());
  await session.loadNext();
  assert.equal(states.at(-1)?.rows.length, 1);
  assert.equal(states.at(-1)?.rows[0]?.displayId, "4T9C-C5EW");
  assert.equal(session.hasPrevious, true);
  await session.loadPrevious();
  assert.equal(states.at(-1)?.rows.length, 2);
  assert.equal(session.hasPrevious, false);
  await session.changePageSize(100);
  assert.deepEqual(requests.at(-1), { cursor: null, pageSize: 100 });
  assert.equal(states.at(-1)?.rows[0]?.displayId, "7K3M-79QP");
  assert.equal(session.pageSize, 100);
});

function inspectionDetails() {
  return {
    summary: {
      questionId: "2R5X-E7YA",
      backend: "ple",
      metadata: { questionTitle: "Enzyme kinetics" },
      publishedQuestionRevisionTuple: { publishedQuestionId: "2R5X-E7YA", revisionNumber: 4 },
    },
    prompt: { kind: "static", blocks: [{ kind: "text", markdown: "Enzyme active site" }] },
    responsePreview: { kind: "shortText" },
    source: { answer: "concealed" },
  };
}

async function loadQuestionPickerInspectionRenderer() {
  const result = await build({
    bundle: true,
    stdin: {
      contents: `
        import { createComponent } from "solid-js";
        import { renderToString } from "solid-js/web";
        import { QuestionPickerInspection } from "./src/features/question_picker/question_picker_inspection.tsx";
        export function renderQuestionPickerInspection(view, previewDocumentUrl, questionImageUrl) {
          return renderToString(() =>
            createComponent(QuestionPickerInspection, {
              view,
              previewDocumentUrl,
              questionImageUrl,
              onClose() {},
            }),
          );
        }
      `,
      resolveDir: new URL("..", import.meta.url).pathname,
      sourcefile: "question_picker_inspection_ssr.js",
      loader: "js",
    },
    format: "esm",
    outfile: "question_picker_inspection_ssr.js",
    platform: "node",
    plugins: [
      {
        name: "css-stub",
        setup(pluginBuild) {
          pluginBuild.onResolve({ filter: /\.css$/ }, (args) => ({
            path: args.path,
            namespace: "css-stub",
          }));
          pluginBuild.onLoad({ filter: /.*/, namespace: "css-stub" }, () => ({
            contents: "export default {};",
            loader: "js",
          }));
        },
      },
      solidPlugin({ solid: { generate: "ssr", hydratable: false } }),
    ],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined) {
    throw new Error("Question inspection bundle is missing JavaScript.");
  }
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module.renderQuestionPickerInspection !== "function") {
    throw new Error("Question inspection bundle does not export renderQuestionPickerInspection.");
  }
  return module.renderQuestionPickerInspection;
}

test("inspectQuestionPickerRow keeps the current selection and QuestionPickerInspection shows the prompt without adding the Question", async () => {
  const selected = row("7K3M-79QP", "Pinned", 2);
  const selection = questionPickerSelection("many", 200, [selected]);
  const candidate = row("2R5X-E7YA", "Enzyme kinetics", 4);
  const inspected = inspectQuestionPickerRow(selection, candidate);
  assert.equal(inspected.selection, selection);
  assert.deepEqual(
    inspected.publishedQuestionRevisionTuple,
    candidate.publishedQuestionRevisionTuple,
  );
  assert.deepEqual(selection.questionIds, ["7K3M-79QP"]);
  const repeated = inspectQuestionPickerRow(selection, selected);
  assert.equal(repeated.selection, selection);
  assert.deepEqual(selection.questionIds, ["7K3M-79QP"]);

  const details = inspectionDetails();
  const view = questionPickerInspectionView(details);
  assert.equal(view.questionTitle, "Enzyme kinetics");
  assert.equal(view.questionId, "2R5X-E7YA");
  assert.equal(view.backend, "ple");
  assert.equal(view.prompt.blocks[0].markdown, "Enzyme active site");
  assert.deepEqual(view.responsePreview, { kind: "shortText" });
  assert.equal(Object.hasOwn(view, "source"), false);
  assert.deepEqual(
    view.publishedQuestionRevisionTuple,
    details.summary.publishedQuestionRevisionTuple,
  );

  const renderQuestionPickerInspection = await loadQuestionPickerInspectionRenderer();
  const imageUrl = () => new URL("https://ple.invalid/questions/2R5X-E7YA/revisions/4/images/site");
  const html = renderQuestionPickerInspection(view, null, imageUrl);
  assert.match(html, /Inspect before adding/);
  assert.match(html, /Enzyme active site/);
  assert.match(html, /This inspection does not add the Question/);
  assert.match(html, /Text response/);
  assert.doesNotMatch(html, /Add selected Questions/);
  assert.doesNotMatch(html, /Add pinned Question/);
  assert.doesNotMatch(html, /concealed/);

  const webworkView = questionPickerInspectionView({
    ...details,
    summary: { ...details.summary, backend: "webwork" },
    responsePreview: null,
  });
  const previewDocumentUrl = "https://ple.invalid/questions/2R5X-E7YA/revisions/4/preview-document";
  const webworkHtml = renderQuestionPickerInspection(webworkView, previewDocumentUrl, imageUrl);
  assert.match(webworkHtml, /This inspection does not add the Question/);
  assert.equal(webworkHtml.includes(previewDocumentUrl), true);
  assert.doesNotMatch(webworkHtml, /Enzyme active site/);
  assert.doesNotMatch(webworkHtml, /Add selected Questions/);
  assert.doesNotMatch(webworkHtml, /concealed/);
});

function emptyQuery() {
  return {
    ...EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
    search: "",
    authorName: null,
    backend: null,
    tag: null,
    subjects: [],
    topics: [],
    questionType: null,
    capability: null,
    questionLicense: null,
    usedInMyCourses: null,
    authorship: "any",
  };
}

function noTruncation() {
  return { authorNames: false, tags: false, subjects: false, topics: false };
}
