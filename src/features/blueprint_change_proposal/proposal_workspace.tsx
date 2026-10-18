import { A, useParams } from "@solidjs/router";
import { Show, createEffect, createResource, createSignal, onCleanup, type JSX } from "solid-js";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintChangeProposalSummaryView } from "../../../generated/api/BlueprintChangeProposalSummaryView";
import type { ContentClassificationClient } from "../../api/content_classification";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import type { BlueprintChangeProposalClient } from "../../api/blueprint_change_proposal";
import { useApplicationApi } from "../../api/application_api";
import { CourseClassificationSummary } from "../../components/course_classification_summary";
import { PageFrame } from "../../components/page_frame/page_frame";
import type { RecordContent } from "../../components/data_sheet/record_content";
import { useSelectedDisplayDateTimeFormatter } from "../../selected_display_zone";
import { ProposalReview } from "./proposal_review";
import { ProposalSearch } from "./proposal_search";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createSearchState } from "../../components/search_block/search_state";
import {
  blueprintCourseSearchDefinition,
  emptyBlueprintSearch,
} from "../blueprint_course/blueprint_course_search_definition";
import {
  blueprintCourseSearchFields,
  blueprintCourseSearchRecordQuery,
} from "../blueprint_course/blueprint_course_search_fields";

type Client = BlueprintCourseClient & BlueprintChangeProposalClient;
function recordPath(id: string): string {
  // ASVS 1.2.2: fixed same-origin route with encoded opaque participant handle.
  return `/blueprint-change-proposals/${encodeURIComponent(id)}`;
}

function proposalState(item: BlueprintChangeProposalSummaryView): string {
  if (item.accepted) {
    return `Accepted target Revision ${item.accepted.targetRevisionTuple.revisionNumber}`;
  }
  if (item.targetIsStale) return "Older target basis; acceptance unavailable";
  return "Proposed";
}

function proposalContent(
  formatDateTime: (timestamp: number | Date) => string,
  item: BlueprintChangeProposalSummaryView,
): RecordContent {
  const title = `${item.sourceNames.longName} to ${item.targetNames.longName}`;
  return {
    title,
    details: [
      { kind: "text", label: "Source", value: item.sourceNames.shortName },
      {
        kind: "text",
        label: "Source Revision",
        value: String(item.sourceRevisionTuple.revisionNumber),
      },
      { kind: "text", label: "Target", value: item.targetNames.shortName },
      {
        kind: "text",
        label: "Target Revision",
        value: String(item.targetRevisionTuple.revisionNumber),
      },
      { kind: "text", label: "State", value: proposalState(item) },
      {
        kind: "time",
        label: "Created",
        value: formatDateTime(item.createdAt),
        dateTime: new Date(item.createdAt).toISOString(),
      },
    ],
    actions: [
      {
        id: "open-proposal",
        kind: "link",
        label: "Open proposal",
        href: recordPath(item.proposalId),
        primary: true,
      },
    ],
  };
}

export function ProposalRecords(props: {
  readonly client: BlueprintChangeProposalClient;
  readonly target?: string;
  readonly formatDateTime: (timestamp: number | Date) => string;
}): JSX.Element {
  return (
    <ProposalSearch
      client={props.client}
      target={props.target}
      formatDateTime={props.formatDateTime}
      content={(item) => proposalContent(props.formatDateTime, item)}
    />
  );
}

export function ProposalTargetTools(props: {
  readonly client: Client;
  readonly target: BlueprintCourseView;
  readonly classificationClient: ContentClassificationClient;
  readonly formatDateTime: (timestamp: number | Date) => string;
}): JSX.Element {
  const [open, setOpen] = createSignal(false);
  const [selected, setSelected] = createSignal<BlueprintCourseView>();
  const [busy, setBusy] = createSignal(false),
    [message, setMessage] = createSignal("");
  const [created, setCreated] = createSignal<string>();
  let sourceLoad = 0;
  let targetGeneration = 0;
  createEffect(() => {
    void props.target.id;
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
  async function choose(blueprintCourseId: string): Promise<void> {
    const generation = ++sourceLoad;
    setSelected(undefined);
    setMessage("");
    if (!blueprintCourseId || blueprintCourseId === props.target.id) return;
    setBusy(true);
    try {
      const result = await props.client.getBlueprintCourse(blueprintCourseId);
      if (generation === sourceLoad) {
        // ASVS 8.3.1: the trusted API authorizes the exact saved source; Archived sources stay ineligible.
        if (result.blueprintCourse.availability === "archived") {
          setMessage("This saved source is unavailable. Choose another source.");
        } else setSelected(result.blueprintCourse);
      }
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
        sourceRevisionTuple: source.current_revision_tuple,
        sourceBlueprintEditNumber: source.blueprint_edit_number,
        targetRevisionTuple: target.current_revision_tuple,
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
  function SourcePicker(): JSX.Element {
    const formatTimestamp = (): ((timestamp: number | Date) => string) => props.formatDateTime;
    const definition = blueprintCourseSearchDefinition(props.client, formatTimestamp, "all");
    const state = createSearchState(
      {
        ...definition,
        initialQuery: emptyBlueprintSearch(false),
        isEligible: () => true,
        content: (blueprint): RecordContent => ({
          ...definition.content(blueprint),
          actions: [
            {
              id: "choose-source",
              kind: "command",
              label: blueprint.id === props.target.id ? "Current Blueprint" : "Choose source",
              primary: blueprint.id !== props.target.id,
              disabled: busy() || blueprint.id === props.target.id,
              pressed: selected()?.id === blueprint.id,
              onClick: () => void choose(blueprint.id),
            },
          ],
        }),
      },
      { runOnMount: true },
    );
    return (
      <SearchBlock
        layout="above"
        state={state}
        controls={
          <SearchControls
            state={state}
            textLabel="Search saved Blueprint sources"
            disabled={busy}
          />
        }
        collection={{
          ariaLabel: "Saved Blueprint sources",
          fields: blueprintCourseSearchFields(props.classificationClient, "all", formatTimestamp),
          query: blueprintCourseSearchRecordQuery({
            query: state.query,
            apply: state.apply,
            sort: state.sort,
            client: props.client,
            scope: "all",
          }),
          disabled: busy,
          emptyState: {
            title: "No saved Blueprint sources match.",
            message: "Try another name or clear the search.",
          },
        }}
      />
    );
  }
  return (
    <section class="blueprint-proposal">
      <A href="/blueprint-change-proposals">My Change Proposals across targets</A>
      <ProposalRecords
        client={props.client}
        target={props.target.id}
        formatDateTime={props.formatDateTime}
      />
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
            <SourcePicker />
            <Show when={selected()}>
              {(source) => (
                <>
                  <h4>{source().long_name}</h4>
                  <p>
                    Source {source().short_name}, Revision{" "}
                    {source().current_revision_tuple.revisionNumber}, edit{" "}
                    {source().blueprint_edit_number}; target {props.target.long_name}, Revision{" "}
                    {props.target.current_revision_tuple.revisionNumber}, edit{" "}
                    {props.target.blueprint_edit_number}.
                  </p>
                  <CourseClassificationSummary value={source().classification} />
                  <p>Source Blueprint Theme: {source().theme}.</p>
                  <p>
                    The exact proposed teaching structure, instructions, settings and Question/Pool
                    pins are shared with the receiving Instructor. Proposal evidence is visible only
                    to proposal participants.
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
  const formatInAccountZone = useSelectedDisplayDateTimeFormatter();
  const formatDateTime = (timestamp: number | Date): string => formatInAccountZone()(timestamp);
  return (
    <PageFrame routeSurface="myChangeProposals">
      <ProposalRecords client={api.client} formatDateTime={formatDateTime} />
    </PageFrame>
  );
}

export function ChangeProposalDetailLivePage(): JSX.Element {
  const api = useApplicationApi(),
    params = useParams();
  const formatInAccountZone = useSelectedDisplayDateTimeFormatter();
  const formatDateTime = (timestamp: number | Date): string => formatInAccountZone()(timestamp);
  const [detail, { refetch }] = createResource(
    () => params["proposalId"] ?? "",
    (id) => api.client.getBlueprintChangeProposal(id),
  );
  return (
    <PageFrame routeSurface="changeProposalDetail">
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
            formatDateTime={formatDateTime}
            refresh={async () => {
              await refetch();
            }}
          />
        )}
      </Show>
    </PageFrame>
  );
}
