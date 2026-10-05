import assert from "node:assert/strict";
import test from "node:test";

import {
  fetchSharedProfileAvatarImage,
  getSharedInstructorProfile,
} from "../src/components/instructor_profile_requests.ts";

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

test("concurrent Instructor Profile and avatar reads share one client request then release", async () => {
  const profileRequest = deferred();
  const avatarRequest = deferred();
  let profileCalls = 0;
  let avatarCalls = 0;
  const client = {
    getInstructorProfile: () => {
      profileCalls += 1;
      return profileRequest.promise;
    },
    fetchProfileAvatarImage: () => {
      avatarCalls += 1;
      return avatarRequest.promise;
    },
  };

  const firstProfile = getSharedInstructorProfile(client, "U7K3M2PA0");
  const secondProfile = getSharedInstructorProfile(client, "U7K3M2PA0");
  const firstAvatar = fetchSharedProfileAvatarImage(client, "image-1");
  const secondAvatar = fetchSharedProfileAvatarImage(client, "image-1");
  assert.strictEqual(firstProfile, secondProfile);
  assert.strictEqual(firstAvatar, secondAvatar);
  assert.equal(profileCalls, 1);
  assert.equal(avatarCalls, 1);

  profileRequest.resolve({ displayName: "Elena Rivera", avatar: null });
  avatarRequest.resolve(new Blob(["image"]));
  await Promise.all([firstProfile, firstAvatar]);

  const nextProfileRequest = deferred();
  const nextAvatarRequest = deferred();
  client.getInstructorProfile = () => {
    profileCalls += 1;
    return nextProfileRequest.promise;
  };
  client.fetchProfileAvatarImage = () => {
    avatarCalls += 1;
    return nextAvatarRequest.promise;
  };
  const nextProfile = getSharedInstructorProfile(client, "U7K3M2PA0");
  const nextAvatar = fetchSharedProfileAvatarImage(client, "image-1");
  assert.equal(profileCalls, 2);
  assert.equal(avatarCalls, 2);
  nextProfileRequest.resolve({ displayName: "Elena Rivera", avatar: null });
  nextAvatarRequest.resolve(new Blob(["image"]));
  await Promise.all([nextProfile, nextAvatar]);
});

test("a rejected Instructor Profile read releases its in-flight entry", async () => {
  const failedRequest = deferred();
  let calls = 0;
  const client = {
    getInstructorProfile: () => {
      calls += 1;
      return failedRequest.promise;
    },
    fetchProfileAvatarImage: async () => new Blob(),
  };

  const first = getSharedInstructorProfile(client, "U7K3M2PA0");
  const second = getSharedInstructorProfile(client, "U7K3M2PA0");
  assert.strictEqual(first, second);
  failedRequest.reject(new Error("inactive Instructor"));
  await assert.rejects(first, /inactive Instructor/);

  const nextRequest = deferred();
  client.getInstructorProfile = () => {
    calls += 1;
    return nextRequest.promise;
  };
  const next = getSharedInstructorProfile(client, "U7K3M2PA0");
  assert.equal(calls, 2);
  nextRequest.resolve({ displayName: "Elena Rivera", avatar: null });
  await next;
});
