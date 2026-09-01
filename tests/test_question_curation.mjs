import assert from "node:assert/strict";
import test from "node:test";

import {
  appendFolderQuestionIds,
  questionSearchFilterFromLibraryQuery,
  folderDeletionFromObserved,
  curationEtagFromObservedEditNumber,
  libraryQueryFromSavedSearch,
  mayMutatePersonalCuration,
  mayEditOpenedQuestionFolder,
  moveFolderQuestionId,
  questionCurationPickerSources,
  removeFolderQuestionId,
  savedSearchDeletionFromObserved,
  savedSearchReplacementFromObserved,
} from "../src/features/question_curation/question_curation_model.ts";
import { decodeQuestionFolderEntryPage } from "../src/api/decoders/question_curation.ts";
import { EMPTY_QUESTION_SEARCH_QUERY } from "../src/pages/library_page_model.ts";
import { publishedProblemFixture } from "./fixtures/published_problem.ts";

test("Question Folder edits retain a unique ordered public Question ID list", () => {
  const appended = appendFolderQuestionIds(["ABC-1234", "DEF-5678"], ["DEF-5678", "GHJ-9KMP"]);
  const moved = moveFolderQuestionId(appended, 2, -1);

  assert.deepEqual(removeFolderQuestionId(moved, "DEF-5678"), ["ABC-1234", "GHJ-9KMP"]);
});

test("Question Folder Entries carry the exact current Question Revision Availability", () => {
  const member = {
    questionId: publishedProblemFixture.publishedQuestion.questionId,
    summary: publishedProblemFixture.publishedQuestion,
    questionRevisionAvailability: { availability: "archived", reason: "Replaced by a correction." },
  };

  const decoded = decodeQuestionFolderEntryPage({ items: [member], nextCursor: null });
  assert.deepEqual(
    decoded.items[0]?.questionRevisionAvailability,
    member.questionRevisionAvailability,
  );
  assert.throws(
    () =>
      decodeQuestionFolderEntryPage({
        items: [
          {
            ...member,
            selectionAvailability: "retained",
          },
        ],
        nextCursor: null,
      }),
    /allowed by this response contract/u,
  );
});

test("saved search filter keeps the current normalized Library meaning", () => {
  const filter = questionSearchFilterFromLibraryQuery({
    ...EMPTY_QUESTION_SEARCH_QUERY,
    search: "  kinase   pathway  ",
    backend: "ple",
    evidence: "available",
  });

  assert.deepEqual(filter, {
    text: "kinase pathway",
    bylines: [],
    backends: ["ple"],
    tags: [],
    question_types: [],
    classifications: [],
    capabilities: [],
    licenses: [],
    evidence: "available",
    used_in_my_courses: "any",
    authorship: "any",
  });
});

test("running a saved search starts with its current Question Library filters", () => {
  const query = libraryQueryFromSavedSearch({
    reference: "QS-7",
    title: "Molecular genetics",
    editNumber: "3",
    filter: {
      text: "genetics",
      bylines: ["Lab team"],
      backends: ["qti"],
      tags: ["inheritance"],
      question_types: ["multipleChoice"],
      classifications: [{ system: "ncbi", code: "9606" }],
      capabilities: ["serverGrading"],
      licenses: ["ccBy"],
      evidence: "available",
      used_in_my_courses: "used",
      authorship: "any",
    },
  });

  assert.deepEqual(query, {
    search: "genetics",
    byline: "Lab team",
    backend: "qti",
    tag: "inheritance",
    questionType: "multipleChoice",
    classification: "ncbi:9606",
    capability: "serverGrading",
    license: "ccBy",
    evidence: "available",
    usedInMyCourses: "used",
    authorship: "any",
  });
});

test("only an authenticated Instructor mode enables personal curation", () => {
  assert.equal(
    mayMutatePersonalCuration({
      authenticated: true,
      account: { id: "A-1", role: "instructor" },
    }),
    true,
  );
  assert.equal(
    mayMutatePersonalCuration({
      authenticated: true,
      account: { id: "A-2", role: "sysadmin" },
    }),
    false,
  );
});

test("private Question Folder editing follows the authenticated Instructor capability", () => {
  assert.equal(mayEditOpenedQuestionFolder(false), false);
  assert.equal(mayEditOpenedQuestionFolder(true), true);
});

test("picker maps named private Question Folders beside the Question Library and My Questions", () => {
  const sources = questionCurationPickerSources(
    [
      {
        reference: "QC-2",
        title: "Exam candidates",
        editNumber: "7",
      },
    ],
    true,
  );

  assert.deepEqual(
    sources.map((source) => [source.kind, source.label]),
    [
      ["library", "Current library"],
      ["mine", "My Questions"],
      ["folder", "Exam candidates"],
    ],
  );
});

test("saved-search replacement sends the edit number already observed with retained filter meaning", () => {
  const search = {
    reference: "QS-3",
    title: "Peptide candidates",
    editNumber: "3",
    filter: questionSearchFilterFromLibraryQuery({
      ...EMPTY_QUESTION_SEARCH_QUERY,
      search: "peptide",
      backend: "ple",
    }),
  };
  const replacement = savedSearchReplacementFromObserved(
    search,
    "Peptide exam candidates",
    EMPTY_QUESTION_SEARCH_QUERY,
  );

  assert.deepEqual(
    {
      reference: replacement.reference,
      editNumber: replacement.editNumber,
      filter: replacement.filter,
    },
    { reference: "QS-3", editNumber: '"3"', filter: search.filter },
  );
});

test("named deletion confirmations carry visible names, consequences, and observed edit numbers", () => {
  const folderDeletion = folderDeletionFromObserved({
    reference: "QC-7",
    title: "Exam candidates",
    editNumber: "7",
  });
  const savedDeletion = savedSearchDeletionFromObserved({
    reference: "QS-3",
    title: "Peptide candidates",
    editNumber: "3",
    filter: questionSearchFilterFromLibraryQuery(EMPTY_QUESTION_SEARCH_QUERY),
  });

  assert.deepEqual(
    [folderDeletion, savedDeletion].map((deletion) => ({
      heading: deletion.heading,
      consequence: deletion.consequence,
      editNumber: deletion.editNumber,
      confirmLabel: deletion.confirmLabel,
    })),
    [
      {
        heading: 'Delete Question Folder "Exam candidates"?',
        consequence:
          "Deleting this Question Folder removes its saved ordered Question list. Published Questions remain available in the Question Library.",
        editNumber: '"7"',
        confirmLabel: "Delete Question Folder",
      },
      {
        heading: 'Delete saved search "Peptide candidates"?',
        consequence:
          "Deleting this saved search removes the shortcut. Current Library questions and filters remain available.",
        editNumber: '"3"',
        confirmLabel: "Delete saved search",
      },
    ],
  );
});

test("curation ETags accept only strong positive observed edit numbers", () => {
  assert.equal(curationEtagFromObservedEditNumber("12"), '"12"');
  assert.throws(() => curationEtagFromObservedEditNumber("012"), /current positive/u);
});
