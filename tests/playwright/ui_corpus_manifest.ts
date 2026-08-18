// Single authority for the committed UI screenshot corpus in docs/screenshots/.
//
// Both capture scripts read this manifest instead of holding their own name lists, so every
// committed artifact has exactly one owning surface and one owning pipeline. The corpus previously
// had two independent owners and no shared declaration, which allowed one pipeline's images to age
// out of step with the other's and left one artifact owned by neither.

/** Who the surface is composed for. */
export type CorpusRole = "instructor" | "student" | "shared";

/** Which capture implementation owns the surface. */
export type CorpusPipeline = "mock" | "live";

/** Named viewport targets from the role-based visual contract in docs/HUMAN_GUIDANCE.md. */
export type CorpusViewport = "laptop" | "tablet" | "phone";

/**
 * Why a surface needs the expensive live pipeline.
 *
 * Live capture serves claims that depend on runtime state. Every other surface uses deterministic
 * mock capture, which runs without container infrastructure.
 */
export type CorpusLiveReason = "requiresRealGrading" | "requiresRendererOutput";

/** What claim the artifact supports, so a later reviewer reads the intent directly. */
export type CorpusEvidencePurpose =
  "layout" | "responsive" | "themeSystem" | "gradingState" | "rendererOutput";

/** One committed image: a surface captured at one viewport. */
export interface CorpusArtifact {
  readonly name: string;
  readonly viewport: CorpusViewport;
}

/** One logical page, captured at one or more viewports. */
export interface CorpusSurface {
  readonly surface: string;
  readonly route: string;
  readonly role: CorpusRole;
  readonly pipeline: CorpusPipeline;
  readonly evidencePurpose: CorpusEvidencePurpose;
  readonly liveReason?: CorpusLiveReason;
  readonly artifacts: readonly CorpusArtifact[];
}

/** CSS-pixel sizes for each named viewport. */
export const CORPUS_VIEWPORT_SIZES = {
  laptop: { width: 1_280, height: 800 },
  tablet: { width: 800, height: 1_280 },
  phone: { width: 390, height: 844 },
} as const satisfies Readonly<Record<CorpusViewport, { width: number; height: number }>>;

/** Repository-relative directory holding the committed corpus. */
export const CORPUS_DIRECTORY = "docs/screenshots";

export const UI_CORPUS_MANIFEST = [
  {
    surface: "instructorCourses",
    route: "/",
    role: "instructor",
    pipeline: "mock",
    evidencePurpose: "layout",
    artifacts: [{ name: "instructor_page_courses.png", viewport: "laptop" }],
  },
  {
    surface: "instructorCourseAssignments",
    route: "/courses/:courseRef",
    role: "instructor",
    pipeline: "mock",
    evidencePurpose: "layout",
    artifacts: [{ name: "instructor_page_course_assignments.png", viewport: "laptop" }],
  },
  {
    surface: "assignmentOverview",
    route: "/courses/:courseRef/assignments/:assignmentRef",
    role: "student",
    pipeline: "mock",
    evidencePurpose: "layout",
    artifacts: [{ name: "instructor_page_assignment_overview.png", viewport: "laptop" }],
  },
  {
    surface: "assignmentCreate",
    route: "/instructor/courses/:courseRef/assignments/new",
    role: "instructor",
    pipeline: "mock",
    evidencePurpose: "layout",
    artifacts: [{ name: "instructor_page_assignment_create.png", viewport: "laptop" }],
  },
  {
    surface: "assignmentEditor",
    route: "/instructor/courses/:courseRef/assignments/:assignmentRef/edit",
    role: "instructor",
    pipeline: "mock",
    evidencePurpose: "layout",
    artifacts: [{ name: "instructor_page_assignment_edit.png", viewport: "laptop" }],
  },
  {
    surface: "courseRoster",
    route: "/instructor/courses/:courseRef/students",
    role: "instructor",
    pipeline: "mock",
    evidencePurpose: "layout",
    artifacts: [{ name: "instructor_page_roster.png", viewport: "laptop" }],
  },
  {
    surface: "gradebook",
    route: "/instructor/courses/:courseRef/gradebook",
    role: "instructor",
    pipeline: "mock",
    evidencePurpose: "layout",
    artifacts: [{ name: "instructor_page_gradebook.png", viewport: "laptop" }],
  },
  {
    surface: "courseAppearance",
    route: "/instructor/courses/:courseRef/appearance",
    role: "instructor",
    pipeline: "mock",
    evidencePurpose: "themeSystem",
    artifacts: [{ name: "instructor_page_course_appearance.png", viewport: "laptop" }],
  },
  {
    surface: "library",
    route: "/library",
    role: "instructor",
    pipeline: "mock",
    evidencePurpose: "layout",
    artifacts: [{ name: "instructor_page_library.png", viewport: "laptop" }],
  },
  {
    surface: "problemDetail",
    route: "/library/:problemRef",
    role: "instructor",
    pipeline: "mock",
    evidencePurpose: "layout",
    artifacts: [{ name: "instructor_page_question_detail.png", viewport: "laptop" }],
  },
  {
    surface: "workspaceList",
    route: "/workspace",
    role: "instructor",
    pipeline: "mock",
    evidencePurpose: "layout",
    artifacts: [{ name: "instructor_page_workspace.png", viewport: "laptop" }],
  },
  {
    surface: "workspaceEditor",
    route: "/workspace/:workspaceRef",
    role: "instructor",
    pipeline: "mock",
    evidencePurpose: "layout",
    artifacts: [{ name: "instructor_page_question_editor.png", viewport: "laptop" }],
  },
  {
    surface: "accountSecurity",
    route: "/account/security",
    role: "shared",
    pipeline: "mock",
    evidencePurpose: "layout",
    artifacts: [{ name: "instructor_page_account_security.png", viewport: "laptop" }],
  },
  {
    surface: "liveCourseOverview",
    route: "/courses/:courseRef",
    role: "instructor",
    pipeline: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [{ name: "instructor_course_overview.png", viewport: "laptop" }],
  },
  {
    surface: "liveRosterActiveStudent",
    route: "/instructor/courses/:courseRef/students",
    role: "instructor",
    pipeline: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [{ name: "instructor_roster_active_student.png", viewport: "laptop" }],
  },
  {
    surface: "liveProblemCatalog",
    route: "/instructor/courses/:courseRef/assignments/new",
    role: "instructor",
    pipeline: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [{ name: "instructor_problem_catalog.png", viewport: "laptop" }],
  },
  {
    surface: "liveAssignmentSettings",
    route: "/instructor/courses/:courseRef/assignments/:assignmentRef/edit",
    role: "instructor",
    pipeline: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [{ name: "instructor_assignment_settings.png", viewport: "laptop" }],
  },
  {
    surface: "liveAssignmentCreated",
    route: "/instructor/courses/:courseRef/assignments/:assignmentRef/edit",
    role: "instructor",
    pipeline: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [{ name: "instructor_assignment_created.png", viewport: "laptop" }],
  },
  {
    surface: "liveGradebookMasteryLoop",
    route: "/instructor/courses/:courseRef/gradebook",
    role: "instructor",
    pipeline: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [{ name: "instructor_gradebook_mastery_loop.png", viewport: "laptop" }],
  },
  {
    surface: "liveGeneticsChapterOneOverview",
    route: "/courses/:courseRef/assignments/:assignmentRef",
    role: "student",
    pipeline: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [{ name: "genetics_chapter_one_overview.png", viewport: "laptop" }],
  },
  {
    surface: "liveStudentAssignmentList",
    route: "/courses/:courseRef",
    role: "student",
    pipeline: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [{ name: "student_assignment_list.png", viewport: "laptop" }],
  },
  {
    surface: "liveStudentTimedProblem",
    route: "/runs/:runRef",
    role: "student",
    pipeline: "live",
    liveReason: "requiresRendererOutput",
    evidencePurpose: "rendererOutput",
    artifacts: [{ name: "student_timed_problem.png", viewport: "laptop" }],
  },
  {
    surface: "liveStudentFreshPractice",
    route: "/runs/:runRef/summary",
    role: "student",
    pipeline: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [{ name: "student_fresh_practice.png", viewport: "laptop" }],
  },
  {
    surface: "liveStudentRetakeFreshProblem",
    route: "/runs/:runRef",
    role: "student",
    pipeline: "live",
    liveReason: "requiresRendererOutput",
    evidencePurpose: "rendererOutput",
    artifacts: [{ name: "student_retake_fresh_problem.png", viewport: "laptop" }],
  },
] as const satisfies ReadonlyArray<CorpusSurface>;

/**
 * Viewports the role-based visual contract expects for a role.
 *
 * Instructors work at the canonical laptop viewport. Students are composed for that same laptop
 * viewport and for the high-priority tablet target. The narrow phone is a single compatibility
 * artifact rather than a per-surface pass, so it is requested explicitly instead of by role.
 */
export function expectedViewportsForRole(role: CorpusRole): readonly CorpusViewport[] {
  if (role === "student") return ["laptop", "tablet"];
  return ["laptop"];
}

/** Every artifact name the manifest declares. */
export function manifestArtifactNames(): readonly string[] {
  const names = UI_CORPUS_MANIFEST.flatMap((surface) =>
    surface.artifacts.map((artifact) => artifact.name),
  );
  return names;
}

/** Artifact names owned by one pipeline. */
export function artifactNamesForPipeline(pipeline: CorpusPipeline): readonly string[] {
  const names = UI_CORPUS_MANIFEST.filter((surface) => surface.pipeline === pipeline).flatMap(
    (surface) => surface.artifacts.map((artifact) => artifact.name),
  );
  return names;
}

/** The surface owning one artifact name, when the manifest declares it. */
export function surfaceForArtifact(name: string): CorpusSurface | undefined {
  const owner = UI_CORPUS_MANIFEST.find((surface) =>
    surface.artifacts.some((artifact) => artifact.name === name),
  );
  return owner;
}

/** The viewport one artifact was captured at, when the manifest declares it. */
export function viewportForArtifact(name: string): CorpusViewport | undefined {
  for (const surface of UI_CORPUS_MANIFEST) {
    const artifact = surface.artifacts.find((candidate) => candidate.name === name);
    if (artifact !== undefined) return artifact.viewport;
  }
  return undefined;
}

/**
 * Surfaces whose committed artifacts do not yet cover every viewport their role expects.
 *
 * This reports coverage the visual contract calls for and the corpus has yet to supply, so the gap
 * is visible as evidence rather than discovered by a reader.
 */
export function surfacesMissingExpectedViewports(): ReadonlyArray<{
  readonly surface: string;
  readonly missing: readonly CorpusViewport[];
}> {
  const gaps = UI_CORPUS_MANIFEST.map((surface) => {
    const present = new Set<CorpusViewport>(surface.artifacts.map((artifact) => artifact.viewport));
    const missing = expectedViewportsForRole(surface.role).filter(
      (viewport) => !present.has(viewport),
    );
    return { surface: surface.surface, missing };
  }).filter((entry) => entry.missing.length > 0);
  return gaps;
}
