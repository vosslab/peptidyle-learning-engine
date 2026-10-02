// Sysadmin Course creation assigns an Instructor. The Sysadmin does not teach the Course.

import type { CreatedCourseInstance, CreateCourseInstanceInput } from "../api/course_instance";
import type { CourseClassificationDraft } from "./course_classification_fields";
import { DecodeError } from "../api/decoder";
import { decodeCourseClassification } from "../api/decoders/course_classification";
import { decodeCreateCourseInstanceInput } from "../api/decoders/course_instance";
import { courseTermFitsActiveLifetime } from "../api/decoders/course_term";

export interface SysadminCourseCreationDraft {
  readonly assignedInstructorAccountId: string;
  readonly classification: CourseClassificationDraft;
  readonly shortName: string;
  readonly longName: string;
  readonly startDate: string;
  readonly endDate: string;
}

export interface SysadminCourseCreator {
  readonly createCourseInstance: (
    input: CreateCourseInstanceInput,
  ) => Promise<CreatedCourseInstance>;
}

export class CourseCreationInputError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "CourseCreationInputError";
  }
}

/**
 * Creates an empty Course Instance for the selected Instructor.
 * The returned receipt is not Course membership for the Sysadmin.
 */
export async function submitSysadminCourseCreation(
  client: SysadminCourseCreator,
  draft: SysadminCourseCreationDraft,
  createdOn: string,
): Promise<string> {
  if (draft.assignedInstructorAccountId.trim().length === 0) {
    throw new CourseCreationInputError(
      "Choose the Instructor Account ID that will teach this Course.",
    );
  }
  let input: CreateCourseInstanceInput;
  try {
    // ASVS 1.5.2/2.2.1: validate the Instructor, classification, names, and dates before transport.
    input = decodeCreateCourseInstanceInput({
      classification: decodeCourseClassification(draft.classification),
      source: { kind: "empty" },
      shortName: draft.shortName,
      longName: draft.longName,
      term: { startDate: draft.startDate, endDate: draft.endDate },
      assignedInstructorAccountId: draft.assignedInstructorAccountId,
    });
  } catch (error) {
    if (error instanceof DecodeError) {
      throw new CourseCreationInputError(
        "Choose a Discipline and an Instructor Account ID, and enter trimmed Course names and dates.",
      );
    }
    throw error;
  }
  if (input.assignedInstructorAccountId === undefined) {
    throw new CourseCreationInputError(
      "Choose the Instructor Account ID that will teach this Course.",
    );
  }
  if (!courseTermFitsActiveLifetime(input.term.endDate, createdOn)) {
    throw new CourseCreationInputError(
      "Choose an end date within six months of today. A Course remains Active for at most six months from creation.",
    );
  }
  const created = await client.createCourseInstance(input);
  if (created.courseInstance.lifecycleState !== "active") {
    throw new Error("A newly created Course Instance must be Active.");
  }
  return `Course ${created.courseInstance.shortName} was created. Instructor ${input.assignedInstructorAccountId} teaches it.`;
}
