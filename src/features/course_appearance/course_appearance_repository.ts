// course_appearance_repository.ts - narrow adapter over the shared same-origin API client.

import type { CourseAppearance } from "../../../generated/api/CourseAppearance";
import type { CourseAppearanceUpdate } from "../../../generated/api/CourseAppearanceUpdate";
import type { CourseBannerCandidateReceipt } from "../../../generated/api/CourseBannerCandidateReceipt";
import type { CourseId } from "../../../generated/api/CourseId";
import type { ApiClient } from "../../api/client";

export interface CourseAppearanceRepository {
  readonly load: (course: CourseId) => Promise<CourseAppearance>;
  readonly uploadBanner: (course: CourseId, image: Blob) => Promise<CourseBannerCandidateReceipt>;
  readonly save: (
    course: CourseId,
    update: CourseAppearanceUpdate,
    revision: string,
  ) => Promise<CourseAppearance>;
}

/** Keeps transport details out of the instructor component and its local state. */
export function createCourseAppearanceRepository(
  client: Pick<
    ApiClient,
    "getCourseAppearance" | "uploadCourseBannerCandidate" | "saveCourseAppearance"
  >,
): CourseAppearanceRepository {
  return {
    load: async (course) => await client.getCourseAppearance(course),
    uploadBanner: async (course, image) => await client.uploadCourseBannerCandidate(course, image),
    save: async (course, update, revision) =>
      await client.saveCourseAppearance(course, update, revision),
  };
}
