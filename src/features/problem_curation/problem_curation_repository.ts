// problem_curation_repository.ts - adapts D2 HTTP capability to the Library workspace.

import type { QuestionCollectionReference } from "../../../generated/api/QuestionCollectionReference";
import { createCatalogRepository } from "../../api/catalog_repository";
import type { ApiClient } from "../../api/client";
import type { CatalogBrowseRepository, CatalogBrowseRow } from "../../pages/library_page_model";
import {
  type ProblemPickerSearchRequest,
  type ProblemPickerSourceRepository,
} from "../problem_picker";
import {
  catalogSearchFilterFromLibraryQuery,
  type ProblemCurationPage,
  type ProblemCurationRepository,
} from "./problem_curation_model";

const PAGE_SIZE = 100;

function rowFromMember(
  member: import("../../../generated/api/QuestionCollectionMemberView").QuestionCollectionMemberView,
): CatalogBrowseRow {
  return {
    displayId: member.questionId,
    title: member.summary.metadata.title,
    summary: `Published ${member.summary.backend} problem.`,
    byline: member.summary.byline.names,
    taxonomy: member.summary.metadata.taxonomy.map((term) => `${term.scheme}:${term.code}`),
    capabilities: member.summary.capabilities,
    license: member.summary.metadata.license.kind,
    evidence: { state: "insufficientEvidence" },
  };
}

function sourceCollection(
  source: ProblemPickerSearchRequest["source"],
): QuestionCollectionReference | null {
  return source.kind === "collection" ? source.collection : null;
}

/**
 * Keeps HTTP mutations and picker source lookup at the route boundary. The
 * Library model receives only bounded pages and current public question views.
 */
export function createProblemCurationRepository(
  client: ApiClient,
  catalog: CatalogBrowseRepository,
): {
  readonly curation: ProblemCurationRepository;
  readonly picker: ProblemPickerSourceRepository;
} {
  const authoredCatalog = createCatalogRepository(client, "authoredByCurrentAccount");
  const sharedCatalog = createCatalogRepository(client, "any");
  const curation: ProblemCurationRepository = {
    async getCollection(reference) {
      const result = await client.getQuestionCollection(reference);
      return { value: result.collection, etag: result.etag };
    },
    async listCollections(cursor) {
      return await client.listQuestionCollections(cursor ?? undefined, PAGE_SIZE);
    },
    async listCollectionMembers(reference, cursor) {
      const result = await client.listQuestionCollectionMembers(
        reference,
        cursor ?? undefined,
        PAGE_SIZE,
      );
      return { ...result.page, etag: result.etag };
    },
    async replaceCollection(replacement) {
      if (replacement.reference === null) {
        const saved = await client.createQuestionCollection({
          title: replacement.title,
          questionIds: replacement.questionIds,
        });
        return { value: saved.collection, etag: saved.etag };
      }
      const saved = await client.replaceQuestionCollection(
        replacement.reference,
        {
          title: replacement.title,
          questionIds: replacement.questionIds,
        },
        replacement.editNumber ?? missingEtag(),
      );
      return { value: saved.collection, etag: saved.etag };
    },
    async deleteCollection(reference, editNumber) {
      await client.deleteQuestionCollection(reference, editNumber);
    },
    async listSavedSearches(cursor) {
      return await client.listSavedProblemSearches(cursor ?? undefined, PAGE_SIZE);
    },
    async replaceSavedSearch(replacement) {
      const request = {
        title: replacement.title,
        filter: replacement.filter ?? catalogSearchFilterFromLibraryQuery(replacement.query),
      };
      if (replacement.reference === null) {
        const saved = await client.createSavedProblemSearch(request);
        return { value: saved.search, etag: saved.etag };
      }
      const saved = await client.replaceSavedProblemSearch(
        replacement.reference,
        request,
        replacement.editNumber ?? missingEtag(),
      );
      return { value: saved.search, etag: saved.etag };
    },
    async getSavedSearch(reference) {
      const result = await client.getSavedProblemSearch(reference);
      return { value: result.search, etag: result.etag };
    },
    async deleteSavedSearch(reference, editNumber) {
      await client.deleteSavedProblemSearch(reference, editNumber);
    },
  };
  const picker: ProblemPickerSourceRepository = {
    async search(request) {
      if (request.source.kind === "catalog") {
        return await catalog.search(request.query, request.cursor);
      }
      if (request.source.kind === "sharedCatalog") {
        return await sharedCatalog.search(request.query, request.cursor);
      }
      if (request.source.kind === "mine") {
        return await authoredCatalog.search(request.query, request.cursor);
      }
      const reference = sourceCollection(request.source);
      if (reference !== null) {
        return memberPage(
          await client.listQuestionCollectionMembers(
            reference,
            request.cursor ?? undefined,
            PAGE_SIZE,
          ),
        );
      }
      throw new Error(
        "Choose the current Library, shared catalog, or a private Question Collection source.",
      );
    },
  };
  return { curation, picker };
}

function memberPage(value: {
  readonly page: ProblemCurationPage<
    import("../../../generated/api/QuestionCollectionMemberView").QuestionCollectionMemberView
  >;
}): unknown {
  return {
    items: value.page.items
      .filter((member) => member.questionVersionAvailability.availability === "available")
      .map(rowFromMember),
    nextCursor: value.page.nextCursor,
    aggregates: [],
  };
}

/** Revisions are canonical decimal values and the server's strong ETag syntax is fixed. */
function missingEtag(): string {
  throw new Error("Load the current curation item before updating it.");
}
