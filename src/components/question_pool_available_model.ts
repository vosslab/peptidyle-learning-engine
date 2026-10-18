// Pool-owned discovery scope; the Library still owns query execution and paging.

import type { QuestionPickerEligibility } from "../features/question_picker";
import type {
  QuestionLibraryBrowseQuery,
  QuestionLibraryBrowseRepository,
} from "../pages/library_page_model";

const memberLicenses = ["CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0"];

/** Keep Pool eligibility in the complete remote query, including facet requests. */
export function questionPoolAvailableQuery(
  query: QuestionLibraryBrowseQuery,
  eligibility: QuestionPickerEligibility,
): QuestionLibraryBrowseQuery {
  // ASVS 2.2.1: retain fixed domain constraints before the Library counts/pages results.
  return {
    ...query,
    kind: "questions",
    questions: "all",
    discipline_uuids: [eligibility.disciplineUuid],
    subject_uuids: [eligibility.subjectUuid],
    cross_discipline: false,
    backends: [eligibility.backend],
    questionTypes: [eligibility.questionType],
    questionLicenses: query.questionLicenses.length === 0 ? memberLicenses : query.questionLicenses,
  };
}

/** Membership never excludes matching Library records or changes this search snapshot. */
export function questionPoolAvailableRepository(
  library: QuestionLibraryBrowseRepository,
  eligibility: QuestionPickerEligibility,
): QuestionLibraryBrowseRepository {
  return {
    search: (query, page) => library.search(questionPoolAvailableQuery(query, eligibility), page),
    listFacetChoices: (field, query, search, cursor) =>
      library.listFacetChoices(
        field,
        questionPoolAvailableQuery(query, eligibility),
        search,
        cursor,
      ),
  };
}
