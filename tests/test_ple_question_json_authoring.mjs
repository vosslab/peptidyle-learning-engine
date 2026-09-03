import assert from "node:assert/strict";
import test from "node:test";

import {
  decodePleQuestionJsonSource,
  parsePleQuestionJsonSource,
  serializePleQuestionJsonSource,
} from "../src/features/ple_question_json_authoring/question_json_codec.ts";
import { createDefaultPleQuestionJsonSource } from "../src/features/ple_question_json_authoring/question_json_defaults.ts";
import {
  createPleQuestionJsonClient,
  PleQuestionJsonConflictError,
  PleQuestionJsonProtocolError,
} from "../src/features/ple_question_json_authoring/question_json_client.ts";
import {
  pleQuestionJsonPublicPreview,
  serializePleQuestionJsonPublicPreview,
} from "../src/features/ple_question_json_authoring/question_json_public_preview.ts";
import {
  createPleQuestionJsonRepository,
  PleQuestionJsonStaleConflictError,
} from "../src/features/ple_question_json_authoring/question_json_repository.ts";
import { PLE_QUESTION_JSON_MEDIA_TYPE } from "../src/features/ple_question_json_authoring/question_json_source.ts";

const workspace = "00000000-0000-4000-8000-000000000001";

function source() {
  return {
    format: "pleQuestionJson",
    version: 3,
    title: "Favorite color",
    questionDescription: "Instructor-facing color-choice example.",
    prompt: "What is my favorite color?",
    response: {
      kind: "singleChoice",
      choices: [
        { id: "blue", text: "Blue", feedback: "Correct choice." },
        { id: "red", text: "Red", feedback: "Not this one." },
      ],
      correctChoice: "blue",
    },
    questionHint: "Compare each choice before responding.",
    feedback: { correct: "Exactly right.", incorrect: "Try again." },
    tags: ["example"],
    questionLicense: "CC-BY-SA-4.0",
    questionCitation: null,
    language: "en-US",
  };
}

function publicationSummary(backend = "ple") {
  return {
    questionId: "7K3-M9QP",
    latestQuestionRevision: { questionId: "7K3-M9QP", revisionNumber: 1 },
    backend,
    questionType: "multipleChoice",
    capabilities: ["serverGrading"],
    metadata: {
      title: "Favorite color",
      questionDescription: "Instructor-facing color-choice example.",
      tags: ["example"],
      questionLicense: "CC-BY-SA-4.0",
      questionCitation: null,
      language: "en-US",
    },
    authorship: { authors: [{ displayName: "Fixture Instructor" }] },
    availability: { availability: "available" },
    publishedAt: 1786000000000,
  };
}

function jsonResponse(value, status = 200, revision = '"1"') {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json", etag: revision },
  });
}

test("codec accepts a valid source and serializes deterministic compact JSON", () => {
  const decoded = decodePleQuestionJsonSource(source());
  const serialized = serializePleQuestionJsonSource(decoded);
  assert.equal(serialized, JSON.stringify(source()));
  assert.deepEqual(parsePleQuestionJsonSource(serialized), decoded);
});

test("Question Citation is exact optional source credit and never becomes an empty record", () => {
  const decoded = decodePleQuestionJsonSource({
    ...source(),
    questionCitation: {
      citationUrl: "https://example.org/reference",
      citationText: "Voss NR. Question source reference. 2026.",
    },
  });
  assert.deepEqual(decoded.questionCitation, {
    citationUrl: "https://example.org/reference",
    citationText: "Voss NR. Question source reference. 2026.",
  });
  assert.throws(() =>
    decodePleQuestionJsonSource({
      ...source(),
      questionCitation: { citationUrl: null, citationText: null },
    }),
  );
});

test("codec normalizes omitted optional feedback to Rust canonical null members", () => {
  const input = source();
  delete input.response.choices[1].feedback;
  input.feedback = {};
  const serialized = serializePleQuestionJsonSource(decodePleQuestionJsonSource(input));
  assert.equal(serialized.includes('"feedback":null'), true);
  assert.equal(serialized.includes('"correct":null'), true);
  assert.equal(serialized.includes('"incorrect":null'), true);
});

test("codec aligns Rust top-level defaults and canonicalizes them on serialization", () => {
  const input = source();
  delete input.feedback;
  delete input.tags;
  const serialized = serializePleQuestionJsonSource(decodePleQuestionJsonSource(input));
  assert.equal(
    serialized,
    JSON.stringify({
      ...source(),
      feedback: { correct: null, incorrect: null },
      tags: [],
    }),
  );
});

test("codec enforces Unicode title bounds", () => {
  const title512 = "😀".repeat(512);
  assert.equal(decodePleQuestionJsonSource({ ...source(), title: title512 }).title, title512);
  assert.throws(() => decodePleQuestionJsonSource({ ...source(), title: "😀".repeat(513) }));
  assert.throws(() => decodePleQuestionJsonSource({ ...source(), questionDescription: "   " }));
  assert.throws(() =>
    decodePleQuestionJsonSource({ ...source(), questionDescription: "😀".repeat(4_001) }),
  );
});

test("source JSON parse failures do not expose parser details or source text", () => {
  const secret = "correctChoice blue private feedback";
  assert.throws(
    () => parsePleQuestionJsonSource(`{${secret}`),
    (error) => {
      assert.equal(error.message.includes(secret), false);
      assert.equal(error.message.includes("Unexpected"), false);
      return true;
    },
  );
});

test("codec rejects unknown fields, invalid identifiers, invalid choice count, and bad correct choices", () => {
  assert.throws(() => decodePleQuestionJsonSource({ ...source(), surprise: true }));
  assert.throws(() => decodePleQuestionJsonSource({ ...source(), classifications: [] }));
  assert.throws(() =>
    decodePleQuestionJsonSource({
      ...source(),
      response: { ...source().response, choices: [{ id: "A", text: "Only one" }] },
    }),
  );
  assert.throws(() =>
    decodePleQuestionJsonSource({
      ...source(),
      response: {
        ...source().response,
        choices: [
          { id: "Bad", text: "One" },
          { id: "two", text: "Two" },
        ],
      },
    }),
  );
  assert.throws(() =>
    decodePleQuestionJsonSource({
      ...source(),
      response: { ...source().response, correctChoice: "green" },
    }),
  );
});

test("defaults use stable semantic IDs and public preview cannot serialize answers, Question Hint, or feedback", () => {
  const defaults = createDefaultPleQuestionJsonSource();
  assert.deepEqual(
    defaults.response.choices.map((choice) => choice.id),
    ["choice_a", "choice_b"],
  );
  const preview = pleQuestionJsonPublicPreview(source());
  assert.deepEqual(preview.response, {
    kind: "multipleChoice",
    choices: [
      { id: "blue", body: [{ kind: "text", markdown: "Blue" }] },
      { id: "red", body: [{ kind: "text", markdown: "Red" }] },
    ],
    selection: { kind: "exactlyOne" },
  });
  const serialized = serializePleQuestionJsonPublicPreview(source());
  assert.equal(serialized.includes("correctChoice"), false);
  assert.equal(serialized.includes("Compare each choice before responding."), false);
  assert.equal(serialized.includes("Correct choice."), false);
  assert.equal(serialized.includes("Exactly right."), false);
});

test("codec accepts an optional Question Hint and rejects blank or oversized authored help", () => {
  assert.equal(
    decodePleQuestionJsonSource(source()).questionHint,
    "Compare each choice before responding.",
  );
  assert.equal(decodePleQuestionJsonSource({ ...source(), questionHint: null }).questionHint, null);
  assert.throws(() => decodePleQuestionJsonSource({ ...source(), questionHint: "  " }));
  assert.throws(() =>
    decodePleQuestionJsonSource({ ...source(), questionHint: "x".repeat(16_385) }),
  );
});

test("matching codec retains stable pairs while public preview excludes their answer map", () => {
  const matching = {
    ...source(),
    response: {
      kind: "matching",
      prompts: [
        { id: "gene", text: "Gene" },
        { id: "allele", text: "Allele" },
      ],
      choices: [
        { id: "dna_segment", text: "DNA segment" },
        { id: "gene_variant", text: "Gene variant" },
      ],
      matches: [
        { prompt: "gene", choice: "dna_segment" },
        { prompt: "allele", choice: "gene_variant" },
      ],
    },
  };
  const decoded = decodePleQuestionJsonSource(matching);
  assert.equal(decoded.response.kind, "matching");
  const preview = serializePleQuestionJsonPublicPreview(decoded);
  assert.equal(preview.includes('"matches"'), false);
  assert.equal(preview.includes('"gene_variant"'), true);
});

test("matching codec refuses duplicate or incomplete pairings", () => {
  const matching = {
    ...source(),
    response: {
      kind: "matching",
      prompts: [
        { id: "one", text: "One" },
        { id: "two", text: "Two" },
      ],
      choices: [
        { id: "first", text: "First" },
        { id: "second", text: "Second" },
      ],
      matches: [
        { prompt: "one", choice: "first" },
        { prompt: "two", choice: "first" },
      ],
    },
  };
  assert.throws(() => decodePleQuestionJsonSource(matching));
});

test("all remaining v3 source Question Types retain semantic IDs and publish answer-free Question Response Formats", () => {
  const cases = [
    {
      kind: "multipleAnswer",
      response: {
        kind: "multipleAnswer",
        choices: [
          { id: "kinase", text: "Kinase", feedback: "Private feedback" },
          { id: "lipid", text: "Lipid", feedback: null },
        ],
        correctChoices: ["kinase"],
      },
      publicKind: "multipleChoice",
      secret: "correctChoices",
    },
    {
      kind: "fillIn",
      response: {
        kind: "fillIn",
        answers: ["adenosine triphosphate"],
        matchMode: "caseInsensitive",
        maxLength: 80,
      },
      publicKind: "shortText",
      secret: "adenosine triphosphate",
    },
    {
      kind: "multiFillIn",
      response: {
        kind: "multiFillIn",
        blanks: [
          {
            id: "energy_currency",
            label: "Cellular energy currency",
            answers: ["ATP"],
            matchMode: "caseInsensitive",
            maxLength: 12,
          },
        ],
      },
      publicKind: "multiBlank",
      secret: "answers",
    },
    {
      kind: "numeric",
      response: {
        kind: "numeric",
        answer: 6.022,
        tolerance: { kind: "relative", fraction: 0.01 },
        unit: "mol^-1",
      },
      publicKind: "numeric",
      secret: '"answer":6.022',
    },
    {
      kind: "ordering",
      response: {
        kind: "ordering",
        items: [
          { id: "template", text: "Template binding" },
          { id: "elongation", text: "Elongation" },
          { id: "termination", text: "Termination" },
        ],
        correctOrder: ["template", "elongation", "termination"],
      },
      publicKind: "ordering",
      secret: "correctOrder",
    },
    {
      kind: "hotspot",
      response: {
        kind: "hotspot",
        surface: {
          asset: "00000000-0000-4000-8000-000000000042",
          checksum: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          description: "A chromosome map",
        },
        regions: [
          { id: "centromere", label: "Centromere", x: 0, y: 0, width: 4_000, height: 4_000 },
          { id: "telomere", label: "Telomere", x: 6_000, y: 6_000, width: 4_000, height: 4_000 },
        ],
        correctRegions: ["centromere"],
      },
      publicKind: "hotspot",
      secret: "correctRegions",
    },
  ];

  for (const item of cases) {
    const decoded = decodePleQuestionJsonSource({ ...source(), response: item.response });
    assert.equal(decoded.response.kind, item.kind);
    const publicResponse = pleQuestionJsonPublicPreview(decoded).response;
    assert.equal(publicResponse.kind, item.publicKind);
    const serialized = serializePleQuestionJsonPublicPreview(decoded);
    assert.equal(serialized.includes(item.secret), false);
  }
  const numericWithoutUnit = decodePleQuestionJsonSource({
    ...source(),
    response: { kind: "numeric", answer: 1, tolerance: { kind: "exact" } },
  });
  assert.equal(numericWithoutUnit.response.kind, "numeric");
  if (numericWithoutUnit.response.kind !== "numeric") throw new Error("Expected numeric source.");
  assert.equal(numericWithoutUnit.response.unit, null);
});

test("hotspot public preview does not disclose correct-region cardinality", () => {
  const baseResponse = {
    kind: "hotspot",
    surface: {
      asset: "00000000-0000-4000-8000-000000000042",
      checksum: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      description: "A chromosome map",
    },
    regions: [
      { id: "centromere", label: "Centromere", x: 0, y: 0, width: 4_000, height: 4_000 },
      { id: "telomere", label: "Telomere", x: 6_000, y: 6_000, width: 4_000, height: 4_000 },
    ],
  };
  const oneCorrect = decodePleQuestionJsonSource({
    ...source(),
    response: { ...baseResponse, correctRegions: ["centromere"] },
  });
  const twoCorrect = decodePleQuestionJsonSource({
    ...source(),
    response: { ...baseResponse, correctRegions: ["centromere", "telomere"] },
  });

  const onePublic = pleQuestionJsonPublicPreview(oneCorrect).response;
  const twoPublic = pleQuestionJsonPublicPreview(twoCorrect).response;
  assert.deepEqual(onePublic, twoPublic);
  assert.deepEqual(onePublic, {
    kind: "hotspot",
    surface: {
      asset: "00000000-0000-4000-8000-000000000042",
      checksum: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    },
    description: "A chromosome map",
    regions: [
      {
        id: "centromere",
        label: [{ kind: "text", markdown: "Centromere" }],
        x: 0,
        y: 0,
        width: 4_000,
        height: 4_000,
      },
      {
        id: "telomere",
        label: [{ kind: "text", markdown: "Telomere" }],
        x: 6_000,
        y: 6_000,
        width: 4_000,
        height: 4_000,
      },
    ],
    selection: { kind: "atLeastOne" },
  });
  assert.equal(serializePleQuestionJsonPublicPreview(oneCorrect).includes("correctRegions"), false);
  assert.equal(serializePleQuestionJsonPublicPreview(twoCorrect).includes("correctRegions"), false);
});

test("remaining v3 source Question Types reject invalid private contracts", () => {
  assert.throws(() =>
    decodePleQuestionJsonSource({
      ...source(),
      response: {
        kind: "multipleAnswer",
        choices: source().response.choices,
        correctChoices: ["blue", "blue"],
      },
    }),
  );
  assert.throws(() =>
    decodePleQuestionJsonSource({
      ...source(),
      response: { kind: "fillIn", answers: ["ATP", "ATP"], matchMode: "exact", maxLength: 4 },
    }),
  );
  assert.throws(() =>
    decodePleQuestionJsonSource({
      ...source(),
      response: {
        kind: "multiFillIn",
        blanks: [
          { id: "same", label: "One", answers: ["one"], matchMode: "exact", maxLength: 8 },
          { id: "same", label: "Two", answers: ["two"], matchMode: "exact", maxLength: 8 },
        ],
      },
    }),
  );
  assert.throws(() =>
    decodePleQuestionJsonSource({
      ...source(),
      response: {
        kind: "numeric",
        answer: 1,
        tolerance: { kind: "significantFigures", digits: 0 },
        unit: null,
      },
    }),
  );
  assert.throws(() =>
    decodePleQuestionJsonSource({
      ...source(),
      response: {
        kind: "ordering",
        items: [
          { id: "first", text: "First" },
          { id: "second", text: "Second" },
          { id: "third", text: "Third" },
        ],
        correctOrder: ["first", "second", "second"],
      },
    }),
  );
  assert.throws(() =>
    decodePleQuestionJsonSource({
      ...source(),
      response: {
        kind: "hotspot",
        surface: {
          asset: "not-an-asset",
          checksum: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          description: "Surface",
        },
        regions: [{ id: "bad", label: "Bad", x: 9_000, y: 0, width: 2_000, height: 1_000 }],
        correctRegions: ["bad"],
      },
    }),
  );
  assert.throws(() =>
    decodePleQuestionJsonSource({
      ...source(),
      response: {
        kind: "hotspot",
        surface: {
          asset: "00000000-0000-4000-8000-000000000042",
          checksum: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          description: "Surface",
        },
        regions: [
          { id: "left", label: "Left", x: 0, y: 0, width: 4_000, height: 4_000 },
          { id: "right", label: "Right", x: 3_000, y: 3_000, width: 4_000, height: 4_000 },
        ],
        correctRegions: ["left"],
      },
    }),
  );
});

test("client sends exact protected paths, headers, body, and revisions", async () => {
  const requests = [];
  const client = createPleQuestionJsonClient({
    basePath: "/ple",
    fetch: async (input, init) => {
      requests.push({ input: String(input), init });
      if (init.method === "GET") {
        return new Response(serializePleQuestionJsonSource(source()), {
          headers: {
            "content-type": `${PLE_QUESTION_JSON_MEDIA_TYPE}; charset=utf-8`,
            etag: '"1"',
          },
        });
      }
      if (init.method === "PUT") return jsonResponse({ saved: true }, 200, '"2"');
      return jsonResponse(publicationSummary(), 201, '"2"');
    },
  });

  const loaded = await client.load(workspace);
  const saved = await client.save(workspace, loaded.source, loaded.revision);
  const publicationRequest = { authorship: { authors: [{ displayName: "Fixture Instructor" }] } };
  const published = await client.publish(workspace, publicationRequest, saved.revision);
  assert.deepEqual(published, publicationSummary());

  assert.equal(requests[0].input, `/ple/api/workspaces/${workspace}/ple-question-json`);
  assert.equal(requests[0].init.headers.accept, PLE_QUESTION_JSON_MEDIA_TYPE);
  assert.equal(requests[1].init.method, "PUT");
  assert.equal(requests[1].init.headers["content-type"], PLE_QUESTION_JSON_MEDIA_TYPE);
  assert.equal(requests[1].init.headers["if-match"], '"1"');
  assert.equal(requests[1].init.body, serializePleQuestionJsonSource(source()));
  assert.equal(requests[2].input, `/ple/api/questions/${workspace}/ple-question-json-publish`);
  assert.equal(requests[2].init.body, JSON.stringify(publicationRequest));
  assert.equal(requests[2].init.headers["if-match"], '"2"');
});

test("publication rejects invalid reviewed Question Authorship before it can make a request", async () => {
  const client = createPleQuestionJsonClient({
    fetch: async () => {
      throw new Error("invalid reviewed Question Authorship must not reach fetch");
    },
  });
  for (const authors of [
    [{ displayName: "Ada\u0007" }],
    [{ displayName: "😀".repeat(121) }],
    [{ displayName: "Ada Lovelace" }, { displayName: "Ada Lovelace" }],
  ]) {
    await assert.rejects(
      client.publish(workspace, { authorship: { authors } }, '"1"'),
      PleQuestionJsonProtocolError,
    );
  }
});

test("client rejects unsafe base paths before it can make a request", () => {
  for (const basePath of ["//evil.example", "/\\evil.example", "/bad\u0000path", "/bad\npath"]) {
    assert.throws(() => createPleQuestionJsonClient({ basePath }));
  }
});

test("client requires exact response media types and body-free JSON errors", async () => {
  const secret = "parser body must not surface";
  const client = createPleQuestionJsonClient({
    fetch: async () =>
      new Response(`{${secret}`, {
        headers: { "content-type": "application/json-everything" },
      }),
  });
  await assert.rejects(client.save(workspace, source()), (error) => {
    assert.equal(error.message.includes(secret), false);
    assert.match(error.message, /application\/json/u);
    return true;
  });

  const malformed = createPleQuestionJsonClient({
    fetch: async () =>
      new Response(`{${secret}`, {
        headers: { "content-type": "application/json" },
      }),
  });
  await assert.rejects(malformed.save(workspace, source()), (error) => {
    assert.equal(error.message.includes(secret), false);
    assert.equal(error.message.includes("Unexpected"), false);
    return true;
  });
});

test("conflicts do not echo a response body and repository preserves the caller source", async () => {
  const secret = "correctChoice=blue private feedback";
  const client = createPleQuestionJsonClient({
    fetch: async () =>
      new Response(secret, { status: 409, headers: { "content-type": "application/json" } }),
  });
  await assert.rejects(client.load(workspace), (error) => {
    assert.ok(error instanceof PleQuestionJsonConflictError);
    assert.equal(error.message.includes(secret), false);
    return true;
  });

  const repository = createPleQuestionJsonRepository({
    async load() {
      return { source: source(), revision: '"1"' };
    },
    async save() {
      throw new PleQuestionJsonConflictError(409, "/api/workspaces/test/ple-question-json");
    },
    async publish() {
      throw new Error("not used");
    },
  });
  await repository.load(workspace);
  const edited = source();
  await assert.rejects(repository.save(workspace, edited), (error) => {
    assert.ok(error instanceof PleQuestionJsonStaleConflictError);
    assert.equal(error.source, edited);
    return true;
  });
});

test("client rejects publication summaries that do not exactly confirm publication", async () => {
  const wrongPublication = createPleQuestionJsonClient({
    fetch: async () => jsonResponse(publicationSummary("webwork")),
  });
  await assert.rejects(
    wrongPublication.publish(
      workspace,
      { authorship: { authors: [{ displayName: "Fixture Instructor" }] } },
      '"1"',
    ),
    /available PLE Question Library summary/u,
  );

  const staleScope = createPleQuestionJsonClient({
    fetch: async () => jsonResponse({ ...publicationSummary(), scope: "public" }),
  });
  await assert.rejects(
    staleScope.publish(
      workspace,
      { authorship: { authors: [{ displayName: "Fixture Instructor" }] } },
      '"1"',
    ),
    /scope must be a field allowed/u,
  );

  for (const summary of [
    { ...publicationSummary(), availability: { availability: "archived", reason: "withdrawn" } },
  ]) {
    const wrongLifecycleOrScope = createPleQuestionJsonClient({
      fetch: async () => jsonResponse(summary),
    });
    await assert.rejects(
      wrongLifecycleOrScope.publish(
        workspace,
        { authorship: { authors: [{ displayName: "Fixture Instructor" }] } },
        '"1"',
      ),
      /available PLE Question Library summary/u,
    );
  }
});

test("client saves a strict PLE hotspot source through its exact endpoint", async () => {
  const hotspot = decodePleQuestionJsonSource({
    ...source(),
    response: {
      kind: "hotspot",
      surface: {
        asset: "00000000-0000-4000-8000-000000000042",
        checksum: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        description: "A chromosome map",
      },
      regions: [{ id: "centromere", label: "Centromere", x: 0, y: 0, width: 4_000, height: 4_000 }],
      correctRegions: ["centromere"],
    },
  });
  const client = createPleQuestionJsonClient({
    fetch: async () => jsonResponse({ saved: true }),
  });
  await client.save(workspace, hotspot);
});

function deferred() {
  let resolve;
  const promise = new Promise((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

test("repository does not regress a workspace revision when an older save finishes last", async () => {
  const firstSave = deferred();
  const secondSave = deferred();
  const observedRevisions = [];
  let publishedRevision;
  const repository = createPleQuestionJsonRepository({
    async load() {
      return { source: source(), revision: '"1"' };
    },
    save(_workspace, _source, revision) {
      observedRevisions.push(revision);
      return observedRevisions.length === 1 ? firstSave.promise : secondSave.promise;
    },
    async publish(_workspace, _request, revision) {
      publishedRevision = revision;
      return publicationSummary();
    },
  });

  await repository.load(workspace);
  const older = repository.save(workspace, source());
  const newer = repository.save(workspace, source());
  secondSave.resolve({ revision: '"3"' });
  await newer;
  firstSave.resolve({ revision: '"2"' });
  await older;
  await repository.publish(workspace, {
    authorship: { authors: [{ displayName: "Fixture Instructor" }] },
  });
  assert.deepEqual(observedRevisions, ['"1"', '"1"']);
  assert.equal(publishedRevision, '"3"');
});
