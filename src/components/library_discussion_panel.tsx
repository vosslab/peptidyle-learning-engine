// Compact retained improvement-thread and impact-notice surface for one Library Object.

import { For, Show, createEffect, createResource, createSignal, type JSX } from "solid-js";

import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import { browserDisplayTimeZone, createDisplayDateTimeFormatter } from "../format_datetime";
import type {
  LibraryImpactNotice,
  LibraryImprovementPost,
  LibraryImprovementThread,
  LibraryObjectDiscussionKind,
} from "../api/library_discussion";
import { RecordDetailList } from "./record_list/record_detail_list";
import "./library_discussion_panel.css";

export interface LibraryDiscussionPanelProps {
  readonly kind: LibraryObjectDiscussionKind;
  readonly publicId: PublishedQuestionId;
}

function activityAnchor(activityId: string): string {
  return `library-activity-${activityId}`;
}

const displayTimeZone = browserDisplayTimeZone();
const formatBrowserDateTime = createDisplayDateTimeFormatter(displayTimeZone);

function timestamp(value: number): string {
  return formatBrowserDateTime(value);
}

function parseRevisionNumber(value: string): number | null {
  if (value.trim() === "") return null;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : null;
}

function ImpactNoticeView(props: {
  readonly kind: LibraryObjectDiscussionKind;
  readonly publicId: PublishedQuestionId;
  readonly notice: LibraryImpactNotice;
  readonly refresh: () => unknown;
}): JSX.Element {
  const notice = props.notice;
  switch (notice.state) {
    case "active":
      return (
        <ActiveImpactNotice
          kind={props.kind}
          publicId={props.publicId}
          notice={notice}
          refresh={props.refresh}
        />
      );
    case "cancelled":
      return <CancelledImpactNotice notice={notice} />;
  }
}

function CancelledImpactNotice(props: {
  readonly notice: Extract<LibraryImpactNotice, { readonly state: "cancelled" }>;
}): JSX.Element {
  return (
    <section
      id={activityAnchor(props.notice.impactNoticeId)}
      class="library-impact-notice"
      data-state={props.notice.state}
    >
      <p>
        <strong>Impact notice</strong> by {props.notice.authorDisplayName} ·{" "}
        {timestamp(props.notice.createdAt)}
        <Show when={props.notice.affectedRevisionNumber !== null}>
          {` · Revision ${props.notice.affectedRevisionNumber}`}
        </Show>
        {` · Cancelled ${timestamp(props.notice.cancelledAt)}`}
      </p>
      <p>{props.notice.body}</p>
    </section>
  );
}

function ActiveImpactNotice(props: {
  readonly kind: LibraryObjectDiscussionKind;
  readonly publicId: PublishedQuestionId;
  readonly notice: Extract<LibraryImpactNotice, { readonly state: "active" }>;
  readonly refresh: () => unknown;
}): JSX.Element {
  const api = useApplicationApi();
  const [body, setBody] = createSignal(props.notice.body);
  const [affectedRevision, setAffectedRevision] = createSignal(
    props.notice.affectedRevisionNumber?.toString() ?? "",
  );
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal("");

  async function update(): Promise<void> {
    const value = parseRevisionNumber(affectedRevision());
    if (affectedRevision().trim() !== "" && value === null) {
      setError("Affected Revision must be a positive whole number.");
      return;
    }
    setSaving(true);
    setError("");
    try {
      await api.client.updateImpactNotice(
        props.kind,
        props.publicId,
        props.notice.impactNoticeId,
        value,
        body(),
      );
      await props.refresh();
    } catch {
      setError("The impact notice could not be updated. Try again.");
    } finally {
      setSaving(false);
    }
  }

  async function cancel(): Promise<void> {
    setSaving(true);
    setError("");
    try {
      await api.client.cancelImpactNotice(props.kind, props.publicId, props.notice.impactNoticeId);
      await props.refresh();
    } catch {
      setError("The impact notice could not be cancelled. Try again.");
    } finally {
      setSaving(false);
    }
  }

  return (
    <section
      id={activityAnchor(props.notice.impactNoticeId)}
      class="library-impact-notice"
      data-state={props.notice.state}
    >
      <p>
        <strong>Impact notice</strong> by {props.notice.authorDisplayName} ·{" "}
        {timestamp(props.notice.createdAt)}
        <Show when={props.notice.affectedRevisionNumber !== null}>
          {` · Revision ${props.notice.affectedRevisionNumber}`}
        </Show>
      </p>
      <Show when={props.notice.viewerMayManage} fallback={<p>{props.notice.body}</p>}>
        <label>
          Notice text
          <textarea value={body()} onInput={(event) => setBody(event.currentTarget.value)} />
        </label>
        <label>
          Affected Revision (optional)
          <input
            inputmode="numeric"
            value={affectedRevision()}
            onInput={(event) => setAffectedRevision(event.currentTarget.value)}
          />
        </label>
        <div class="library-discussion-actions">
          <button type="button" disabled={saving()} onClick={() => void update()}>
            Save impact notice
          </button>
          <button type="button" disabled={saving()} onClick={() => void cancel()}>
            Cancel impact notice
          </button>
        </div>
      </Show>
      <Show when={error()}>
        <p role="alert">{error()}</p>
      </Show>
    </section>
  );
}

/** Text-only stewardship activity. It never exposes Watch recipients or Account identifiers. */
export function LibraryDiscussionPanel(props: LibraryDiscussionPanelProps): JSX.Element {
  const api = useApplicationApi();
  const session = useSessionBootstrap();
  const [discussion, { refetch }] = createResource(
    () => [props.kind, props.publicId] as const,
    ([kind, publicId]) => api.client.getLibraryDiscussion(kind, publicId),
  );
  const [threadBody, setThreadBody] = createSignal("");
  const [noticeBody, setNoticeBody] = createSignal("");
  const [noticeRevision, setNoticeRevision] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal("");

  createEffect(() => {
    if (discussion() === undefined) return;
    const target = document.getElementById(window.location.hash.slice(1));
    target?.scrollIntoView({ block: "start" });
  });

  async function createThread(): Promise<void> {
    setBusy(true);
    setError("");
    try {
      await api.client.createImprovementThread(props.kind, props.publicId, threadBody());
      setThreadBody("");
      await refetch();
    } catch {
      setError("The improvement thread could not be created. Use text only and try again.");
    } finally {
      setBusy(false);
    }
  }

  async function createNotice(): Promise<void> {
    const affectedRevision = parseRevisionNumber(noticeRevision());
    if (noticeRevision().trim() !== "" && affectedRevision === null) {
      setError("Affected Revision must be a positive whole number.");
      return;
    }
    setBusy(true);
    setError("");
    try {
      await api.client.createImpactNotice(
        props.kind,
        props.publicId,
        affectedRevision,
        noticeBody(),
      );
      setNoticeBody("");
      setNoticeRevision("");
      await refetch();
    } catch {
      setError("The impact notice could not be created. Try again.");
    } finally {
      setBusy(false);
    }
  }

  function failedToLoad(): boolean {
    return discussion.error !== undefined;
  }

  function mayParticipate(): boolean {
    const state = session.state();
    return state.kind === "authenticated" && state.session.account.productRole === "instructor";
  }

  return (
    <section
      class="library-discussion-panel"
      aria-label="Library improvement threads and impact notices"
    >
      <h2>Improvement threads and impact notices</h2>
      <Show when={discussion.loading}>
        <p role="status">Loading Library activity...</p>
      </Show>
      <Show when={failedToLoad()}>
        <p role="alert">Library activity could not load. Reload this Library Object.</p>
      </Show>
      <Show when={discussion()}>
        {(view) => (
          <>
            <Show when={mayParticipate()}>
              <section class="library-discussion-create">
                <h3>Start an improvement thread</h3>
                <label>
                  Text-only improvement note
                  <textarea
                    value={threadBody()}
                    onInput={(event) => setThreadBody(event.currentTarget.value)}
                  />
                </label>
                <button type="button" disabled={busy()} onClick={() => void createThread()}>
                  Start thread
                </button>
              </section>
            </Show>
            <Show when={view().viewerMayManage}>
              <section class="library-discussion-create">
                <h3>Post an impact notice</h3>
                <label>
                  Notice text
                  <textarea
                    value={noticeBody()}
                    onInput={(event) => setNoticeBody(event.currentTarget.value)}
                  />
                </label>
                <label>
                  Affected Revision (optional)
                  <input
                    inputmode="numeric"
                    value={noticeRevision()}
                    onInput={(event) => setNoticeRevision(event.currentTarget.value)}
                  />
                </label>
                <button type="button" disabled={busy()} onClick={() => void createNotice()}>
                  Post impact notice
                </button>
              </section>
            </Show>
            <Show when={error()}>
              <p role="alert">{error()}</p>
            </Show>
            <section>
              <h3>Impact notices</h3>
              <RecordDetailList
                ariaLabel="Impact notices"
                emptyState={{ title: "No impact notices." }}
                recordId={(notice) => notice.impactNoticeId}
                rows={view().impactNotices}
                state={{ kind: "ready" }}
                renderRecord={(notice) => (
                  <ImpactNoticeView
                    kind={props.kind}
                    publicId={props.publicId}
                    notice={notice}
                    refresh={refetch}
                  />
                )}
              />
            </section>
            <section>
              <h3>Improvement threads</h3>
              <RecordDetailList
                ariaLabel="Improvement threads"
                emptyState={{ title: "No improvement threads." }}
                recordId={(thread) => thread.threadId}
                rows={view().threads}
                state={{ kind: "ready" }}
                renderRecord={(thread) => (
                  <ThreadView
                    kind={props.kind}
                    publicId={props.publicId}
                    thread={thread}
                    refresh={refetch}
                    mayParticipate={mayParticipate()}
                  />
                )}
              />
            </section>
          </>
        )}
      </Show>
    </section>
  );
}

function ThreadView(props: {
  readonly kind: LibraryObjectDiscussionKind;
  readonly publicId: PublishedQuestionId;
  readonly thread: LibraryImprovementThread;
  readonly refresh: () => unknown;
  readonly mayParticipate: boolean;
}): JSX.Element {
  const api = useApplicationApi();
  const [reply, setReply] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal("");

  async function act(action: () => Promise<void>): Promise<void> {
    setBusy(true);
    setError("");
    try {
      await action();
      await props.refresh();
    } catch {
      setError("This thread could not be updated. Try again.");
    } finally {
      setBusy(false);
    }
  }

  function threadStateLabel(): string {
    switch (props.thread.state) {
      case "open":
        return "Open thread";
      case "resolved":
        return `Resolved ${timestamp(props.thread.resolvedAt)}`;
    }
  }

  function managementAction(): boolean {
    switch (props.thread.state) {
      case "open":
        return true;
      case "resolved":
        return false;
    }
  }

  function managementLabel(): string {
    switch (props.thread.state) {
      case "open":
        return "Resolve thread";
      case "resolved":
        return "Reopen thread";
    }
  }

  return (
    <section
      id={activityAnchor(props.thread.threadId)}
      class="library-improvement-thread"
      data-state={props.thread.state}
    >
      <p>
        <strong>{threadStateLabel()}</strong> · Created for Revision{" "}
        {props.thread.creationRevisionNumber}
      </p>
      <For each={props.thread.posts}>
        {(post) => (
          <PostView
            kind={props.kind}
            publicId={props.publicId}
            post={post}
            refresh={props.refresh}
            mayEdit={props.mayParticipate && post.viewerMayEdit}
          />
        )}
      </For>
      <div class="library-discussion-actions">
        <Show when={props.mayParticipate}>
          <label>
            Reply
            <textarea value={reply()} onInput={(event) => setReply(event.currentTarget.value)} />
          </label>
          <button
            type="button"
            disabled={busy()}
            onClick={() =>
              void act(async () => {
                await api.client.replyToImprovementThread(
                  props.kind,
                  props.publicId,
                  props.thread.threadId,
                  reply(),
                );
                setReply("");
              })
            }
          >
            Reply
          </button>
        </Show>
        <Show when={props.thread.viewerMayResolve}>
          <button
            type="button"
            disabled={busy()}
            onClick={() =>
              void act(() =>
                api.client.setImprovementThreadResolved(
                  props.kind,
                  props.publicId,
                  props.thread.threadId,
                  managementAction(),
                ),
              )
            }
          >
            {managementLabel()}
          </button>
        </Show>
      </div>
      <Show when={error()}>
        <p role="alert">{error()}</p>
      </Show>
    </section>
  );
}

function PostView(props: {
  readonly kind: LibraryObjectDiscussionKind;
  readonly publicId: PublishedQuestionId;
  readonly post: LibraryImprovementPost;
  readonly refresh: () => unknown;
  readonly mayEdit: boolean;
}): JSX.Element {
  const api = useApplicationApi();
  const [body, setBody] = createSignal(props.post.body);
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal("");
  async function save(): Promise<void> {
    setSaving(true);
    setError("");
    try {
      await api.client.editOwnImprovementPost(
        props.kind,
        props.publicId,
        props.post.postId,
        body(),
      );
      await props.refresh();
    } catch {
      setError("Your post could not be updated. Try again.");
    } finally {
      setSaving(false);
    }
  }
  return (
    <section id={activityAnchor(props.post.postId)} class="library-improvement-post">
      <p>
        <strong>{props.post.authorDisplayName}</strong> · {timestamp(props.post.createdAt)}
        <Show when={props.post.updatedAt !== null}> · Edited</Show>
      </p>
      <Show when={props.mayEdit} fallback={<p>{props.post.body}</p>}>
        <label>
          Your post
          <textarea value={body()} onInput={(event) => setBody(event.currentTarget.value)} />
        </label>
        <button type="button" disabled={saving()} onClick={() => void save()}>
          Save post
        </button>
      </Show>
      <Show when={error()}>
        <p role="alert">{error()}</p>
      </Show>
    </section>
  );
}
