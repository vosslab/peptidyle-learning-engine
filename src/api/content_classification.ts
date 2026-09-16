// Read-only identity-backed global classification selectors.

export interface ContentClassificationItem {
  readonly uuid: string;
  readonly name: string;
}

export interface ContentClassificationClient {
  readonly listDisciplines: () => Promise<ReadonlyArray<ContentClassificationItem>>;
  readonly listSubjects: (
    disciplineUuid: string,
  ) => Promise<ReadonlyArray<ContentClassificationItem>>;
  readonly listTopics: (subjectUuid: string) => Promise<ReadonlyArray<ContentClassificationItem>>;
  readonly listSubtopics: (topicUuid: string) => Promise<ReadonlyArray<ContentClassificationItem>>;
}
