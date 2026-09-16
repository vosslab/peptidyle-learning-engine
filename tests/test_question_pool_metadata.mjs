import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeQuestionPoolLibraryPage,
  decodeQuestionPoolMetadata,
} from "../src/api/decoders/question_pool_library.ts";

const metadata = {
  title: "Inheritance reasoning",
  description: "Interpret interchangeable pedigrees.",
  disciplineUuid: "00000000-0000-0000-0000-000000000001",
  subjectUuid: "00000000-0000-0000-0000-000000000002",
  topicUuid: null,
  subtopicUuid: null,
  tags: ["pedigrees"],
};

test("Pool list retains independent metadata and exact Revision identity", () => {
  const page = {
    items: [
      {
        questionPoolRevision: { questionPoolId: "3S8B-Z4DZ", revisionNumber: 4 },
        metadata,
        memberCount: 2,
      },
    ],
    nextCursor: null,
  };
  assert.deepEqual(decodeQuestionPoolLibraryPage(page), page);
  const unicodeMetadata = {
    ...metadata,
    title: "\u00a0Inheritance\u00a0",
    description: "\u2003Interpret pedigrees\u2003",
    topicUuid: "00000000-0000-0000-0000-000000000003",
    subtopicUuid: "00000000-0000-0000-0000-000000000004",
    tags: ["\u00a0pedigrees\u00a0"],
  };
  const unicodePage = { ...page, items: [{ ...page.items[0], metadata: unicodeMetadata }] };
  assert.deepEqual(decodeQuestionPoolLibraryPage(unicodePage), unicodePage);
  const manyTags = Array.from({ length: 100 }, (_value, index) => `tag-${index}`);
  assert.deepEqual(
    decodeQuestionPoolMetadata({ ...metadata, tags: manyTags }, "metadata").tags,
    manyTags,
  );
  assert.deepEqual(
    decodeQuestionPoolMetadata({ ...metadata, title: "\u{1f9ec}".repeat(512) }, "metadata").title,
    "\u{1f9ec}".repeat(512),
  );
});

test("Pool metadata rejects missing required fields, unknown fields and malformed classifications", () => {
  for (const patch of [
    { title: undefined },
    { description: " " },
    { title: " trailing " },
    { disciplineUuid: null },
    { subjectUuid: "Biology" },
    { topicUuid: "Genetics" },
    { subtopicUuid: "00000000-0000-0000-0000-000000000004" },
    { tags: ["pedigrees", "pedigrees"] },
    { tags: ["a".repeat(121)] },
    { ownerAccountId: "forbidden" },
  ]) {
    assert.throws(
      () => decodeQuestionPoolMetadata({ ...metadata, ...patch }, "metadata"),
      DecodeError,
    );
  }
});
