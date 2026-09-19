import assert from "node:assert/strict";
import test from "node:test";

import { decodePleQuestionJsonSource } from "../src/features/ple_question_json_authoring/question_json_codec.ts";
import {
  pleQuestionJsonPublicPreview,
  serializePleQuestionJsonPublicPreview,
} from "../src/features/ple_question_json_authoring/question_json_public_preview.ts";
import { source } from "./ple_question_json_authoring_support.mjs";

test("all remaining source Question Types retain semantic IDs and publish answer-free Question Response Formats", () => {
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
        randomizeChoices: true,
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
          questionAsset: "00000000-0000-4000-8000-000000000042",
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
      questionAsset: "00000000-0000-4000-8000-000000000042",
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
      questionAsset: "00000000-0000-4000-8000-000000000042",
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

test("remaining source Question Types reject invalid private contracts", () => {
  assert.throws(() =>
    decodePleQuestionJsonSource({
      ...source(),
      response: {
        kind: "fillIn",
        answers: ["ATP"],
        matchMode: "exact",
        maxLength: 4,
        randomizeChoices: true,
      },
    }),
  );
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
          asset: "00000000-0000-4000-8000-000000000042",
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
          questionAsset: "00000000-0000-4000-8000-000000000042",
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
