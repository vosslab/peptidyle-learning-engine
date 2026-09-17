// Stable untrusted-response contract for retained Library discussions.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeLibraryDiscussionView } from "../src/api/decoders/library_discussion.ts";

const validDiscussion = {
  viewerMayManage: false,
  threads: [
    {
      threadId: "00000000-0000-4000-8000-000000000001",
      creationRevisionNumber: 1,
      state: "open",
      createdAt: 1_750_000_000_000,
      resolvedAt: null,
      viewerMayResolve: false,
      posts: [
        {
          postId: "00000000-0000-4000-8000-000000000002",
          authorDisplayName: "Dr. Rivera",
          body: "Clarify the prompt wording.",
          createdAt: 1_750_000_000_000,
          updatedAt: null,
          viewerMayEdit: false,
        },
      ],
    },
    {
      threadId: "00000000-0000-4000-8000-000000000003",
      creationRevisionNumber: 2,
      state: "resolved",
      createdAt: 1_750_000_000_010,
      resolvedAt: 1_750_000_000_012,
      viewerMayResolve: false,
      posts: [
        {
          postId: "00000000-0000-4000-8000-000000000004",
          authorDisplayName: "Dr. Rivera",
          body: "Clarify the prompt wording.",
          createdAt: 1_750_000_000_010,
          updatedAt: null,
          viewerMayEdit: false,
        },
      ],
    },
  ],
  impactNotices: [
    {
      impactNoticeId: "00000000-0000-4000-8000-000000000005",
      affectedRevisionNumber: null,
      authorDisplayName: "Dr. Rivera",
      body: "Check the current item before reusing it.",
      state: "active",
      createdAt: 1_750_000_000_000,
      updatedAt: 1_750_000_000_001,
      cancelledAt: null,
      viewerMayManage: false,
    },
    {
      impactNoticeId: "00000000-0000-4000-8000-000000000006",
      affectedRevisionNumber: 1,
      authorDisplayName: "Dr. Rivera",
      body: "Revision 1 has an answer-key issue.",
      state: "cancelled",
      createdAt: 1_750_000_000_000,
      updatedAt: 1_750_000_000_001,
      cancelledAt: 1_750_000_000_001,
      viewerMayManage: false,
    },
  ],
};

function discussionWith(patch) {
  const discussion = structuredClone(validDiscussion);
  patch(discussion);
  return discussion;
}

test("Library discussion decoder returns all retained lifecycle union variants", () => {
  assert.deepEqual(decodeLibraryDiscussionView(validDiscussion), validDiscussion);
});

test("Library discussion decoder rejects contradictory retained lifecycle data", () => {
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.threads[0].state = "open";
          discussion.threads[0].resolvedAt = 1_750_000_000_001;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.threads[1].resolvedAt = null;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.threads[1].resolvedAt = 1_749_999_999_999;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.threads[0].posts = [];
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.threads[0].posts.push({
            ...discussion.threads[0].posts[0],
            postId: "00000000-0000-4000-8000-000000000007",
            createdAt: 1_749_999_999_999,
          });
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.threads[0].posts[0].createdAt = 1_750_000_000_001;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.threads[0].posts[0].updatedAt = 1_749_999_999_999;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.impactNotices[0].cancelledAt = 1_750_000_000_001;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.impactNotices[0].updatedAt = 1_749_999_999_999;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.impactNotices[1].cancelledAt = null;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.impactNotices[1].updatedAt = 1_750_000_000_002;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.impactNotices[1].viewerMayManage = true;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.impactNotices[1].cancelledAt = 1_749_999_999_999;
          discussion.impactNotices[1].updatedAt = 1_749_999_999_999;
        }),
      ),
    DecodeError,
  );
});

test("Library discussion decoder retains Unicode scalars and rejects unknown fields", () => {
  assert.doesNotThrow(() =>
    decodeLibraryDiscussionView(
      discussionWith((discussion) => {
        discussion.threads[0].posts[0].body = "\uD83E\uDDEC";
      }),
    ),
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.threads[0].posts[0].body = "\uD83E";
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.threads[0].posts[0].body = "\uDDEC";
        }),
      ),
    DecodeError,
  );
  assert.doesNotThrow(() =>
    decodeLibraryDiscussionView(
      discussionWith((discussion) => {
        discussion.threads[0].posts[0].body = "\u{1F9EC}".repeat(4_000);
      }),
    ),
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.threads[0].posts[0].body = "\u{1F9EC}".repeat(4_001);
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () => decodeLibraryDiscussionView({ ...validDiscussion, unexpected: true }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.threads[0].state = "unrecognized";
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryDiscussionView(
        discussionWith((discussion) => {
          discussion.impactNotices[0].state = "unrecognized";
        }),
      ),
    DecodeError,
  );
});
