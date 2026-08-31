// Literal public fixture for focused Node and Playwright boundary tests.
//
// This test-owned data is intentionally independent of Rust fixture generation,
// the browser source tree, and every production build artifact. It carries no
// answer key, provider credential, object key, or private asset body.

const workspace = "0198e000-0000-7000-8000-000000000002";
const problem = "0198e000-0000-7000-8000-000000000003";
const version = "0198e000-0000-7000-8000-000000000004";
const assignmentId = "0198e000-0000-7000-8000-000000000006";
const studentRecordId = "0198e000-0000-7000-8000-000000000007";
const courseId = "0198e000-0000-7000-8000-000000000014";

const metadata = {
  title: "Peptide bond resonance and planarity",
  tags: ["biochemistry", "protein-structure"],
  taxonomy: [
    {
      scheme: "Peptidyle",
      code: "BIOCHEM.PEPTIDE_BOND",
      label: "Peptide bond structure",
    },
  ],
  license: { kind: "ccBy" },
  language: "en-US",
};

const prompt = [
  {
    kind: "text",
    markdown:
      "In the {{residue}} peptide example, which bond has restricted rotation because resonance gives it partial double-bond character?",
  },
  {
    kind: "image",
    asset: {
      asset: "0198e000-0000-7000-8000-000000000010",
      checksum: "9d6816fe63b5a1410e9e94c132c42029445cc26e0310bcb468c0d14aa218801b",
    },
    description: "Structural formula highlighting the carbonyl carbon-to-nitrogen bond.",
  },
  {
    kind: "image",
    asset: {
      asset: "0198e000-0000-7000-8000-000000000012",
      checksum: "bdb2582fd499b6d020e3d972f0c1bfcb5124977ecfe9a9f87e3a7e554543623b",
    },
    description: "The six atoms of a peptide group shown in one plane.",
  },
];

const response = {
  kind: "multipleChoice",
  choices: [
    {
      id: "amide",
      body: [{ kind: "text", markdown: "The carbonyl carbon-to-nitrogen bond" }],
    },
    {
      id: "carbonyl",
      body: [{ kind: "text", markdown: "The carbonyl carbon-to-oxygen bond" }],
    },
    {
      id: "alpha-carbon",
      body: [{ kind: "text", markdown: "The nitrogen-to-alpha-carbon bond" }],
    },
  ],
  selection: { kind: "exactlyOne" },
};

const questionSettings = {
  source: { backend: "native", family: "peptide_bond_geometry" },
  prompt,
  response,
  attemptPolicy: { maxAttempts: null },
  timingPolicy: { kind: "untimed" },
  randomization: {
    kind: "seeded",
    generator: { id: "peptide-bond-choice", version: "1" },
    parameters: {
      residue: { kind: "choice", options: ["glycine", "alanine", "proline"] },
    },
  },
  grading: { mode: "allOrNothing", points: 1 },
};

function attempt(
  id: string,
  issuedQuestion: string,
  seed: number,
  selected: string | undefined,
  issuedAt: number,
): object {
  const submitted = selected !== undefined;
  return {
    id,
    issuedQuestion,
    seed,
    parameterHash: "a".repeat(64),
    response: submitted ? { kind: "multipleChoice", selected: [selected] } : null,
    status: submitted ? "submitted" : "in_progress",
    result: submitted
      ? {
          correct: selected === "amide",
          pointsEarned: selected === "amide" ? 1 : 0,
          pointsPossible: 1,
        }
      : null,
    timing: { issuedAt, deadline: null, submittedAt: submitted ? issuedAt + 100 : null },
    provenance: {
      adapter: { id: "native-adapter", version: "1" },
      renderer: null,
      generator: { id: "peptide-bond-choice", version: "1" },
      sourceArtifact: null,
      assetObjects: [
        "0198e000-0000-7000-8000-000000000011",
        "0198e000-0000-7000-8000-000000000013",
      ],
      grading: { id: "generic-grader", version: "1" },
      renderedQuestionSha256: "b".repeat(64),
    },
    issuedCapability: "flatPresentation",
    poolSelection: null,
  };
}

export const publishedProblemFixture = {
  catalogProblem: {
    questionId: "7K3-M9QP",
    backend: "native",
    questionType: "multipleChoice",
    capabilities: [
      "algorithmicGeneration",
      "clientRendering",
      "serverGrading",
      "hints",
      "perQuestionTiming",
    ],
    metadata,
    byline: { names: ["Fixture Instructor"] },
    availability: { availability: "available" },
    publishedAt: 1786000000000,
  },
  publishedProblem: {
    problem,
    version,
    workspace,
    ...questionSettings,
    metadata,
  },
  draft: {
    workspace,
    ...questionSettings,
    metadata: { ...metadata, title: "Draft: peptide resonance wording revision" },
  },
  course: {
    id: courseId,
    reference: "C-1",
    title: "BIOC 301: Biochemistry",
    term: { startDate: "2026-08-24", endDate: "2026-12-18", timeZone: "America/Chicago" },
    role: "student",
  },
  assignment: {
    id: assignmentId,
    reference: "A-1",
    courseId,
    title: "Peptide bond mastery",
    entries: [
      {
        kind: "fixedQuestion",
        id: "0198e000-0000-7000-8000-000000000017",
        questionId: "7K3-M9QP",
        title: metadata.title,
        backend: "native",
        capabilities: [
          "algorithmicGeneration",
          "clientRendering",
          "serverGrading",
          "hints",
          "perQuestionTiming",
        ],
        pointsPossible: "1",
        deliveryState: "active",
        scoringMode: "normal",
      },
    ],
    disclosurePolicy: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      feedback_text: "after_submit",
      solution: "after_submit",
      class_statistics: "never",
    },
    policies: {
      completion: { kind: "allCorrect" },
      grade: "highest",
      continuedPractice: { kind: "unlimited" },
      variation: "newSeeds",
    },
  },
  studentRecord: studentRecordId,
  runs: [
    {
      id: "0198e000-0000-7000-8000-000000000020",
      reference: "R-1",
      studentRecord: studentRecordId,
      assignment: assignmentId,
      attemptNumber: 1,
      startedAt: 1786000001000,
      completedAt: 1786000001300,
      score: 0,
      variation: "newSeeds",
    },
    {
      id: "0198e000-0000-7000-8000-000000000021",
      reference: "R-2",
      studentRecord: studentRecordId,
      assignment: assignmentId,
      attemptNumber: 2,
      startedAt: 1786000002000,
      completedAt: 1786000002300,
      score: 1,
      variation: "newSeeds",
    },
  ],
  issuedQuestions: [
    {
      id: "0198e000-0000-7000-8000-000000000040",
      assignmentAttempt: "0198e000-0000-7000-8000-000000000020",
      assignmentEntry: "0198e000-0000-7000-8000-000000000017",
      definitionEntryIndex: 0,
      issuedPosition: 0,
      reference: { problem, version },
      statisticsEligible: true,
      questionPoolEntry: null,
      selectionSeed: null,
    },
    {
      id: "0198e000-0000-7000-8000-000000000041",
      assignmentAttempt: "0198e000-0000-7000-8000-000000000021",
      assignmentEntry: "0198e000-0000-7000-8000-000000000017",
      definitionEntryIndex: 0,
      issuedPosition: 0,
      reference: { problem, version },
      statisticsEligible: true,
      questionPoolEntry: null,
      selectionSeed: null,
    },
  ],
  attempts: [
    attempt(
      "0198e000-0000-7000-8000-000000000030",
      "0198e000-0000-7000-8000-000000000040",
      1001,
      "carbonyl",
      1786000001100,
    ),
    attempt(
      "0198e000-0000-7000-8000-000000000031",
      "0198e000-0000-7000-8000-000000000041",
      1002,
      "amide",
      1786000002100,
    ),
  ],
};
