// Single authority for the committed UI screenshot corpus in docs/screenshots/.

export type CorpusRole = "instructor" | "student" | "shared";
export type CorpusPipeline = "mock" | "live";
export type CorpusCaptureOwner = "instructorMock" | "studentMock" | "live";
export type CorpusViewport = "laptop" | "tablet" | "iphonePro" | "square";
export type CorpusLiveReason = "requiresRealGrading" | "requiresRendererOutput";
export type CorpusEvidencePurpose =
  | "layout"
  | "responsive"
  | "themeSystem"
  | "gradingState"
  | "rendererOutput"
  | "studentPerspective"
  | "accessBoundary";

export interface CorpusArtifact {
  /** Safe repository-relative identity, including the corpus and role directories. */
  readonly path: string;
  readonly viewport: CorpusViewport;
}

export interface CorpusSurface {
  readonly surface: string;
  readonly route: string;
  readonly role: CorpusRole;
  readonly pipeline: CorpusPipeline;
  readonly captureOwner: CorpusCaptureOwner;
  readonly evidencePurpose: CorpusEvidencePurpose;
  readonly liveReason?: CorpusLiveReason;
  /** Exact viewport matrix required for this surface's acceptance evidence. */
  readonly requiredViewports?: readonly CorpusViewport[];
  readonly artifacts: readonly CorpusArtifact[];
}

export const CORPUS_VIEWPORT_SIZES = {
  laptop: { width: 1_280, height: 800 },
  tablet: { width: 800, height: 1_280 },
  iphonePro: { width: 393, height: 852 },
  square: { width: 800, height: 800 },
} as const satisfies Readonly<Record<CorpusViewport, { width: number; height: number }>>;

export const CORPUS_DIRECTORY = "docs/screenshots";

function artifact(
  role: CorpusRole,
  relativePath: string,
  viewport: CorpusViewport = "laptop",
): CorpusArtifact {
  return { path: `${CORPUS_DIRECTORY}/${role}/${relativePath}`, viewport };
}

export const UI_CORPUS_MANIFEST = [
  {
    surface: "instructorCourses",
    route: "/",
    role: "instructor",
    pipeline: "mock",
    captureOwner: "instructorMock",
    evidencePurpose: "layout",
    artifacts: [artifact("instructor", "instructor_page_courses.png")],
  },
  {
    surface: "instructorCourseAssignments",
    route: "/courses/:courseRef",
    role: "instructor",
    pipeline: "mock",
    captureOwner: "instructorMock",
    evidencePurpose: "layout",
    artifacts: [artifact("instructor", "instructor_page_course_assignments.png")],
  },
  {
    surface: "assignmentOverview",
    route: "/courses/:courseRef/assignments/:assignmentRef",
    role: "instructor",
    pipeline: "mock",
    captureOwner: "instructorMock",
    evidencePurpose: "layout",
    artifacts: [artifact("instructor", "instructor_page_assignment_overview.png")],
  },
  {
    surface: "assignmentCreate",
    route: "/instructor/courses/:courseRef/assignments/new",
    role: "instructor",
    pipeline: "mock",
    captureOwner: "instructorMock",
    evidencePurpose: "layout",
    artifacts: [artifact("instructor", "instructor_page_assignment_create.png")],
  },
  {
    surface: "assignmentEditor",
    route: "/instructor/courses/:courseRef/assignments/:assignmentRef/edit",
    role: "instructor",
    pipeline: "mock",
    captureOwner: "instructorMock",
    evidencePurpose: "layout",
    artifacts: [artifact("instructor", "instructor_page_assignment_edit.png")],
  },
  {
    surface: "courseRoster",
    route: "/instructor/courses/:courseRef/students",
    role: "instructor",
    pipeline: "mock",
    captureOwner: "instructorMock",
    evidencePurpose: "layout",
    artifacts: [artifact("instructor", "instructor_page_roster.png")],
  },
  {
    surface: "gradebook",
    route: "/instructor/courses/:courseRef/gradebook",
    role: "instructor",
    pipeline: "mock",
    captureOwner: "instructorMock",
    evidencePurpose: "layout",
    artifacts: [artifact("instructor", "instructor_page_gradebook.png")],
  },
  {
    surface: "courseAppearance",
    route: "/instructor/courses/:courseRef/appearance",
    role: "instructor",
    pipeline: "mock",
    captureOwner: "instructorMock",
    evidencePurpose: "themeSystem",
    artifacts: [artifact("instructor", "instructor_page_course_appearance.png")],
  },
  {
    surface: "library",
    route: "/library",
    role: "instructor",
    pipeline: "mock",
    captureOwner: "instructorMock",
    evidencePurpose: "layout",
    artifacts: [artifact("instructor", "instructor_page_library.png")],
  },
  {
    surface: "problemDetail",
    route: "/library/:problemRef",
    role: "instructor",
    pipeline: "mock",
    captureOwner: "instructorMock",
    evidencePurpose: "layout",
    artifacts: [artifact("instructor", "instructor_page_question_detail.png")],
  },
  {
    surface: "workspaceList",
    route: "/workspace",
    role: "instructor",
    pipeline: "mock",
    captureOwner: "instructorMock",
    evidencePurpose: "layout",
    artifacts: [artifact("instructor", "instructor_page_workspace.png")],
  },
  {
    surface: "workspaceEditor",
    route: "/workspace/:workspaceRef",
    role: "instructor",
    pipeline: "mock",
    captureOwner: "instructorMock",
    evidencePurpose: "layout",
    artifacts: [artifact("instructor", "instructor_page_question_editor.png")],
  },
  {
    surface: "studentAllowedAssignmentOverview",
    route: "/courses/:courseRef/assignments/:assignmentRef",
    role: "student",
    pipeline: "mock",
    captureOwner: "studentMock",
    evidencePurpose: "studentPerspective",
    requiredViewports: ["laptop", "tablet", "iphonePro", "square"],
    artifacts: [
      artifact(
        "student",
        "access/allowed_assignment_overview/student_assignment_overview_laptop.png",
        "laptop",
      ),
      artifact(
        "student",
        "access/allowed_assignment_overview/student_assignment_overview_tablet.png",
        "tablet",
      ),
      artifact(
        "student",
        "access/allowed_assignment_overview/student_assignment_overview_iphone_pro.png",
        "iphonePro",
      ),
      artifact(
        "student",
        "access/allowed_assignment_overview/student_assignment_overview_square.png",
        "square",
      ),
    ],
  },
  {
    surface: "studentInstructorRouteDenial",
    route: "/instructor/courses/:courseRef/gradebook",
    role: "student",
    pipeline: "mock",
    captureOwner: "studentMock",
    evidencePurpose: "accessBoundary",
    requiredViewports: ["laptop", "tablet", "iphonePro", "square"],
    artifacts: [
      artifact(
        "student",
        "access/instructor_route_denial/student_instructor_route_denial_laptop.png",
        "laptop",
      ),
      artifact(
        "student",
        "access/instructor_route_denial/student_instructor_route_denial_tablet.png",
        "tablet",
      ),
      artifact(
        "student",
        "access/instructor_route_denial/student_instructor_route_denial_iphone_pro.png",
        "iphonePro",
      ),
      artifact(
        "student",
        "access/instructor_route_denial/student_instructor_route_denial_square.png",
        "square",
      ),
    ],
  },
  {
    surface: "accountSecurity",
    route: "/account/security",
    role: "shared",
    pipeline: "mock",
    captureOwner: "studentMock",
    evidencePurpose: "layout",
    artifacts: [artifact("shared", "instructor_page_account_security.png")],
  },
  {
    surface: "liveCourseOverview",
    route: "/courses/:courseRef",
    role: "instructor",
    pipeline: "live",
    captureOwner: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [artifact("instructor", "instructor_course_overview.png")],
  },
  {
    surface: "liveRosterActiveStudent",
    route: "/instructor/courses/:courseRef/students",
    role: "instructor",
    pipeline: "live",
    captureOwner: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [artifact("instructor", "instructor_roster_active_student.png")],
  },
  {
    surface: "liveProblemCatalog",
    route: "/instructor/courses/:courseRef/assignments/new",
    role: "instructor",
    pipeline: "live",
    captureOwner: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [artifact("instructor", "instructor_problem_catalog.png")],
  },
  {
    surface: "liveAssignmentSettings",
    route: "/instructor/courses/:courseRef/assignments/:assignmentRef/edit",
    role: "instructor",
    pipeline: "live",
    captureOwner: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [artifact("instructor", "instructor_assignment_settings.png")],
  },
  {
    surface: "liveAssignmentCreated",
    route: "/instructor/courses/:courseRef/assignments/:assignmentRef/edit",
    role: "instructor",
    pipeline: "live",
    captureOwner: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [artifact("instructor", "instructor_assignment_created.png")],
  },
  {
    surface: "liveGradebookMasteryLoop",
    route: "/instructor/courses/:courseRef/gradebook",
    role: "instructor",
    pipeline: "live",
    captureOwner: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [artifact("instructor", "instructor_gradebook_mastery_loop.png")],
  },
  {
    surface: "liveGeneticsChapterOneOverview",
    route: "/courses/:courseRef/assignments/:assignmentRef",
    role: "student",
    pipeline: "live",
    captureOwner: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [artifact("student", "genetics_chapter_one_overview.png")],
  },
  {
    surface: "liveStudentAssignmentList",
    route: "/courses/:courseRef",
    role: "student",
    pipeline: "live",
    captureOwner: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [artifact("student", "student_assignment_list.png")],
  },
  {
    surface: "liveStudentTimedProblem",
    route: "/runs/:runRef",
    role: "student",
    pipeline: "live",
    captureOwner: "live",
    liveReason: "requiresRendererOutput",
    evidencePurpose: "rendererOutput",
    artifacts: [artifact("student", "student_timed_problem.png")],
  },
  {
    surface: "liveStudentFreshPractice",
    route: "/runs/:runRef/summary",
    role: "student",
    pipeline: "live",
    captureOwner: "live",
    liveReason: "requiresRealGrading",
    evidencePurpose: "gradingState",
    artifacts: [artifact("student", "student_fresh_practice.png")],
  },
  {
    surface: "liveStudentRetakeFreshProblem",
    route: "/runs/:runRef",
    role: "student",
    pipeline: "live",
    captureOwner: "live",
    liveReason: "requiresRendererOutput",
    evidencePurpose: "rendererOutput",
    artifacts: [artifact("student", "student_retake_fresh_problem.png")],
  },
] as const satisfies ReadonlyArray<CorpusSurface>;

function allArtifacts(): ReadonlyArray<{
  readonly artifact: CorpusArtifact;
  readonly surface: CorpusSurface;
}> {
  return UI_CORPUS_MANIFEST.flatMap((surface) =>
    surface.artifacts.map((surfaceArtifact) => ({ artifact: surfaceArtifact, surface })),
  );
}

/** Reject absolute, traversal, non-PNG, or misplaced role paths. */
export function validateCorpusArtifactPath(artifactPath: string, role?: CorpusRole): void {
  const prefix = `${CORPUS_DIRECTORY}/`;
  if (
    artifactPath.startsWith("/") ||
    artifactPath.includes("\\") ||
    !artifactPath.startsWith(prefix) ||
    artifactPath.split("/").some((part) => part === "" || part === "." || part === "..")
  ) {
    throw new Error(`unsafe corpus artifact path: ${artifactPath}`);
  }
  const relativeParts = artifactPath.slice(prefix.length).split("/");
  if (
    relativeParts.length < 2 ||
    !["instructor", "student", "shared"].includes(relativeParts[0] ?? "")
  ) {
    throw new Error(`corpus artifact path must use a role directory: ${artifactPath}`);
  }
  if (role !== undefined && relativeParts[0] !== role) {
    throw new Error(`corpus artifact path does not match role ${role}: ${artifactPath}`);
  }
  const nestedDirectories = relativeParts.slice(1, -1);
  if (nestedDirectories.some((part) => !/^[a-z0-9_]+$/u.test(part))) {
    throw new Error(`corpus artifact directory is invalid: ${artifactPath}`);
  }
  const basename = relativeParts[relativeParts.length - 1];
  if (!/^[a-z0-9_]+\.png$/u.test(basename ?? "")) {
    throw new Error(`corpus artifact basename is invalid: ${artifactPath}`);
  }
}

function validateManifest(): void {
  const paths = new Set<string>();
  const basenames = new Set<string>();
  for (const { artifact: surfaceArtifact, surface } of allArtifacts()) {
    validateCorpusArtifactPath(surfaceArtifact.path, surface.role);
    const basename = surfaceArtifact.path.slice(surfaceArtifact.path.lastIndexOf("/") + 1);
    if (paths.has(surfaceArtifact.path) || basenames.has(basename)) {
      throw new Error(`duplicate corpus artifact identity: ${surfaceArtifact.path}`);
    }
    if ((surface.pipeline === "live") !== (surface.captureOwner === "live")) {
      throw new Error(`surface pipeline and capture owner disagree: ${surface.surface}`);
    }
    if (surface.requiredViewports !== undefined) {
      const required = new Set<CorpusViewport>(surface.requiredViewports);
      const present = new Set<CorpusViewport>(
        surface.artifacts.map((artifact) => artifact.viewport),
      );
      if (
        required.size !== surface.requiredViewports.length ||
        present.size !== surface.artifacts.length ||
        required.size !== present.size ||
        surface.requiredViewports.some((viewport) => !present.has(viewport))
      ) {
        throw new Error(
          `surface does not contain its exact required viewport matrix: ${surface.surface}`,
        );
      }
    }
    paths.add(surfaceArtifact.path);
    basenames.add(basename);
  }
}

validateManifest();

export function expectedViewportsForRole(role: CorpusRole): readonly CorpusViewport[] {
  if (role === "student") return ["laptop", "tablet"];
  return ["laptop"];
}

export function manifestArtifactPaths(): readonly string[] {
  return allArtifacts().map(({ artifact: surfaceArtifact }) => surfaceArtifact.path);
}

export function artifactPathsForPipeline(pipeline: CorpusPipeline): readonly string[] {
  return UI_CORPUS_MANIFEST.filter((surface) => surface.pipeline === pipeline).flatMap((surface) =>
    surface.artifacts.map((surfaceArtifact) => surfaceArtifact.path),
  );
}

export function artifactPathsForCaptureOwner(owner: CorpusCaptureOwner): readonly string[] {
  return UI_CORPUS_MANIFEST.filter((surface) => surface.captureOwner === owner).flatMap((surface) =>
    surface.artifacts.map((surfaceArtifact) => surfaceArtifact.path),
  );
}

export function surfaceForArtifact(artifactPath: string): CorpusSurface | undefined {
  return UI_CORPUS_MANIFEST.find((surface) =>
    surface.artifacts.some((surfaceArtifact) => surfaceArtifact.path === artifactPath),
  );
}

export function surfaceByName(surfaceName: string): CorpusSurface | undefined {
  return UI_CORPUS_MANIFEST.find((surface) => surface.surface === surfaceName);
}

export function artifactPathForBasename(basename: string): string | undefined {
  return manifestArtifactPaths().find((artifactPath) => artifactPath.endsWith(`/${basename}`));
}

export function viewportForArtifact(artifactPath: string): CorpusViewport | undefined {
  for (const surface of UI_CORPUS_MANIFEST) {
    const surfaceArtifact = surface.artifacts.find((candidate) => candidate.path === artifactPath);
    if (surfaceArtifact !== undefined) return surfaceArtifact.viewport;
  }
  return undefined;
}

export function captureOwnerForArtifact(artifactPath: string): CorpusCaptureOwner | undefined {
  return surfaceForArtifact(artifactPath)?.captureOwner;
}

export function surfacesMissingExpectedViewports(): ReadonlyArray<{
  readonly surface: string;
  readonly missing: readonly CorpusViewport[];
}> {
  return UI_CORPUS_MANIFEST.map((surface) => {
    const present = new Set<CorpusViewport>(
      surface.artifacts.map((surfaceArtifact) => surfaceArtifact.viewport),
    );
    const missing = expectedViewportsForRole(surface.role).filter(
      (viewport) => !present.has(viewport),
    );
    return { surface: surface.surface, missing };
  }).filter((entry) => entry.missing.length > 0);
}

export function surfacesWithRequiredViewports(): readonly CorpusSurface[] {
  const required: CorpusSurface[] = [];
  for (const manifestSurface of UI_CORPUS_MANIFEST) {
    const surface: CorpusSurface = manifestSurface;
    if (surface.requiredViewports !== undefined) required.push(surface);
  }
  return required;
}
