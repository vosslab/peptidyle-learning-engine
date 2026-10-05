// instructor_profile_page.tsx - signed-in view of one active Instructor Profile.

import { useParams } from "@solidjs/router";
import { Match, Switch, createResource, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { InstructorProfileLink } from "../components/instructor_profile_link";
import { PageFrame } from "../components/page_frame";
import { validateCanonicalPublicId } from "../question_id";

/** The URL identifies an Instructor; the API independently authorizes the signed-in viewer. */
export function InstructorProfilePage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams<{ accountId: string }>();
  const accountId = (): string | undefined =>
    validateCanonicalPublicId("account", params.accountId) ?? undefined;
  const [profile] = createResource(accountId, (id) =>
    applicationApi.client.getInstructorProfile(id),
  );
  return (
    <PageFrame
      routeSurface="instructor-profile"
      eyebrow="PLE Instructor"
      title="Instructor profile"
    >
      <Switch>
        <Match when={accountId() === undefined}>
          <p role="alert">This Instructor Profile address is invalid.</p>
        </Match>
        <Match when={profile.loading}>
          <p class="calm-status" role="status">
            Loading Instructor Profile...
          </p>
        </Match>
        <Match when={profile.error !== undefined}>
          <p role="alert">This Instructor Profile is unavailable.</p>
        </Match>
        <Match when={profile()}>
          {(value) => (
            <InstructorProfileLink
              accountId={accountId()!}
              avatar={value().avatar}
              displayName={value().displayName}
            />
          )}
        </Match>
      </Switch>
    </PageFrame>
  );
}
