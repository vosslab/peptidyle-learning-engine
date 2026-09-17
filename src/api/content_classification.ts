// Identity-backed global classification selectors and Sysadmin lifecycle capability.

export interface ContentClassificationItem {
  readonly uuid: string;
  readonly name: string;
  /** Retired Disciplines remain visible for discovery and existing references. */
  readonly isRetired: boolean;
}

export interface ContentClassificationClient {
  readonly listDisciplines: () => Promise<ReadonlyArray<ContentClassificationItem>>;
  /** Discovery preserves retired values rather than making existing content unreadable. */
  readonly listDisciplinesIncludingRetired: () => Promise<ReadonlyArray<ContentClassificationItem>>;
  readonly listSubjects: (
    disciplineUuid: string,
  ) => Promise<ReadonlyArray<ContentClassificationItem>>;
  readonly listTopics: (subjectUuid: string) => Promise<ReadonlyArray<ContentClassificationItem>>;
  readonly listSubtopics: (topicUuid: string) => Promise<ReadonlyArray<ContentClassificationItem>>;
}

/** Sysadmin-only mutation boundary; names remain server-normalized. */
export interface ContentDisciplineAdministrationClient {
  readonly createDiscipline: (name: string) => Promise<ContentClassificationItem>;
  readonly renameDiscipline: (uuid: string, name: string) => Promise<ContentClassificationItem>;
  readonly retireDiscipline: (uuid: string) => Promise<ContentClassificationItem>;
  readonly restoreDiscipline: (uuid: string) => Promise<ContentClassificationItem>;
}
