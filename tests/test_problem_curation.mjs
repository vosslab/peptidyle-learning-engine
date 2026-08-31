import assert from "node:assert/strict";
import test from "node:test";

import {
  appendCollectionQuestionIds,
  catalogSearchFilterFromLibraryQuery,
  collectionDeletionFromObserved,
  curationEtagFromObservedEditNumber,
  libraryQueryFromSavedSearch,
  mayMutatePersonalCuration,
  mayEditOpenedQuestionCollection,
  moveCollectionQuestionId,
  problemCurationPickerSources,
  removeCollectionQuestionId,
  savedSearchDeletionFromObserved,
  savedSearchReplacementFromObserved,
} from "../src/features/problem_curation/problem_curation_model.ts";
import { decodeQuestionCollectionMemberPage } from "../src/api/decoders/problem_curation.ts";
import { EMPTY_CATALOG_QUERY } from "../src/pages/library_page_model.ts";
import { publishedProblemFixture } from "./fixtures/published_problem.ts";

test("collection edits retain a unique ordered public Question ID list", () => {
  const appended = appendCollectionQuestionIds(["ABC-1234", "DEF-5678"], ["DEF-5678", "GHJ-9KMP"]);
  const moved = moveCollectionQuestionId(appended, 2, -1);

  assert.deepEqual(removeCollectionQuestionId(moved, "DEF-5678"), ["ABC-1234", "GHJ-9KMP"]);
});

test("collection members carry the exact current Question Version Availability", () => {
  const member = {
    questionId: publishedProblemFixture.catalogProblem.questionId,
    summary: publishedProblemFixture.catalogProblem,
    questionVersionAvailability: { availability: "archived", reason: "Replaced by a correction." },
  };

  const decoded = decodeQuestionCollectionMemberPage({ items: [member], nextCursor: null });
  assert.deepEqual(decoded.items[0]?.questionVersionAvailability, member.questionVersionAvailability);
  assert.throws(
    () =>
      decodeQuestionCollectionMemberPage({
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
  const filter = catalogSearchFilterFromLibraryQuery({
    ...EMPTY_CATALOG_QUERY,
    search: "  kinase   pathway  ",
    backend: "native",
    evidence: "available",
  });

  assert.deepEqual(filter, {
    text: "kinase pathway",
    bylines: [],
    backends: ["native"],
    tags: [],
    question_types: [],
    taxonomy: [],
    capabilities: [],
    licenses: [],
    evidence: "available",
    used_in_my_courses: "any",
    authorship: "any",
  });
});

test("running a saved search starts with its current-catalog filters", () => {
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
      taxonomy: [{ scheme: "ncbi", code: "9606" }],
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
    taxonomy: "ncbi:9606",
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

test("private collection editing follows the authenticated Instructor capability", () => {
  assert.equal(mayEditOpenedQuestionCollection(false), false);
  assert.equal(mayEditOpenedQuestionCollection(true), true);
});

test("picker maps named private collections beside the current Library and authored catalog", () => {
  const sources = problemCurationPickerSources(
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
      ["catalog", "Current library"],
      ["mine", "My published questions"],
      ["collection", "Exam candidates"],
    ],
  );
});

test("saved-search replacement sends the edit number already observed with retained filter meaning", () => {
  const search = {
    reference: "QS-3",
    title: "Peptide candidates",
    editNumber: "3",
    filter: catalogSearchFilterFromLibraryQuery({
      ...EMPTY_CATALOG_QUERY,
      search: "peptide",
      backend: "native",
    }),
  };
  const replacement = savedSearchReplacementFromObserved(
    search,
    "Peptide exam candidates",
    EMPTY_CATALOG_QUERY,
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
  const collectionDeletion = collectionDeletionFromObserved({
    reference: "QC-7",
    title: "Exam candidates",
    editNumber: "7",
  });
  const savedDeletion = savedSearchDeletionFromObserved({
    reference: "QS-3",
    title: "Peptide candidates",
    editNumber: "3",
    filter: catalogSearchFilterFromLibraryQuery(EMPTY_CATALOG_QUERY),
  });

  assert.deepEqual(
    [collectionDeletion, savedDeletion].map((deletion) => ({
      heading: deletion.heading,
      consequence: deletion.consequence,
      editNumber: deletion.editNumber,
      confirmLabel: deletion.confirmLabel,
    })),
    [
      {
        heading: 'Delete collection "Exam candidates"?',
        consequence:
          "Deleting this collection removes its saved ordered question list. Published questions remain available in the Library.",
        editNumber: '"7"',
        confirmLabel: "Delete collection",
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
