import { A, useParams } from "@solidjs/router";
import {
  For,
  Show,
  createEffect,
  createResource,
  createSignal,
  onCleanup,
  type JSX,
} from "solid-js";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintChangeProposalPageView } from "../../../generated/api/BlueprintChangeProposalPageView";
import type { BlueprintCourseSummaryView } from "../../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import type { BlueprintChangeProposalClient } from "../../api/blueprint_change_proposal";
import { useApplicationApi } from "../../api/application_api";
import { CourseClassificationSummary } from "../../components/course_classification_summary";
import { ProposalReview } from "./proposal_review";

type Client = BlueprintCourseClient & BlueprintChangeProposalClient;
function recordPath(id: string): string {
  // ASVS 1.2.2: fixed same-origin route with encoded opaque participant handle.
  return `/blueprint-change-proposals/${encodeURIComponent(id)}`;
}

export function ProposalRecords(props: {
  readonly client: BlueprintChangeProposalClient;
  readonly target?: string;
}): JSX.Element {
  const [page, setPage] = createSignal<BlueprintChangeProposalPageView>();
  const [busy, setBusy] = createSignal(false),
    [failed, setFailed] = createSignal(false);
  const [opened, setOpened] = createSignal(false);
  let loadGeneration = 0;
  createEffect(() => {
    const target = props.target;
    void target;
    ++loadGeneration;
    setPage(undefined);
    setOpened(false);
    setBusy(false);
    setFailed(false);
  });
  onCleanup(() => {
    ++loadGeneration;
  });
  async function load(cursor?: string): Promise<void> {
    if (busy()) return;
    const generation = ++loadGeneration;
    const target = props.target;
    setBusy(true);
    setFailed(false);
    try {
      const result = target
        ? await props.client.listBlueprintChangeProposalsForTarget(target, cursor)
        : await props.client.listMyBlueprintChangeProposals(cursor);
      if (generation !== loadGeneration || target !== props.target) return;
      setPage({
        ...result,
        items: cursor ? [...(page()?.items ?? []), ...result.items] : result.items,
      });
    } catch {
      if (generation === loadGeneration) setFailed(true);
    } finally {
      if (generation === loadGeneration) setBusy(false);
    }
  }
  return (
    <section class="blueprint-proposal">
      <h2>
        {props.target
          ? "Incoming / submitted Change Proposals for this target"
          : "My Change Proposals"}
      </h2>
      <button
        type="button"
        disabled={busy()}
        onClick={() => {
          setOpened(true);
          void load();
        }}
      >
        {busy() ? "Loading records..." : opened() ? "Refresh records" : "Open records"}
      </button>
      <Show when={failed()}>
        <p role="alert">Records could not load. Refresh records to try again.</p>
      </Show>
      <Show when={page()}>
        {(value) => (
          <>
            <For
              each={value().items}
              fallback={<p>No authorized Change Proposals in this view.</p>}
            >
              {(item) => (
                <article>
                  <h3>
                    <A href={recordPath(item.proposalId)}>
                      {item.sourceNames.longName} to {item.targetNames.longName}
                    </A>
                  </h3>
                  <p>
                    Source {item.sourceNames.shortName}, Revision {item.source.revision}; comparison
                    target {item.targetNames.shortName}, Revision {item.target.revision}.{" "}
                    {item.accepted
                      ? `Accepted target Revision ${item.accepted.target.revision}`
                      : item.targetIsStale
                        ? "Older target basis; acceptance unavailable"
                        : "Proposed"}
                    . Created {item.createdAt}.
                  </p>
                </article>
              )}
            </For>
            <Show when={value().nextCursor}>
              {(cursor) => (
                <button type="button" disabled={busy()} onClick={() => void load(cursor())}>
                  Load more records
                </button>
              )}
            </Show>
          </>
        )}
      </Show>
    </section>
  );
}

export function ProposalTargetTools(props: {
  readonly client: Client;
  readonly target: BlueprintCourseView;
}): JSX.Element {
  const [open, setOpen] = createSignal(false);
  const [sources, setSources] = createSignal<readonly BlueprintCourseSummaryView[]>([]);
  const [cursor, setCursor] = createSignal<string>();
  const [selected, setSelected] = createSignal<BlueprintCourseView>();
  const [busy, setBusy] = createSignal(false),
    [message, setMessage] = createSignal("");
  const [created, setCreated] = createSignal<string>();
  let sourceLoad = 0;
  let targetGeneration = 0;
  createEffect(() => {
    const reference = props.target.id;
    void reference;
    ++targetGeneration;
    ++sourceLoad;
    setOpen(false);
    setSelected(undefined);
    setCreated(undefined);
    setMessage("");
    setBusy(false);
  });
  onCleanup(() => {
    ++targetGeneration;
    ++sourceLoad;
  });
  async function loadSources(next?: string): Promise<void> {
    const generation = targetGeneration;
    setBusy(true);
    try {
      // List includes owned Private saved sources; never use public-only discovery here.
      const page = await props.client.listBlueprintCourses(next);
      if (generation !== targetGeneration) return;
      setSources(next ? [...sources(), ...page.items] : page.items);
      setCursor(page.nextCursor ?? undefined);
    } catch {
      if (generation === targetGeneration)
        setMessage("Saved sources could not load. Close and reopen the picker to retry.");
    } finally {
      if (generation === targetGeneration) setBusy(false);
    }
  }
  async function choose(reference: string): Promise<void> {
    const generation = ++sourceLoad;
    setSelected(undefined);
    setMessage("");
    if (!reference) return;
    setBusy(true);
    try {
      const result = await props.client.getBlueprintCourse(reference);
      if (generation === sourceLoad) setSelected(result.blueprintCourse);
    } catch {
      if (generation === sourceLoad)
        setMessage("This saved source is unavailable. Choose another source.");
    } finally {
      if (generation === sourceLoad) setBusy(false);
    }
  }
  function cancel(): void {
    ++sourceLoad;
    setSelected(undefined);
    setOpen(false);
    setBusy(false);
    setMessage("Proposal cancelled. Nothing was sent.");
  }
  async function submit(): Promise<void> {
    const source = selected();
    if (!source || busy()) return;
    const generation = targetGeneration;
    const target = props.target;
    setBusy(true);
    try {
      const result = await props.client.createBlueprintChangeProposal(target.id, {
        source: source.current_revision,
        sourceBlueprintEditNumber: source.blueprint_edit_number,
        target: target.current_revision,
        targetBlueprintEditNumber: target.blueprint_edit_number,
      });
      if (generation !== targetGeneration) return;
      setCreated(result.proposal.proposalId);
      setOpen(false);
      setSelected(undefined);
      setMessage(
        "Proposal recorded. The receiving owner decides what to accept; no target or daughter was changed.",
      );
    } catch {
      if (generation !== targetGeneration) return;
      setSelected(undefined);
      setMessage(
        "Proposal was not confirmed. Reload the target and pick the saved source again before submitting.",
      );
    } finally {
      if (generation === targetGeneration) setBusy(false);
    }
  }
  return (
    <section class="blueprint-proposal">
      <A href="/blueprint-change-proposals">My Change Proposals across targets</A>
      <ProposalRecords client={props.client} target={props.target.id} />
      <Show
        when={
          props.target.read_access !== "blueprint_course_owner" &&
          props.target.availability !== "archived"
        }
      >
        <button
          type="button"
          disabled={busy()}
          onClick={() => {
            setOpen(true);
            setCreated(undefined);
            void loadSources();
          }}
        >
          Propose saved Blueprint changes
        </button>
        <Show when={open()}>
          <section aria-label="Choose saved proposal source">
            <h3>Choose the saved source explicitly</h3>
            <p>
              Only exact saved source and target Revisions and metadata are proposed. Unsaved editor
              changes are excluded.
            </p>
            <label>
              Saved source Blueprint
              <select
                disabled={busy()}
                value={selected()?.id ?? ""}
                onChange={(e) => void choose(e.currentTarget.value)}
              >
                <option value="">Choose a source</option>
                <For
                  each={sources().filter(
                    (source) => source.id !== props.target.id && source.availability !== "archived",
                  )}
                >
                  {(source) => (
                    <option value={source.id}>
                      {source.long_name} ({source.short_name}; {source.availability}; Revision{" "}
                      {source.current_revision.revision})
                    </option>
                  )}
                </For>
              </select>
            </label>
            <Show when={cursor()}>
              {(next) => (
                <button type="button" disabled={busy()} onClick={() => void loadSources(next())}>
                  More saved sources
                </button>
              )}
            </Show>
            <Show when={selected()}>
              {(source) => (
                <>
                  <h4>{source().long_name}</h4>
                  <p>
                    Source {source().short_name}, Revision {source().current_revision.revision},
                    edit {source().blueprint_edit_number}; target {props.target.long_name}, Revision{" "}
                    {props.target.current_revision.revision}, edit{" "}
                    {props.target.blueprint_edit_number}.
                  </p>
                  <CourseClassificationSummary value={source().classification} />
                  <p>
                    The exact proposed teaching structure, instructions, settings and Question/Pool
                    pins are shared with the receiving Instructor, including when your source is
                    Private. This does not make the Private source generally visible.
                  </p>
                  <button
                    class="primary-action"
                    type="button"
                    disabled={busy()}
                    onClick={() => void submit()}
                  >
                    {busy() ? "Recording proposal..." : "Confirm saved-source proposal"}
                  </button>
                </>
              )}
            </Show>
            <button type="button" disabled={busy()} onClick={cancel}>
              Cancel proposal
            </button>
          </section>
        </Show>
      </Show>
      <Show when={message()}>
        <p role="status">{message()}</p>
      </Show>
      <Show when={created()}>{(id) => <A href={recordPath(id())}>Open recorded proposal</A>}</Show>
    </section>
  );
}

export function MyChangeProposalsLivePage(): JSX.Element {
  const api = useApplicationApi();
  return (
    <main class="page">
      <h1>My Change Proposals</h1>
      <p>Your own submissions across target Blueprints, including retained accepted records.</p>
      <ProposalRecords client={api.client} />
    </main>
  );
}

export function ChangeProposalDetailLivePage(): JSX.Element {
  const api = useApplicationApi(),
    params = useParams();
  const [detail, { refetch }] = createResource(
    () => params["proposalId"] ?? "",
    (id) => api.client.getBlueprintChangeProposal(id),
  );
  return (
    <main class="page">
      <h1>Review Blueprint Change Proposal</h1>
      <A href="/blueprint-change-proposals">My Change Proposals</A>
      <Show when={detail.loading}>
        <p role="status">Loading frozen proposal evidence...</p>
      </Show>
      <Show when={Boolean(detail.error)}>
        <p role="alert">
          This proposal is unavailable or could not load. Check your session and try again.
        </p>
        <button type="button" onClick={() => void refetch()}>
          Retry record
        </button>
      </Show>
      <Show when={!detail.error && detail()} keyed>
        {(value) => (
          <ProposalReview
            client={api.client}
            detail={value}
            refresh={async () => {
              await refetch();
            }}
          />
        )}
      </Show>
    </main>
  );
}
