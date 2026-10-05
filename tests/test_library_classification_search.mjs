// Stable search identity contracts. Failure means a selected filter was lost or broadened.
import assert from "node:assert/strict";
import test from "node:test";
import {
  libraryClassificationChange,
  libraryClassificationFilter,
} from "../src/api/library_classification_filter.ts";
import {
  createQuestionLibraryRepository,
  questionSearchRequest,
} from "../src/api/question_library_repository.ts";
import { questionPoolLibraryFilter } from "../src/api/question_pool_library_filter.ts";
import { questionSearchPath } from "../src/api/question_search_query.ts";
import {
  BLOOM_COGNITIVE_PROCESSES,
  BLOOM_KNOWLEDGE_DIMENSIONS,
} from "../src/api/decoders/bloom_classification.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { createQuestionPoolLibraryClient } from "../src/api/http_client/question_pool_library.ts";
import { DecodeError } from "../src/api/decoder.ts";
import { decodeQuestionBulkMetadataCurrent } from "../src/api/decoders/question_bulk_metadata.ts";
import { createQuestionBulkMetadataClient } from "../src/api/http_client/question_bulk_metadata.ts";
import { decodeQuestionPoolMetadata } from "../src/api/decoders/question_pool_library.ts";
import { decodeQuestionSearchFacets } from "../src/api/decoders/question_type_facets.ts";
import { decodeQuestionSearchPage } from "../src/api/decoders/question_library.ts";
import { fastUiQuestionLibraryPage } from "./support/fast_ui_question_library_fixture.ts";
import {
  recoverLibrarySearch,
  searchHandoffQuery,
  searchWithinResultsPath,
} from "../src/pages/library_search_parameters.ts";
import {
  EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  NO_QUESTION_LIBRARY_FACET_TRUNCATION,
  QuestionLibraryBrowseSession,
  decodeQuestionLibraryBrowsePage,
  normalizeQuestionLibraryBrowseQuery,
} from "../src/pages/library_page_model.ts";
import {
  FIRST_MY_QUESTIONS_POSITION,
  loadMyQuestions,
  myQuestionsNextPosition,
  myQuestionsPageSizePosition,
  myQuestionsPreviousPosition,
} from "../src/pages/my_questions_model.ts";
import { FAST_UI_QUESTION_LIBRARY_IDS } from "./support/fast_ui_question_library_fixture.ts";

const identities = {
  discipline_uuid: "00000000-0000-0000-0000-000000000001",
  subject_uuid: "00000000-0000-0000-0000-000000000002",
  topic_uuid: "00000000-0000-0000-0000-000000000003",
  subtopic_uuid: "00000000-0000-0000-0000-000000000004",
  cross_discipline: true,
};
function query() {
  return {
    ...EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
    ...identities,
    search: 'discipline:biology topic:"cell division" -mitosis',
    tag: "review",
    subjects: ["biochemistry"],
    topics: ["inheritance"],
    bloomCognitiveProcess: "Analyze",
    bloomKnowledgeDimension: "Procedural Knowledge",
    sort: "publishedNewest",
  };
}
function row() {
  return {
    displayId: "7K3M-79QP",
    publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 1 },
    questionTitle: "Cell division",
    summary: "Answer-free summary",
    bloom: {
      cognitiveProcess: "Understand",
      knowledgeDimension: "Conceptual Knowledge",
      classificationEditNumber: "1",
    },
    disciplineName: "Biology",
    disciplineIsRetired: false,
    questionFormat: "pleQuestionJson",
    authorNames: [],
    capabilities: [],
    questionLicense: "CC-BY-4.0",
    evidence: { state: "unavailable" },
  };
}

test("Library Objects share the Discipline Subject Topic Subtopic and Tag vocabulary", () => {
  const request = questionSearchRequest(query(), null);
  assert.equal(request.discipline_uuid, identities.discipline_uuid);
  assert.equal(request.subject_uuid, identities.subject_uuid);
  assert.equal(request.topic_uuid, identities.topic_uuid);
  assert.equal(request.subtopic_uuid, identities.subtopic_uuid);
  assert.deepEqual(request.tags, ["review"]);
});

test("Library Objects use shared metadata for organization, search, filtering, and discovery", () => {
  const browse = query();
  const question = questionSearchRequest(browse, null);
  assert.equal(question.text, browse.search);
  assert.equal(question.discipline_uuid, identities.discipline_uuid);
  assert.equal(question.subject_uuid, identities.subject_uuid);
  assert.equal(question.topic_uuid, identities.topic_uuid);
  assert.equal(question.subtopic_uuid, identities.subtopic_uuid);
  assert.deepEqual(question.tags, ["review"]);
  const pool = questionPoolLibraryFilter({
    discipline_uuid: identities.discipline_uuid,
    subject_uuid: identities.subject_uuid,
    topic_uuid: identities.topic_uuid,
    subtopic_uuid: identities.subtopic_uuid,
    cross_discipline: false,
    text: "Pedigree Review",
    tags: ["Review"],
  });
  assert.equal(pool.text, "pedigree review");
  assert.equal(pool.discipline_uuid, identities.discipline_uuid);
  assert.equal(pool.subject_uuid, identities.subject_uuid);
  assert.equal(pool.topic_uuid, identities.topic_uuid);
  assert.equal(pool.subtopic_uuid, identities.subtopic_uuid);
  assert.deepEqual(pool.tags, ["review"]);
  assert.throws(
    () =>
      questionPoolLibraryFilter({
        discipline_uuid: null,
        subject_uuid: identities.subject_uuid,
        topic_uuid: null,
        subtopic_uuid: null,
        cross_discipline: false,
      }),
    /Library classification requires its selected parents/,
  );
});

test("The Question Library is one global collection of Published Questions and Question Pools", async () => {
  const requests = [];
  const questionPage = fastUiQuestionLibraryPage();
  const poolPage = {
    items: [],
    nextCursor: null,
    bloomFacets: {
      cognitiveProcesses: BLOOM_COGNITIVE_PROCESSES.map((cognitiveProcess) => ({
        cognitiveProcess,
        count: 0,
      })),
      knowledgeDimensions: BLOOM_KNOWLEDGE_DIMENSIONS.map((knowledgeDimension) => ({
        knowledgeDimension,
        count: 0,
      })),
    },
  };
  const fetchImplementation = async (input) => {
    const url = new URL(String(input), "https://example.test");
    requests.push(url);
    const body = url.pathname === "/api/question-pools" ? poolPage : questionPage;
    return new Response(JSON.stringify(body), {
      headers: { "content-type": "application/json", "cache-control": "no-store" },
    });
  };
  const repository = createQuestionLibraryRepository(
    createHttpApiClient({ fetch: fetchImplementation }),
  );
  const pools = createQuestionPoolLibraryClient({ fetch: fetchImplementation });
  const questions = await repository.search(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY, null);
  const poolsFound = await pools.listQuestionPools();
  assert.deepEqual(
    requests.map((url) => url.pathname),
    ["/api/questions/search", "/api/question-pools"],
  );
  for (const url of requests) {
    assert.equal(url.pathname.includes("course"), false);
    for (const key of url.searchParams.keys()) assert.equal(/course/iu.test(key), false);
  }
  assert.equal(requests[0].searchParams.has("used_in_my_courses"), false);
  assert.equal(requests[0].searchParams.get("authorship"), "any");
  assert.equal(questions.items.length, questionPage.items.length);
  assert.equal(questions.items[0].displayId, questionPage.items[0].summary.questionId);
  assert.equal(poolsFound.items.length, 0);
  assert.equal(poolsFound.bloomFacets.cognitiveProcesses.length, BLOOM_COGNITIVE_PROCESSES.length);
});

test("Library metadata describes the Library Object rather than a Course Assessment or textbook", () => {
  const questionMetadata = {
    questionId: "7K3M-79QP",
    metadataEditNumber: 1,
    tags: ["review"],
    disciplineUuid: identities.discipline_uuid,
    subjectUuid: identities.subject_uuid,
    topicUuid: identities.topic_uuid,
    subtopicUuid: identities.subtopic_uuid,
  };
  const [decoded] = decodeQuestionBulkMetadataCurrent({ items: [questionMetadata] });
  assert.equal(decoded.questionId, questionMetadata.questionId);
  assert.equal(decoded.metadataEditNumber, questionMetadata.metadataEditNumber);
  assert.deepEqual(decoded.tags, questionMetadata.tags);
  assert.equal(decoded.disciplineUuid, identities.discipline_uuid);
  assert.equal(decoded.subjectUuid, identities.subject_uuid);
  assert.equal(decoded.topicUuid, identities.topic_uuid);
  assert.equal(decoded.subtopicUuid, identities.subtopic_uuid);
  const poolMetadata = {
    title: "Inheritance reasoning",
    description: "Interpret interchangeable pedigrees.",
    disciplineUuid: identities.discipline_uuid,
    disciplineName: "Biology",
    disciplineIsRetired: false,
    subjectUuid: identities.subject_uuid,
    topicUuid: identities.topic_uuid,
    subtopicUuid: identities.subtopic_uuid,
    tags: ["review"],
  };
  const pool = decodeQuestionPoolMetadata(poolMetadata, "metadata");
  assert.equal(pool.title, poolMetadata.title);
  assert.equal(pool.description, poolMetadata.description);
  assert.deepEqual(pool.tags, poolMetadata.tags);
  assert.equal(pool.disciplineUuid, identities.discipline_uuid);
  assert.equal(pool.subjectUuid, identities.subject_uuid);
  for (const location of ["courseInstanceId", "assessmentId", "textbook"]) {
    assert.equal(Object.hasOwn(decoded, location), false);
    assert.equal(Object.hasOwn(pool, location), false);
    assert.throws(
      () =>
        decodeQuestionBulkMetadataCurrent({
          items: [{ ...questionMetadata, [location]: "location" }],
        }),
      DecodeError,
    );
    assert.throws(
      () => decodeQuestionPoolMetadata({ ...poolMetadata, [location]: "location" }, "metadata"),
      DecodeError,
    );
  }
});

test("search metadata updates Title, Description, and classification without a Question Revision", async () => {
  const bodies = [];
  let fetched;
  const client = createQuestionBulkMetadataClient(async (_input, init) => {
    fetched = true;
    const body = JSON.parse(init.body);
    bodies.push(body);
    return new Response(
      JSON.stringify({
        results: body.selection.map((item) => ({
          questionId: item.questionId,
          metadataEditNumber: item.metadataEditNumber + 1,
        })),
      }),
      {
        status: 200,
        headers: { "content-type": "application/json", "cache-control": "no-store" },
      },
    );
  }, "");
  const shared = {
    tags: ["review"],
    disciplineUuid: identities.discipline_uuid,
    subjectUuid: identities.subject_uuid,
    topicUuid: identities.topic_uuid,
    subtopicUuid: identities.subtopic_uuid,
  };
  const selection = [
    {
      questionId: "2R5X-E7YA",
      metadataEditNumber: 1,
      questionTitle: "Pedigree",
      questionDescription: "Read the first pedigree.",
    },
    {
      questionId: "7K3M-79QP",
      metadataEditNumber: 3,
      questionTitle: "Second pedigree",
      questionDescription: "Read the second pedigree.",
    },
  ];
  const results = await client.updateQuestionBulkMetadata({ selection, patch: shared });
  assert.deepEqual(bodies[0].selection, selection);
  assert.deepEqual(bodies[0].patch, shared);
  assert.equal(Object.hasOwn(bodies[0], "revisionNumber"), false);
  assert.equal(
    bodies[0].selection.some((item) => Object.hasOwn(item, "revisionNumber")),
    false,
  );
  assert.notEqual(bodies[0].selection[0].questionTitle, bodies[0].selection[1].questionTitle);
  assert.deepEqual(results, [
    { questionId: "2R5X-E7YA", metadataEditNumber: 2 },
    { questionId: "7K3M-79QP", metadataEditNumber: 4 },
  ]);
  fetched = false;
  await assert.rejects(
    client.updateQuestionBulkMetadata({
      selection: [{ questionId: "2R5X-E7YA", metadataEditNumber: 1, questionTitle: " " }],
      patch: {},
    }),
    DecodeError,
  );
  assert.equal(fetched, false);
});

test("Library URL handoff and strict wire request retain hierarchy, filters, and sort", () => {
  const original = query();
  const restored = searchHandoffQuery(
    new URL(searchWithinResultsPath(original), "https://example.test").search,
  );
  assert.deepEqual(restored, original);
  assert.deepEqual(
    libraryClassificationFilter(normalizeQuestionLibraryBrowseQuery(restored)),
    identities,
  );
  const parameters = new URL(
    questionSearchPath(questionSearchRequest(restored, "next-page")),
    "https://example.test",
  ).searchParams;
  for (const [field, value] of Object.entries(identities))
    assert.equal(parameters.get(field), String(value));
  assert.equal(parameters.get("text"), original.search);
  assert.equal(parameters.get("tags"), "review");
  assert.equal(parameters.get("subjects"), "biochemistry");
  assert.equal(parameters.get("bloom_cognitive_process"), "Analyze");
  assert.equal(parameters.get("bloom_knowledge_dimension"), "Procedural Knowledge");
  assert.equal(parameters.get("sort"), "publishedNewest");
  assert.equal(parameters.get("cursor"), "next-page");
  const empty = new URL(
    questionSearchPath(questionSearchRequest(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY, null)),
    "https://example.test",
  ).searchParams;
  for (const field of Object.keys(identities)) assert.equal(empty.has(field), false);
  assert.equal(empty.get("sort"), "titleAscending");
});

test("Library cascade clears descendants and cross mode without changing independent filters", () => {
  const original = query();
  const discipline = { ...original, ...libraryClassificationChange("discipline_uuid", null) };
  assert.deepEqual(
    libraryClassificationFilter(discipline),
    libraryClassificationFilter(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY),
  );
  const subject = { ...original, ...libraryClassificationChange("subject_uuid", null) };
  assert.equal(subject.topic_uuid, null);
  assert.equal(subject.subtopic_uuid, null);
  assert.equal(subject.cross_discipline, false);
  const topic = { ...original, ...libraryClassificationChange("topic_uuid", null) };
  assert.equal(topic.subtopic_uuid, null);
  assert.equal(topic.subject_uuid, original.subject_uuid);
  assert.equal(topic.cross_discipline, true);
  assert.equal(discipline.search, original.search);
  assert.equal(discipline.tag, original.tag);
});

test("Library request rejects malformed identities, incomplete chains, false booleans and unknown fields", () => {
  const request = questionSearchRequest(query(), null);
  for (const change of [
    { discipline_uuid: "biology" },
    { discipline_uuid: null },
    { subject_uuid: null },
    { topic_uuid: null },
    { cross_discipline: "false" },
    { hidden: true },
    { backends: ["imathas"] },
    { bloom_cognitive_process: "analyze" },
    { bloom_knowledge_dimension: "Procedural" },
  ]) {
    assert.throws(() => questionSearchPath({ ...request, ...change }));
  }
  assert.throws(() => questionSearchPath({ ...request, sort: "unknown" }));
  assert.equal(
    new URL(
      questionSearchPath({ ...request, page_size: 250 }),
      "https://example.test",
    ).searchParams.get("page_size"),
    "250",
  );
  assert.throws(() => questionSearchPath({ ...request, page_size: 251 }));
  assert.throws(() => questionSearchPath({ ...request, page_size: 0 }));
  assert.throws(() => searchHandoffQuery("?cross_discipline=1"));
  assert.throws(() => searchHandoffQuery("?sort=unknown"));
  assert.throws(() => searchHandoffQuery("?sort=titleAscending&sort=publishedNewest"));
  assert.throws(() => searchHandoffQuery("?bloomCognitiveProcess=analyze"));
  assert.throws(() => searchHandoffQuery("?bloomCognitiveProcess="));
  assert.throws(() =>
    searchHandoffQuery(
      "?bloomKnowledgeDimension=Factual+Knowledge&bloomKnowledgeDimension=Procedural+Knowledge",
    ),
  );
});

test("Question discovery rejects zero page size before dispatch", async () => {
  let dispatched = false;
  const client = createHttpApiClient({
    fetch: async () => {
      dispatched = true;
      throw new Error("invalid page size must not dispatch");
    },
  });
  assert.throws(() =>
    client.searchQuestionLibrary({ ...questionSearchRequest(query(), null), page_size: 0 }),
  );
  assert.equal(dispatched, false);
});

test("Library malformed URL recovery removes only the rejected strict options", () => {
  const invalidSort = new URL(searchWithinResultsPath(query()), "https://example.test");
  invalidSort.searchParams.append("sort", "titleAscending");
  const recoveredSort = searchHandoffQuery(recoverLibrarySearch(invalidSort.search));
  assert.equal(recoveredSort.sort, "titleAscending");
  assert.equal(recoveredSort.tag, "review");
  assert.deepEqual(libraryClassificationFilter(recoveredSort), identities);

  const recoveredClassification = searchHandoffQuery(
    recoverLibrarySearch("?subject_uuid=bad&sort=publishedNewest&tag=review"),
  );
  assert.equal(recoveredClassification.sort, "publishedNewest");
  assert.equal(recoveredClassification.tag, "review");
  assert.deepEqual(
    libraryClassificationFilter(recoveredClassification),
    libraryClassificationFilter(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY),
  );

  const recoveredBloom = searchHandoffQuery(
    recoverLibrarySearch("?bloomCognitiveProcess=analyze&tag=review"),
  );
  assert.equal(recoveredBloom.bloomCognitiveProcess, null);
  assert.equal(recoveredBloom.tag, "review");
});

test("Library continuation and Retry preserve identity and sort", async () => {
  const requests = [];
  let failNext = true;
  const session = new QuestionLibraryBrowseSession(
    {
      search: async (value, cursor) => {
        requests.push({ value, cursor });
        if (cursor !== null && failNext) {
          failNext = false;
          throw new Error("temporary failure");
        }
        return {
          items: [row()],
          nextCursor: cursor === null ? "next-page" : null,
          aggregates: [],
          facetTruncation: NO_QUESTION_LIBRARY_FACET_TRUNCATION,
        };
      },
    },
    () => {},
  );
  await session.reset(query());
  await session.loadNext();
  assert.equal(session.state.kind, "error");
  await session.retry();
  assert.equal(session.state.kind, "ready");
  assert.deepEqual(
    requests.map((request) => libraryClassificationFilter(request.value)),
    [identities, identities, identities],
  );
  assert.deepEqual(
    requests.map((request) => request.cursor),
    [null, "next-page", "next-page"],
  );
  assert.deepEqual(
    requests.map((request) => request.value.sort),
    ["publishedNewest", "publishedNewest", "publishedNewest"],
  );
  assert.deepEqual(
    requests.map((request) => [
      request.value.bloomCognitiveProcess,
      request.value.bloomKnowledgeDimension,
    ]),
    [
      ["Analyze", "Procedural Knowledge"],
      ["Analyze", "Procedural Knowledge"],
      ["Analyze", "Procedural Knowledge"],
    ],
  );
});

test("Bloom facet decoder requires all guide values in guide order, including zeros", () => {
  const facets = {
    authorNames: [],
    authorNamesTruncated: false,
    backends: [],
    tags: [],
    tagsTruncated: false,
    subjects: [],
    subjectsTruncated: false,
    topics: [],
    topicsTruncated: false,
    questionTypes: [],
    capabilities: [],
    questionLicenses: [],
    usedInMyCourses: { used: 0 },
    bloomCognitiveProcesses: [
      "Remember",
      "Understand",
      "Apply",
      "Analyze",
      "Evaluate",
      "Create",
    ].map((cognitiveProcess, index) => ({ cognitiveProcess, count: index })),
    bloomKnowledgeDimensions: [
      "Factual Knowledge",
      "Conceptual Knowledge",
      "Procedural Knowledge",
      "Metacognitive Knowledge",
    ].map((knowledgeDimension) => ({ knowledgeDimension, count: 0 })),
  };
  assert.deepEqual(decodeQuestionSearchFacets(facets, "facets"), facets);
  assert.throws(() =>
    decodeQuestionSearchFacets(
      { ...facets, bloomCognitiveProcesses: facets.bloomCognitiveProcesses.toReversed() },
      "facets",
    ),
  );
  assert.throws(() =>
    decodeQuestionSearchFacets(
      { ...facets, bloomKnowledgeDimensions: facets.bloomKnowledgeDimensions.slice(1) },
      "facets",
    ),
  );
});

test("Question discovery decoder accepts 250 rows and rejects 251", () => {
  const page = fastUiQuestionLibraryPage();
  const items = Array.from({ length: 250 }, () => page.items[0]);
  assert.equal(decodeQuestionSearchPage({ ...page, items }).items.length, 250);
  assert.throws(() => decodeQuestionSearchPage({ ...page, items: [...items, page.items[0]] }));
});

test("Library browse decoder accepts 250 rows and rejects 251", () => {
  const items = Array.from({ length: 250 }, row);
  const page = {
    items,
    nextCursor: null,
    aggregates: [],
    facetTruncation: NO_QUESTION_LIBRARY_FACET_TRUNCATION,
  };
  assert.equal(decodeQuestionLibraryBrowsePage(page).items.length, 250);
  assert.throws(() => decodeQuestionLibraryBrowsePage({ ...page, items: [...items, row()] }));
});

test("My Questions continues the authored library page", async () => {
  const requests = [];
  const client = {
    async searchQuestionLibrary(request) {
      requests.push(request);
      const page = fastUiQuestionLibraryPage();
      return {
        ...page,
        items: page.items.slice(0, 1),
        nextCursor: request.cursor === null ? "next-page" : null,
      };
    },
  };
  const first = await loadMyQuestions(client, { cursor: null, pageSize: 50 });
  assert.equal(first.items[0].displayId, FAST_UI_QUESTION_LIBRARY_IDS[0]);
  assert.equal(first.nextCursor, "next-page");
  const next = myQuestionsNextPosition(FIRST_MY_QUESTIONS_POSITION, first.nextCursor);
  const second = await loadMyQuestions(client, { cursor: next.inputCursor, pageSize: 100 });
  assert.equal(second.nextCursor, null);
  assert.equal(myQuestionsPreviousPosition(next).inputCursor, null);
  assert.equal(myQuestionsPreviousPosition(FIRST_MY_QUESTIONS_POSITION), null);
  const sized = myQuestionsPageSizePosition(250);
  await loadMyQuestions(client, { cursor: sized.inputCursor, pageSize: sized.pageSize });
  assert.deepEqual(
    requests.map((request) => [request.authorship, request.cursor, request.page_size]),
    [
      ["authoredByCurrentAccount", null, 50],
      ["authoredByCurrentAccount", "next-page", 100],
      ["authoredByCurrentAccount", null, 250],
    ],
  );

  const { build } = await import("esbuild");
  const { solidPlugin } = await import("esbuild-plugin-solid");
  const bundled = await build({
    bundle: true,
    stdin: {
      contents: `
        import { createComponent, Suspense } from "solid-js";
        import { renderToStringAsync } from "solid-js/web";
        import { ApplicationApiProvider } from "./src/api/application_api.tsx";
        import { RouteScopeProvider } from "./src/ribbon/route_scope_context.tsx";
        import { MyQuestionsPage } from "./src/pages/my_questions_page.tsx";
        export function renderMyQuestions(applicationApi) {
          return renderToStringAsync(() =>
            createComponent(Suspense, {
              get children() {
                return createComponent(ApplicationApiProvider, {
                  applicationApi,
                  get children() {
                    return createComponent(RouteScopeProvider, {
                      pathname: "/authoring/questions",
                      get children() {
                        return createComponent(MyQuestionsPage, {});
                      },
                    });
                  },
                });
              },
            }),
          );
        }
      `,
      resolveDir: new URL("..", import.meta.url).pathname,
      sourcefile: "my_questions_ssr.js",
      loader: "js",
    },
    format: "esm",
    outfile: "my_questions_ssr.js",
    platform: "node",
    write: false,
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
  });
  const javascript = bundled.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined) throw new Error("My Questions SSR bundle is missing JavaScript.");
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const rendered = await import(`data:text/javascript;base64,${encoded}`);
  const html = await rendered.renderMyQuestions({
    client,
    queries: {
      courseScope: async () => {
        throw new Error("My Questions does not read a Course");
      },
      assessmentAttemptScope: async () => {
        throw new Error("My Questions does not read an Attempt");
      },
      assessmentAttemptHistory: async () => {
        throw new Error("My Questions does not read Attempt history");
      },
    },
  });
  assert.match(html, /My Published Question pages/);
  assert.match(html, />Previous</);
  assert.match(html, />Next</);
  assert.match(html, /Records per page/);
  assert.match(html, new RegExp(FAST_UI_QUESTION_LIBRARY_IDS[0]));
  assert.doesNotMatch(html, /More Published Questions match/);
});
