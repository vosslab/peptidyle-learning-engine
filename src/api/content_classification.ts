// Identity-backed global classification selectors and Sysadmin lifecycle capability.

export interface ContentClassificationItem {
  readonly uuid: string;
  readonly name: string;
  /** Retired Disciplines remain visible for discovery and existing references. */
  readonly isRetired: boolean;
}

/** A Subject create either selects the new row or offers an existing global name. */
export interface ContentSubjectCreation {
  readonly uuid: string;
  readonly name: string;
  readonly needsAcceptance: boolean;
}

/** A stored request for a Discipline. The requester is an Account ID, not an email. */
export interface ContentDisciplineRequest {
  readonly uuid: string;
  readonly requestedName: string;
  readonly requestedByAccountId: string;
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
  /** Records a name for a Sysadmin to add. This does not create the Discipline. */
  readonly requestContentDiscipline: (name: string) => Promise<ContentDisciplineRequest>;
  /** Creates a Subject in the selected Discipline, or offers an existing global name. */
  readonly createSubject: (name: string, disciplineUuid: string) => Promise<ContentSubjectCreation>;
  /** Associates an offered Subject after the Instructor accepts it. */
  readonly acceptSubjectDiscipline: (
    subjectUuid: string,
    disciplineUuid: string,
  ) => Promise<ContentClassificationItem>;
  readonly createTopic: (name: string, subjectUuid: string) => Promise<ContentClassificationItem>;
  readonly createSubtopic: (name: string, topicUuid: string) => Promise<ContentClassificationItem>;
}

/** Sysadmin-only mutation boundary; names remain server-normalized. */
export interface ContentDisciplineAdministrationClient {
  readonly createDiscipline: (name: string) => Promise<ContentClassificationItem>;
  readonly renameDiscipline: (uuid: string, name: string) => Promise<ContentClassificationItem>;
  readonly retireDiscipline: (uuid: string) => Promise<ContentClassificationItem>;
  readonly restoreDiscipline: (uuid: string) => Promise<ContentClassificationItem>;
  /** Open Discipline requests, each labeled by the requester Account ID. */
  readonly listOpenDisciplineRequests: () => Promise<ReadonlyArray<ContentDisciplineRequest>>;
  /** Closes one request after the Sysadmin creates the Discipline or dismisses the name. */
  readonly resolveDisciplineRequest: (uuid: string) => Promise<void>;
  /** Creates the requested Discipline and closes the request atomically. */
  readonly fulfillDisciplineRequest: (uuid: string) => Promise<ContentClassificationItem>;
}
