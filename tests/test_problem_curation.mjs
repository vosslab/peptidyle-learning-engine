import assert from "node:assert/strict";
import test from "node:test";

import {
  appendCollectionQuestionIds,
  catalogSearchFilterFromLibraryQuery,
  collectionDeletionFromObserved,
  curationEtagFromObservedRevision,
  libraryQueryFromSavedSearch,
  mayMutatePersonalCuration,
  mayEditOpenedProblemCollection,
  moveCollectionQuestionId,
  problemCurationPickerSources,
  removeCollectionQuestionId,
  savedSearchDeletionFromObserved,
  savedSearchReplacementFromObserved,
} from "../src/features/problem_curation/problem_curation_model.ts";
import { EMPTY_CATALOG_QUERY } from "../src/pages/library_page_model.ts";

test("collection edits retain a unique ordered public Question ID list", () => {
  const appended = appendCollectionQuestionIds(["ABC-1234", "DEF-5678"], ["DEF-5678", "GHJ-9KMP"]);
  const moved = moveCollectionQuestionId(appended, 2, -1);

  assert.deepEqual(removeCollectionQuestionId(moved, "DEF-5678"), ["ABC-1234", "GHJ-9KMP"]);
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
    responseFamilies: [],
    taxonomy: [],
    capabilities: [],
    licenses: [],
    publicationScopes: [],
    evidence: "available",
    usedInMyCourses: "any",
    authorship: "any",
  });
});

test("running a saved search starts with its current-catalog filters", () => {
  const query = libraryQueryFromSavedSearch({
    reference: "PS-7",
    title: "Molecular genetics",
    revision: "3",
    filter: {
      text: "genetics",
      bylines: ["Lab team"],
      backends: ["qti"],
      tags: ["inheritance"],
      responseFamilies: ["multipleChoice"],
      taxonomy: [{ scheme: "ncbi", code: "9606" }],
      capabilities: ["serverGrading"],
      licenses: ["ccBy"],
      publicationScopes: ["public"],
      evidence: "available",
      usedInMyCourses: "used",
      authorship: "any",
    },
  });

  assert.deepEqual(query, {
    search: "genetics",
    byline: "Lab team",
    backend: "qti",
    tag: "inheritance",
    responseFamily: "multipleChoice",
    taxonomy: "ncbi:9606",
    capability: "serverGrading",
    license: "ccBy",
    evidence: "available",
    usedInMyCourses: "used",
    authorship: "any",
    publicationScopes: ["public"],
  });
});

test("only an authenticated Instructor mode enables personal curation", () => {
  assert.equal(
    mayMutatePersonalCuration({
      authenticated: true,
      tenant: "T-1",
      user: { id: "U-1", displayName: "Elena", roles: ["instructor"] },
    }),
    true,
  );
  assert.equal(
    mayMutatePersonalCuration({
      authenticated: true,
      tenant: "T-1",
      user: { id: "U-2", displayName: "Morgan", roles: ["sysadmin"] },
    }),
    false,
  );
});

test("Sysadmin institution-reader collection detail stays browse and reuse only", () => {
  assert.equal(mayEditOpenedProblemCollection(false, "institutionReader"), false);
  assert.equal(mayEditOpenedProblemCollection(true, "owner"), true);
});

test("picker maps only named collections beside one dedicated Favorites source", () => {
  const sources = problemCurationPickerSources(
    [
      {
        reference: "PC-1",
        kind: "favorites",
        title: "Favorites",
        visibility: "private",
        revision: "4",
        access: "owner",
      },
      {
        reference: "PC-2",
        kind: "named",
        title: "Exam candidates",
        visibility: "institution",
        revision: "7",
        access: "owner",
      },
    ],
    true,
  );

  assert.deepEqual(
    sources.map((source) => [source.kind, source.label]),
    [
      ["catalog", "Current library"],
      ["mine", "My published questions"],
      ["favorites", "Favorites"],
      ["collection", "Exam candidates"],
    ],
  );
});

test("saved-search replacement sends the revision already observed with retained filter meaning", () => {
  const search = {
    reference: "PS-3",
    title: "Peptide candidates",
    revision: "3",
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
      revision: replacement.revision,
      filter: replacement.filter,
    },
    { reference: "PS-3", revision: '"3"', filter: search.filter },
  );
});

test("named deletion confirmations carry visible names, consequences, and observed revisions", () => {
  const collectionDeletion = collectionDeletionFromObserved({
    reference: "PC-7",
    kind: "named",
    title: "Exam candidates",
    visibility: "private",
    revision: "7",
    access: "owner",
  });
  const savedDeletion = savedSearchDeletionFromObserved({
    reference: "PS-3",
    title: "Peptide candidates",
    revision: "3",
    filter: catalogSearchFilterFromLibraryQuery(EMPTY_CATALOG_QUERY),
  });

  assert.deepEqual(
    [collectionDeletion, savedDeletion].map((deletion) => ({
      heading: deletion.heading,
      consequence: deletion.consequence,
      revision: deletion.revision,
      confirmLabel: deletion.confirmLabel,
    })),
    [
      {
        heading: 'Delete collection "Exam candidates"?',
        consequence:
          "Deleting this collection removes its saved ordered question list. Published questions remain available in the Library.",
        revision: '"7"',
        confirmLabel: "Delete collection",
      },
      {
        heading: 'Delete saved search "Peptide candidates"?',
        consequence:
          "Deleting this saved search removes the shortcut. Current Library questions and filters remain available.",
        revision: '"3"',
        confirmLabel: "Delete saved search",
      },
    ],
  );
});

test("curation ETags accept only strong positive observed revisions", () => {
  assert.equal(curationEtagFromObservedRevision("12"), '"12"');
  assert.throws(() => curationEtagFromObservedRevision("012"), /current positive/u);
});
