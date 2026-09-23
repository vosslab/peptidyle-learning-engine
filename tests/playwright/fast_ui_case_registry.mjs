// Named current-source fixtures for the stack-free fast UI lane.

export const FAST_UI_CASES = {
  "record-list-ready": {
    kind: "primitive",
    source: "src/components/record_list/record_list.tsx",
    fixture: "primitive-ready",
    parityEligible: false,
    viewportCoverage: "laptop, tablet, phone, square",
  },
  "record-list-empty": {
    kind: "primitive",
    source: "src/components/record_list/record_list.tsx",
    fixture: "primitive-empty",
    parityEligible: false,
    viewportCoverage: "laptop, tablet, phone, square",
  },
  "record-list-loading": {
    kind: "primitive",
    source: "src/components/record_list/record_list.tsx",
    fixture: "primitive-loading",
    parityEligible: false,
    viewportCoverage: "laptop, tablet, phone, square",
  },
  "record-list-error": {
    kind: "primitive",
    source: "src/components/record_list/record_list.tsx",
    fixture: "primitive-error",
    parityEligible: false,
    viewportCoverage: "laptop, tablet, phone, square",
  },
  "provided-avatar-picker": {
    kind: "production-consumer",
    source: "src/features/profile_avatar/provided_avatar_picker.tsx",
    entrypoint: "provided-avatar-picker-harness",
    dataSeam: "selected avatar signal",
    parityEligible: false,
    viewportCoverage: "laptop, tablet, phone, square",
  },
  "page-frame-reading": {
    kind: "route-composition",
    source: "src/pages/student_course_landing_page.tsx",
    entrypoint: "student-course-entry-m6-harness",
    mode: "landing",
    dataSeam: "student current-Course projection",
    parityEligible: true,
    viewportCoverage: "laptop",
  },
  "page-frame-full-width": {
    kind: "route-composition",
    source: "src/pages/gradebook_page.tsx",
    entrypoint: "current-production-app",
    pathname: "/instructor/courses/CI7K3M2QAZ/gradebook",
    dataSeam: "application API Gradebook projection",
    parityEligible: true,
    viewportCoverage: "laptop",
  },
  "student-coursework": {
    kind: "route-composition",
    source: "src/pages/student_course_landing_page.tsx",
    entrypoint: "student-course-entry-m6-harness",
    mode: "landing",
    dataSeam: "student current-Course projection",
    parityEligible: true,
    viewportCoverage: "laptop, phone",
  },
  "question-library": {
    kind: "route-composition",
    source: "src/pages/library_page.tsx",
    entrypoint: "current-production-app",
    pathname: "/library/browse?tag=protein",
    dataSeam: "QuestionLibraryBrowseRepository",
    parityEligible: true,
    viewportCoverage: "laptop, production windowing",
  },
};

export function fastUiCase(name) {
  const entry = FAST_UI_CASES[name];
  if (entry === undefined) {
    const choices = Object.keys(FAST_UI_CASES).join(", ");
    throw new Error(`Unknown fast UI case "${name}". Choose one of: ${choices}.`);
  }
  return entry;
}
