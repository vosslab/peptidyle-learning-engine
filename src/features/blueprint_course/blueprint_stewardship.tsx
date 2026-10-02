// Instructor-only lineage endorsements and self-only Watch activity.
import { Show, createResource, createSignal, type JSX } from "solid-js";
import type {
  BlueprintStewardshipClient,
  BlueprintWatchEvent,
} from "../../api/blueprint_stewardship";
import "./blueprint_stewardship.css";
import { RecordList, type RecordContent } from "../../components/record_list/record_list";

interface Props {
  readonly client: BlueprintStewardshipClient;
  readonly blueprintCourseId: string;
  readonly formatDateTime: (timestamp: number | Date) => string;
}

function eventLabel(kind: BlueprintWatchEvent["kind"]): string {
  switch (kind) {
    case "revision":
      return "New Blueprint Revision";
    case "published":
      return "Blueprint published";
    case "archived":
      return "Blueprint archived";
    case "restored":
      return "Blueprint restored";
  }
}

function watchEventContent(
  formatDateTime: Props["formatDateTime"],
  event: BlueprintWatchEvent,
): RecordContent {
  return {
    title: eventLabel(event.kind),
    details: [
      {
        kind: "time",
        label: "Occurred",
        value: formatDateTime(event.occurredAt),
        dateTime: new Date(event.occurredAt).toISOString(),
      },
    ],
    actions: [],
  };
}

/** The surrounding Instructor detail route mounts this only for Public or Archived Blueprints. */
export function BlueprintStewardship(props: Props): JSX.Element {
  const [busy, setBusy] = createSignal(false);
  const [notice, setNotice] = createSignal("");
  const [failed, setFailed] = createSignal(false);
  const [state, { refetch }] = createResource(
    () => props.blueprintCourseId,
    async (blueprintCourseId) => {
      const [star, watch] = await Promise.all([
        props.client.getBlueprintStar(blueprintCourseId),
        props.client.getBlueprintWatch(blueprintCourseId),
      ]);
      return { star, watch };
    },
  );
  const [instructors, { refetch: refetchInstructors }] = createResource(
    () => props.blueprintCourseId,
    (blueprintCourseId) => props.client.getBlueprintStarredInstructors(blueprintCourseId),
  );
  const [events, { refetch: refetchEvents }] = createResource(
    () => props.blueprintCourseId,
    (blueprintCourseId) => props.client.getBlueprintWatchEvents(blueprintCourseId),
  );

  async function toggle(kind: "star" | "watch"): Promise<void> {
    const current = state();
    if (current === undefined || busy() || state.loading || state.error !== undefined) return;
    setBusy(true);
    setNotice("");
    setFailed(false);
    try {
      if (kind === "star")
        await props.client.setBlueprintStar(
          props.blueprintCourseId,
          !current.star.viewerHasStarred,
        );
      else await props.client.setBlueprintWatch(props.blueprintCourseId, !current.watch.watching);
      // Refresh the preference that owns the control before confirming the action.
      await refetch();
      setNotice(
        kind === "star"
          ? "Your Star preference is saved."
          : "Your private Watch preference is saved.",
      );
    } catch {
      setFailed(true);
      setNotice("Your preference could not be confirmed. Reload stewardship before trying again.");
    } finally {
      setBusy(false);
    }
  }

  async function reload(): Promise<void> {
    setFailed(false);
    setNotice("");
    const results = await Promise.allSettled([refetch(), refetchInstructors(), refetchEvents()]);
    if (results[0]?.status === "rejected") setFailed(true);
  }

  return (
    <section
      class="blueprint-stewardship"
      aria-label="Blueprint stewardship"
      aria-busy={busy() || state.loading}
    >
      <h2>Stars and Watch activity</h2>
      <p>A Star is a visible endorsement. Your Watch preference and activity are private.</p>
      <Show when={state.error !== undefined || failed()}>
        <p role="alert">
          Blueprint stewardship could not be confirmed. Reload to see its current state.
        </p>
      </Show>
      <Show when={state.loading}>
        <p role="status">Loading Blueprint stewardship...</p>
      </Show>
      <Show when={state.error === undefined && state()}>
        {(current) => (
          <>
            <div class="blueprint-stewardship__actions">
              <button
                type="button"
                class="quiet-action"
                aria-pressed={current().star.viewerHasStarred}
                disabled={busy() || state.loading}
                onClick={() => void toggle("star")}
              >
                {current().star.viewerHasStarred ? "Starred" : "Star"}
              </button>
              <span>
                {current().star.starCount} {current().star.starCount === 1 ? "Star" : "Stars"}
              </span>
              <button
                type="button"
                class="quiet-action"
                aria-pressed={current().watch.watching}
                disabled={busy() || state.loading}
                onClick={() => void toggle("watch")}
              >
                {current().watch.watching ? "Watching" : "Watch"}
              </button>
            </div>
            <details>
              <summary>Instructors who Starred this Blueprint</summary>
              {/* ASVS 1.2.1, 8.2.3: exact server-projected names rendered as text, without identity links. */}
              <RecordList
                ariaLabel="Instructors who starred this Blueprint"
                emptyState={{ title: "No Instructor Stars to show." }}
                recordId={(instructor) => instructor.displayName}
                content={(instructor): RecordContent => ({
                  title: instructor.displayName,
                  details: [],
                  actions: [],
                })}
                rows={instructors() ?? []}
                state={
                  instructors.error !== undefined
                    ? {
                        kind: "error",
                        message: "Starred Instructor names could not be loaded.",
                        retry: () => void refetchInstructors(),
                      }
                    : instructors.loading
                      ? { kind: "loading" }
                      : { kind: "ready" }
                }
              />
            </details>
            <details>
              <summary>Your private Watch activity</summary>
              <p>Most recent 25 notifications for this Blueprint.</p>
              <RecordList
                ariaLabel="Your private Watch activity"
                emptyState={{ title: "No Watch activity yet." }}
                recordId={(event) => `${event.kind}-${event.occurredAt}`}
                content={(event) => watchEventContent(props.formatDateTime, event)}
                rows={events() ?? []}
                state={
                  events.error !== undefined
                    ? {
                        kind: "error",
                        message: "Watch activity could not be loaded.",
                        retry: () => void refetchEvents(),
                      }
                    : events.loading
                      ? { kind: "loading" }
                      : { kind: "ready" }
                }
              />
            </details>
          </>
        )}
      </Show>
      <button
        type="button"
        class="quiet-action"
        disabled={busy() || state.loading}
        onClick={() => void reload()}
      >
        Reload stewardship
      </button>
      <Show when={notice()}>
        <p role={failed() ? "alert" : "status"}>{notice()}</p>
      </Show>
    </section>
  );
}
