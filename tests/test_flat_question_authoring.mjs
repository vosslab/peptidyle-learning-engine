import assert from "node:assert/strict";
import test from "node:test";

import {
  decodeFlatQuestionSource,
  parseFlatQuestionSource,
  serializeFlatQuestionSource,
} from "../src/features/flat_question_authoring/flat_question_codec.ts";
import { createDefaultFlatQuestionSource } from "../src/features/flat_question_authoring/flat_question_defaults.ts";
import {
  createFlatQuestionClient,
  FlatQuestionConflictError,
  FlatQuestionProtocolError,
} from "../src/features/flat_question_authoring/flat_question_client.ts";
import {
  flatQuestionPublicPreview,
  serializeFlatQuestionPublicPreview,
} from "../src/features/flat_question_authoring/flat_question_public_preview.ts";
import {
  createFlatQuestionRepository,
  FlatQuestionStaleConflictError,
} from "../src/features/flat_question_authoring/flat_question_repository.ts";
import { FLAT_QUESTION_MEDIA_TYPE } from "../src/features/flat_question_authoring/flat_question_source.ts";

const workspace = "00000000-0000-4000-8000-000000000001";

function source() {
  return {
    format: "pleFlatQuestion",
    version: 2,
    title: "Favorite color",
    prompt: "What is my favorite color?",
    response: {
      kind: "singleChoice",
      choices: [
        { id: "blue", text: "Blue", feedback: "Correct choice." },
        { id: "red", text: "Red", feedback: "Not this one." },
      ],
      correctChoice: "blue",
    },
    feedback: { correct: "Exactly right.", incorrect: "Try again." },
    points: 1,
    questionAttemptLimit: { maxAttempts: null },
    questionAttemptTimeLimit: { kind: "unlimited" },
    tags: ["example"],
    taxonomy: [],
    license: { kind: "ccBySa" },
    language: "en-US",
  };
}

function publicDefinition(includeVersion = false) {
  const definition = {
    workspace,
    source: { backend: "native" },
    questionFormat: "pleFlatQuestionV2",
    prompt: [{ kind: "text", markdown: "What is my favorite color?" }],
    response: {
      kind: "multipleChoice",
      choices: [
        { id: "blue", body: [{ kind: "text", markdown: "Blue" }] },
        { id: "red", body: [{ kind: "text", markdown: "Red" }] },
      ],
      selection: { kind: "exactlyOne" },
    },
    questionType: "multipleChoice",
    questionAttemptLimit: { maxAttempts: null },
    questionAttemptTimeLimit: { kind: "unlimited" },
    questionVariationDefinition: { kind: "static" },
    grading: { mode: "allOrNothing", points: 1 },
    metadata: {
      title: "Favorite color",
      tags: ["example"],
      taxonomy: [],
      license: { kind: "ccBySa" },
      language: "en-US",
    },
  };
  if (!includeVersion) return definition;
  return {
    ...definition,
    problem: "00000000-0000-4000-8000-000000000002",
    version: "00000000-0000-4000-8000-000000000003",
  };
}

function publicationSummary(backend = "native") {
  return {
    questionId: "7K3-M9QP",
    backend,
    questionType: "multipleChoice",
    capabilities: ["serverGrading"],
    metadata: publicDefinition().metadata,
    byline: { names: ["Fixture Instructor"] },
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
  const decoded = decodeFlatQuestionSource(source());
  const serialized = serializeFlatQuestionSource(decoded);
  assert.equal(serialized, JSON.stringify(source()));
  assert.deepEqual(parseFlatQuestionSource(serialized), decoded);
});

test("codec rejects every retired question-level feedback disclosure value", () => {
  for (const feedback of ["immediateFull", "immediateCorrectness", "deferred", "onRelease"]) {
    assert.throws(() =>
      decodeFlatQuestionSource({
        ...source(),
        questionAttemptLimit: { maxAttempts: null, feedback },
      }),
    );
  }
});

test("codec normalizes omitted optional feedback to Rust canonical null members", () => {
  const input = source();
  delete input.response.choices[1].feedback;
  input.feedback = {};
  const serialized = serializeFlatQuestionSource(decodeFlatQuestionSource(input));
  assert.equal(serialized.includes('"feedback":null'), true);
  assert.equal(serialized.includes('"correct":null'), true);
  assert.equal(serialized.includes('"incorrect":null'), true);
});

test("codec aligns Rust top-level defaults and canonicalizes them on serialization", () => {
  const input = source();
  delete input.feedback;
  delete input.tags;
  delete input.taxonomy;
  const serialized = serializeFlatQuestionSource(decodeFlatQuestionSource(input));
  assert.equal(
    serialized,
    JSON.stringify({
      ...source(),
      feedback: { correct: null, incorrect: null },
      tags: [],
      taxonomy: [],
    }),
  );
});

test("codec enforces Unicode title and Rust u32 numeric bounds", () => {
  const title512 = "😀".repeat(512);
  assert.equal(decodeFlatQuestionSource({ ...source(), title: title512 }).title, title512);
  assert.throws(() => decodeFlatQuestionSource({ ...source(), title: "😀".repeat(513) }));

  const maximum = 4_294_967_295;
  const timed = {
    ...source(),
    questionAttemptLimit: { maxAttempts: maximum },
    questionAttemptTimeLimit: { kind: "limited", seconds: maximum, graceSeconds: maximum },
  };
  assert.deepEqual(decodeFlatQuestionSource(timed).questionAttemptTimeLimit, timed.questionAttemptTimeLimit);
  assert.throws(() =>
    decodeFlatQuestionSource({
      ...timed,
      questionAttemptLimit: { maxAttempts: maximum + 1 },
    }),
  );
  assert.throws(() =>
    decodeFlatQuestionSource({
      ...timed,
      questionAttemptTimeLimit: { kind: "limited", seconds: maximum + 1, graceSeconds: 0 },
    }),
  );
  assert.throws(() =>
    decodeFlatQuestionSource({
      ...timed,
      questionAttemptTimeLimit: { kind: "limited", seconds: 1, graceSeconds: maximum + 1 },
    }),
  );
});

test("source JSON parse failures do not expose parser details or source text", () => {
  const secret = "correctChoice blue private feedback";
  assert.throws(
    () => parseFlatQuestionSource(`{${secret}`),
    (error) => {
      assert.equal(error.message.includes(secret), false);
      assert.equal(error.message.includes("Unexpected"), false);
      return true;
    },
  );
});

test("codec rejects unknown fields, invalid identifiers, invalid choice count, and bad correct choices", () => {
  assert.throws(() => decodeFlatQuestionSource({ ...source(), surprise: true }));
  assert.throws(() =>
    decodeFlatQuestionSource({
      ...source(),
      response: { ...source().response, choices: [{ id: "A", text: "Only one" }] },
    }),
  );
  assert.throws(() =>
    decodeFlatQuestionSource({
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
    decodeFlatQuestionSource({
      ...source(),
      response: { ...source().response, correctChoice: "green" },
    }),
  );
});

test("defaults use stable semantic IDs and public preview cannot serialize answers or feedback", () => {
  const defaults = createDefaultFlatQuestionSource();
  assert.deepEqual(
    defaults.response.choices.map((choice) => choice.id),
    ["choice_a", "choice_b"],
  );
  const preview = flatQuestionPublicPreview(source());
  assert.deepEqual(preview.response, {
    kind: "multipleChoice",
    choices: [
      { id: "blue", body: [{ kind: "text", markdown: "Blue" }] },
      { id: "red", body: [{ kind: "text", markdown: "Red" }] },
    ],
    selection: { kind: "exactlyOne" },
  });
  const serialized = serializeFlatQuestionPublicPreview(source());
  assert.equal(serialized.includes("correctChoice"), false);
  assert.equal(serialized.includes("Correct choice."), false);
  assert.equal(serialized.includes("Exactly right."), false);
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
  const decoded = decodeFlatQuestionSource(matching);
  assert.equal(decoded.response.kind, "matching");
  const preview = serializeFlatQuestionPublicPreview(decoded);
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
  assert.throws(() => decodeFlatQuestionSource(matching));
});

test("all remaining v2 source Question Types retain semantic IDs and publish answer-free Question Response Formats", () => {
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
    const decoded = decodeFlatQuestionSource({ ...source(), response: item.response });
    assert.equal(decoded.response.kind, item.kind);
    const publicResponse = flatQuestionPublicPreview(decoded).response;
    assert.equal(publicResponse.kind, item.publicKind);
    const serialized = serializeFlatQuestionPublicPreview(decoded);
    assert.equal(serialized.includes(item.secret), false);
  }
  const numericWithoutUnit = decodeFlatQuestionSource({
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
  const oneCorrect = decodeFlatQuestionSource({
    ...source(),
    response: { ...baseResponse, correctRegions: ["centromere"] },
  });
  const twoCorrect = decodeFlatQuestionSource({
    ...source(),
    response: { ...baseResponse, correctRegions: ["centromere", "telomere"] },
  });

  const onePublic = flatQuestionPublicPreview(oneCorrect).response;
  const twoPublic = flatQuestionPublicPreview(twoCorrect).response;
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
  assert.equal(serializeFlatQuestionPublicPreview(oneCorrect).includes("correctRegions"), false);
  assert.equal(serializeFlatQuestionPublicPreview(twoCorrect).includes("correctRegions"), false);
});

test("remaining v2 source Question Types reject invalid private contracts", () => {
  assert.throws(() =>
    decodeFlatQuestionSource({
      ...source(),
      response: {
        kind: "multipleAnswer",
        choices: source().response.choices,
        correctChoices: ["blue", "blue"],
      },
    }),
  );
  assert.throws(() =>
    decodeFlatQuestionSource({
      ...source(),
      response: { kind: "fillIn", answers: ["ATP", "ATP"], matchMode: "exact", maxLength: 4 },
    }),
  );
  assert.throws(() =>
    decodeFlatQuestionSource({
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
    decodeFlatQuestionSource({
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
    decodeFlatQuestionSource({
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
    decodeFlatQuestionSource({
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
    decodeFlatQuestionSource({
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
  const client = createFlatQuestionClient({
    basePath: "/ple",
    fetch: async (input, init) => {
      requests.push({ input: String(input), init });
      if (init.method === "GET") {
        return new Response(serializeFlatQuestionSource(source()), {
          headers: { "content-type": `${FLAT_QUESTION_MEDIA_TYPE}; charset=utf-8`, etag: '"1"' },
        });
      }
      if (init.method === "PUT") return jsonResponse(publicDefinition(), 200, '"2"');
      return jsonResponse(publicationSummary(), 201, '"2"');
    },
  });

  const loaded = await client.load(workspace);
  const saved = await client.save(workspace, loaded.source, loaded.revision);
  const publicationRequest = { byline: { names: ["Fixture Instructor"] } };
  const published = await client.publish(workspace, publicationRequest, saved.revision);
  assert.deepEqual(published, publicationSummary());

  assert.equal(requests[0].input, `/ple/api/workspaces/${workspace}/flat-question`);
  assert.equal(requests[0].init.headers.accept, FLAT_QUESTION_MEDIA_TYPE);
  assert.equal(requests[1].init.method, "PUT");
  assert.equal(requests[1].init.headers["content-type"], FLAT_QUESTION_MEDIA_TYPE);
  assert.equal(requests[1].init.headers["if-match"], '"1"');
  assert.equal(requests[1].init.body, serializeFlatQuestionSource(source()));
  assert.equal(requests[2].input, `/ple/api/problems/${workspace}/flat-question-publish`);
  assert.equal(requests[2].init.body, JSON.stringify(publicationRequest));
  assert.equal(requests[2].init.headers["if-match"], '"2"');
});

test("publication rejects invalid reviewed bylines before it can make a request", async () => {
  const client = createFlatQuestionClient({
    fetch: async () => {
      throw new Error("invalid reviewed bylines must not reach fetch");
    },
  });
  for (const names of [["Ada\u0007"], ["😀".repeat(121)], ["Ada Lovelace", "Ada Lovelace"]]) {
    await assert.rejects(
      client.publish(workspace, { byline: { names } }, '"1"'),
      FlatQuestionProtocolError,
    );
  }
});

test("client rejects unsafe base paths before it can make a request", () => {
  for (const basePath of ["//evil.example", "/\\evil.example", "/bad\u0000path", "/bad\npath"]) {
    assert.throws(() => createFlatQuestionClient({ basePath }));
  }
});

test("client requires exact response media types and body-free JSON errors", async () => {
  const secret = "parser body must not surface";
  const client = createFlatQuestionClient({
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

  const malformed = createFlatQuestionClient({
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
  const client = createFlatQuestionClient({
    fetch: async () =>
      new Response(secret, { status: 409, headers: { "content-type": "application/json" } }),
  });
  await assert.rejects(client.load(workspace), (error) => {
    assert.ok(error instanceof FlatQuestionConflictError);
    assert.equal(error.message.includes(secret), false);
    return true;
  });

  const repository = createFlatQuestionRepository({
    async load() {
      return { source: source(), revision: '"1"' };
    },
    async save() {
      throw new FlatQuestionConflictError(409, "/api/workspaces/test/flat-question");
    },
    async publish() {
      throw new Error("not used");
    },
  });
  await repository.load(workspace);
  const edited = source();
  await assert.rejects(repository.save(workspace, edited), (error) => {
    assert.ok(error instanceof FlatQuestionStaleConflictError);
    assert.equal(error.source, edited);
    return true;
  });
});

test("client rejects public responses whose identity does not match the requested workspace", async () => {
  const client = createFlatQuestionClient({
    fetch: async () =>
      jsonResponse({ ...publicDefinition(), workspace: "00000000-0000-4000-8000-000000000099" }),
  });
  await assert.rejects(client.save(workspace, source()), /does not match its workspace/u);
});

test("client rejects save DTOs and publication summaries that do not exactly confirm publication", async () => {
  const wrongSave = createFlatQuestionClient({
    fetch: async () => jsonResponse({ ...publicDefinition(), questionFormat: "nativeAlgorithmic" }),
  });
  await assert.rejects(wrongSave.save(workspace, source()), /PLE flat-question V2 format/u);

  const wrongPublication = createFlatQuestionClient({
    fetch: async () => jsonResponse(publicationSummary("webwork")),
  });
  await assert.rejects(
    wrongPublication.publish(workspace, { byline: { names: ["Fixture Instructor"] } }, '"1"'),
    /available native Question Library summary/u,
  );

  const staleScope = createFlatQuestionClient({
    fetch: async () => jsonResponse({ ...publicationSummary(), scope: "public" }),
  });
  await assert.rejects(
    staleScope.publish(workspace, { byline: { names: ["Fixture Instructor"] } }, '"1"'),
    /scope must be a field allowed/u,
  );

  for (const summary of [
    { ...publicationSummary(), availability: { availability: "archived", reason: "withdrawn" } },
  ]) {
    const wrongLifecycleOrScope = createFlatQuestionClient({
      fetch: async () => jsonResponse(summary),
    });
    await assert.rejects(
      wrongLifecycleOrScope.publish(
        workspace,
        { byline: { names: ["Fixture Instructor"] } },
        '"1"',
      ),
      /available native Question Library summary/u,
    );
  }
});

test("client accepts the exact native hotspot Question Type for a strict hotspot source", async () => {
  const hotspot = decodeFlatQuestionSource({
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
  const client = createFlatQuestionClient({
    fetch: async () =>
      jsonResponse({
        ...publicDefinition(),
        questionType: "hotspot",
        response: {
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
          ],
          selection: { kind: "atLeastOne" },
        },
      }),
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
  const repository = createFlatQuestionRepository({
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
  secondSave.resolve({ draft: publicDefinition(), revision: '"3"' });
  await newer;
  firstSave.resolve({ draft: publicDefinition(), revision: '"2"' });
  await older;
  await repository.publish(workspace, { byline: { names: ["Fixture Instructor"] } });
  assert.deepEqual(observedRevisions, ['"1"', '"1"']);
  assert.equal(publishedRevision, '"3"');
});
