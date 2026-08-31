// question_curation_repository.ts - adapts Question Curation to the Library workspace.

import type { QuestionFolderReference } from "../../../generated/api/QuestionFolderReference";
import { createQuestionLibraryRepository } from "../../api/question_library_repository";
import type { ApiClient } from "../../api/client";
import type {
  QuestionLibraryRepository,
  QuestionSearchResult,
} from "../../pages/library_page_model";
import {
  type QuestionPickerSearchRequest,
  type QuestionPickerSourceRepository,
} from "../question_picker";
import {
  questionSearchFilterFromLibraryQuery,
  type QuestionCurationPage,
  type QuestionCurationRepository,
} from "./question_curation_model";

const PAGE_SIZE = 100;

function rowFromFolderEntry(
  entry: import("../../../generated/api/QuestionFolderEntryView").QuestionFolderEntryView,
): QuestionSearchResult {
  return {
    displayId: entry.questionId,
    title: entry.summary.metadata.title,
    summary: `Published ${entry.summary.backend} Question.`,
    byline: entry.summary.byline.names,
    classifications: entry.summary.metadata.classifications.map(
      (classification) => `${classification.system}:${classification.code}`,
    ),
    capabilities: entry.summary.capabilities,
    license: entry.summary.metadata.license.kind,
    evidence: { state: "insufficientEvidence" },
  };
}

function sourceFolder(
  source: QuestionPickerSearchRequest["source"],
): QuestionFolderReference | null {
  return source.kind === "folder" ? source.folder : null;
}

/**
 * Keeps HTTP mutations and picker source lookup at the route boundary. The
 * Library model receives only bounded pages and current public question views.
 */
export function createQuestionCurationRepository(
  client: ApiClient,
  questionLibrary: QuestionLibraryRepository,
): {
  readonly curation: QuestionCurationRepository;
  readonly picker: QuestionPickerSourceRepository;
} {
  const authoredQuestionLibrary = createQuestionLibraryRepository(
    client,
    "authoredByCurrentAccount",
  );
  const sharedQuestionLibrary = createQuestionLibraryRepository(client, "any");
  const curation: QuestionCurationRepository = {
    async getFolder(reference) {
      const result = await client.getQuestionFolder(reference);
      return { value: result.folder, etag: result.etag };
    },
    async listFolders(cursor) {
      return await client.listQuestionFolders(cursor ?? undefined, PAGE_SIZE);
    },
    async listFolderEntries(reference, cursor) {
      const result = await client.listQuestionFolderEntries(
        reference,
        cursor ?? undefined,
        PAGE_SIZE,
      );
      return { ...result.page, etag: result.etag };
    },
    async replaceFolder(replacement) {
      if (replacement.reference === null) {
        const saved = await client.createQuestionFolder({
          title: replacement.title,
          questionIds: replacement.questionIds,
        });
        return { value: saved.folder, etag: saved.etag };
      }
      const saved = await client.replaceQuestionFolder(
        replacement.reference,
        {
          title: replacement.title,
          questionIds: replacement.questionIds,
        },
        replacement.editNumber ?? missingEtag(),
      );
      return { value: saved.folder, etag: saved.etag };
    },
    async deleteFolder(reference, editNumber) {
      await client.deleteQuestionFolder(reference, editNumber);
    },
    async listSavedSearches(cursor) {
      return await client.listSavedQuestionSearches(cursor ?? undefined, PAGE_SIZE);
    },
    async replaceSavedSearch(replacement) {
      const request = {
        title: replacement.title,
        filter: replacement.filter ?? questionSearchFilterFromLibraryQuery(replacement.query),
      };
      if (replacement.reference === null) {
        const saved = await client.createSavedQuestionSearch(request);
        return { value: saved.search, etag: saved.etag };
      }
      const saved = await client.replaceSavedQuestionSearch(
        replacement.reference,
        request,
        replacement.editNumber ?? missingEtag(),
      );
      return { value: saved.search, etag: saved.etag };
    },
    async getSavedSearch(reference) {
      const result = await client.getSavedQuestionSearch(reference);
      return { value: result.search, etag: result.etag };
    },
    async deleteSavedSearch(reference, editNumber) {
      await client.deleteSavedQuestionSearch(reference, editNumber);
    },
  };
  const picker: QuestionPickerSourceRepository = {
    async search(request) {
      if (request.source.kind === "library") {
        return await questionLibrary.search(request.query, request.cursor);
      }
      if (request.source.kind === "sharedLibrary") {
        return await sharedQuestionLibrary.search(request.query, request.cursor);
      }
      if (request.source.kind === "mine") {
        return await authoredQuestionLibrary.search(request.query, request.cursor);
      }
      const reference = sourceFolder(request.source);
      if (reference !== null) {
        return folderEntryPage(
          await client.listQuestionFolderEntries(reference, request.cursor ?? undefined, PAGE_SIZE),
        );
      }
      throw new Error("Choose the current Question Library or a private Question Folder source.");
    },
  };
  return { curation, picker };
}

function folderEntryPage(value: {
  readonly page: QuestionCurationPage<
    import("../../../generated/api/QuestionFolderEntryView").QuestionFolderEntryView
  >;
}): unknown {
  return {
    items: value.page.items
      .filter((member) => member.questionRevisionAvailability.availability === "available")
      .map(rowFromFolderEntry),
    nextCursor: value.page.nextCursor,
    aggregates: [],
  };
}

/** Revisions are canonical decimal values and the server's strong ETag syntax is fixed. */
function missingEtag(): string {
  throw new Error("Load the current curation item before updating it.");
}
