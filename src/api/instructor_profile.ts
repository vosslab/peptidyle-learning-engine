export interface InstructorProfile {
  readonly timeZone: string;
}

export interface UpdateInstructorProfileInput {
  readonly timeZone: string;
}

export interface InstructorProfileThumbnail {
  readonly reference: string | null;
}
