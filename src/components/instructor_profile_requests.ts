// instructor_profile_requests.ts - shares only simultaneous Instructor Profile reads.

import type { ApiClient } from "../api/client";
import type { InstructorProfileView } from "../api/profile_avatar";

type InstructorProfileReadClient = Pick<
  ApiClient,
  "fetchProfileAvatarImage" | "getInstructorProfile"
>;

const inFlightReadsByClient = new WeakMap<
  InstructorProfileReadClient,
  Map<string, Promise<unknown>>
>();

function shareInFlightRead<Result>(
  client: InstructorProfileReadClient,
  key: string,
  read: () => Promise<Result>,
): Promise<Result> {
  let reads = inFlightReadsByClient.get(client);
  if (reads === undefined) {
    reads = new Map<string, Promise<unknown>>();
    inFlightReadsByClient.set(client, reads);
  }

  const existing = reads.get(key) as Promise<Result> | undefined;
  if (existing !== undefined) return existing;

  const request = read();
  reads.set(key, request);
  function discardSettledRequest(): void {
    if (reads?.get(key) === request) reads.delete(key);
  }
  void request.then(discardSettledRequest, discardSettledRequest);
  return request;
}

/** Shares one concurrent public Instructor Profile request without retaining settled Profile data. */
export function getSharedInstructorProfile(
  client: InstructorProfileReadClient,
  accountId: string,
): Promise<InstructorProfileView> {
  return shareInFlightRead(client, `profile:${accountId}`, () =>
    client.getInstructorProfile(accountId),
  );
}

/** Shares one concurrent authorized Profile-image request without retaining settled image data. */
export function fetchSharedProfileAvatarImage(
  client: InstructorProfileReadClient,
  profileImageId: string,
): Promise<Blob> {
  return shareInFlightRead(client, `avatar:${profileImageId}`, () =>
    client.fetchProfileAvatarImage(profileImageId),
  );
}
