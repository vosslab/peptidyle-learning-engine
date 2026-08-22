// Literal public fixture for narrow Node and legacy Playwright tests.
//
// This test-owned data is intentionally independent of Rust fixture generation,
// the browser source tree, and every production build artifact. It carries no
// answer key, provider credential, object key, or private asset body.

const tenant = "0198e000-0000-7000-8000-000000000001";
const workspace = "0198e000-0000-7000-8000-000000000002";
const problem = "0198e000-0000-7000-8000-000000000003";
const version = "0198e000-0000-7000-8000-000000000004";
const assignmentId = "0198e000-0000-7000-8000-000000000006";
const enrollmentId = "0198e000-0000-7000-8000-000000000007";
const studentId = "0198e000-0000-7000-8000-000000000008";
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
  run: string,
  seed: number,
  selected: string | undefined,
  issuedAt: number,
): object {
  const submitted = selected !== undefined;
  return {
    id,
    tenant,
    run,
    problem,
    questionVersion: version,
    assignmentPosition: 0,
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
    timer: { issuedAt, deadline: null, submittedAt: submitted ? issuedAt + 100 : null },
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
  };
}

export const publishedProblemFixture = {
  catalogProblem: {
    questionId: "7K3-M9QP",
    backend: "native",
    capabilities: [
      "algorithmicGeneration",
      "clientRendering",
      "serverGrading",
      "hints",
      "perQuestionTiming",
    ],
    metadata,
    byline: { names: ["Fixture Instructor"] },
    scope: "public",
    lifecycle: { state: "published" },
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
    tenant,
    title: "BIOC 301: Biochemistry",
    term: { startDate: "2026-08-24", endDate: "2026-12-18", timeZone: "America/Chicago" },
    role: "student",
  },
  assignment: {
    id: assignmentId,
    reference: "A-1",
    tenant,
    courseId,
    title: "Peptide bond mastery",
    items: [
      {
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
        position: 0,
        pointsPossible: "1",
        deliveryState: "active",
        scoringMode: "normal",
      },
    ],
    selectionGroups: [],
    disclosurePolicy: {
      score: "afterSubmit",
      perItemCorrectness: "afterSubmit",
      feedbackText: "afterSubmit",
      solution: "afterSubmit",
      classStatistics: "never",
    },
    policies: {
      completion: { kind: "allCorrect" },
      grade: "highest",
      continuedPractice: { kind: "unlimited" },
      variation: "newSeeds",
    },
  },
  enrollment: {
    id: enrollmentId,
    tenant,
    assignment: assignmentId,
    user: "0198e000-0000-7000-8000-000000000016",
    student: studentId,
    firstCompletedAt: 1786000001300,
    currentGradeRun: "0198e000-0000-7000-8000-000000000021",
    bestGradeRun: "0198e000-0000-7000-8000-000000000021",
  },
  runs: [
    {
      id: "0198e000-0000-7000-8000-000000000020",
      reference: "R-1",
      tenant,
      enrollment: enrollmentId,
      runNumber: 1,
      startedAt: 1786000001000,
      completedAt: 1786000001300,
      score: 0,
      mode: "assigned",
      variation: "newSeeds",
    },
    {
      id: "0198e000-0000-7000-8000-000000000021",
      reference: "R-2",
      tenant,
      enrollment: enrollmentId,
      runNumber: 2,
      startedAt: 1786000002000,
      completedAt: 1786000002300,
      score: 1,
      mode: "practice",
      variation: "newSeeds",
    },
  ],
  attempts: [
    attempt(
      "0198e000-0000-7000-8000-000000000030",
      "0198e000-0000-7000-8000-000000000020",
      1001,
      "carbonyl",
      1786000001100,
    ),
    attempt(
      "0198e000-0000-7000-8000-000000000031",
      "0198e000-0000-7000-8000-000000000021",
      1002,
      "amide",
      1786000002100,
    ),
  ],
  gradebook: [
    {
      tenant,
      courseId,
      enrollmentId,
      studentId,
      learnerName: "Jordan Learner",
      assignmentId,
      assignmentTitle: "Peptide bond mastery",
      summary: {
        tenant,
        enrollment: enrollmentId,
        currentScore: 1,
        bestScore: 1,
        latestScore: 0,
        completedRunCount: 2,
        totalQuestionAttempts: 2,
        lastActivityAt: 1786000002200,
      },
      scoringStatus: "current",
    },
  ],
};
