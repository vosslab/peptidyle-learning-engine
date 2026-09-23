import type { QuestionSearchPage } from "../../generated/api/QuestionSearchPage";
import {
  BLOOM_COGNITIVE_PROCESSES,
  BLOOM_KNOWLEDGE_DIMENSIONS,
} from "../../src/api/decoders/bloom_classification";
import { validateCanonicalQuestionIdSyntax } from "../../generated/api/QuestionIdSyntaxContract";

export const FAST_UI_QUESTION_LIBRARY_SEED = "fast-ui-question-library-v1";

function questionLibrarySeedIds(): ReadonlyArray<string> {
  const identifiers: Array<string> = [];
  const alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
  for (let payload = 0; identifiers.length < 18; payload += 1) {
    const suffix = payload.toString(32).padStart(3, "0");
    for (const checkCharacter of alphabet) {
      const candidate = `FAST-${checkCharacter}${suffix}`;
      if (validateCanonicalQuestionIdSyntax(candidate) !== null) {
        identifiers.push(candidate);
        break;
      }
    }
  }
  return identifiers;
}

export const FAST_UI_QUESTION_LIBRARY_IDS = questionLibrarySeedIds();

export function fastUiQuestionLibraryPage(): QuestionSearchPage {
  return {
    items: FAST_UI_QUESTION_LIBRARY_IDS.map((questionId, index) => ({
      summary: {
        questionId,
        publishedQuestionRevisionTuple: { publishedQuestionId: questionId, revisionNumber: 1 },
        backend: "ple",
        questionFormat: "pleQuestionJson",
        questionType: "multipleChoice",
        capabilities: ["clientRendering"],
        metadata: {
          questionTitle: `Protein structure seed Question ${String(index + 1)}`,
          questionDescription: "A deterministic Question Library row for fast route composition.",
          tags: ["protein"],
          questionLicense: "CC-BY-4.0",
          questionCitation: null,
          language: "en",
        },
        authorship: { authors: [{ displayName: "Fast UI fixture" }] },
        availability: { availability: "available" },
        publishedAt: 1_789_920_000_000,
        bloom: null,
      },
      disciplineName: "Biochemistry",
      disciplineIsRetired: false,
      evidence: { state: "unavailable" },
    })),
    nextCursor: null,
    facets: {
      authorNames: [],
      authorNamesTruncated: false,
      backends: [],
      tags: [{ tag: "protein", count: 18 }],
      tagsTruncated: false,
      subjects: [],
      subjectsTruncated: false,
      topics: [],
      topicsTruncated: false,
      questionTypes: [],
      capabilities: [],
      questionLicenses: [],
      usedInMyCourses: { used: 0 },
      bloomCognitiveProcesses: BLOOM_COGNITIVE_PROCESSES.map((cognitiveProcess) => ({
        cognitiveProcess,
        count: 0,
      })),
      bloomKnowledgeDimensions: BLOOM_KNOWLEDGE_DIMENSIONS.map((knowledgeDimension) => ({
        knowledgeDimension,
        count: 0,
      })),
    },
  };
}
