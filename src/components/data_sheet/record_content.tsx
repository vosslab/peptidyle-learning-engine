import { For, Show, createMemo, type Accessor, type JSX } from "solid-js";

import { assessmentTypePresentation } from "../../assessment_type_presentation";
import type { AssessmentType } from "../../../generated/api/AssessmentType";
import type { CourseClassification } from "../../../generated/api/CourseClassification";
import type { QuestionAuthor } from "../../../generated/api/QuestionAuthor";
import { RibbonIcon } from "../../ribbon/ribbon_icon";
import { CopyableQuestionId } from "../copyable_question_id";
import { CourseClassificationSummary } from "../course_classification_summary";
import { InstructorProfileLink } from "../instructor_profile_link";
import { RecordActionControl } from "./record_action_control";

export type RecordFact =
  | {
      readonly kind: "text";
      readonly value: string;
      readonly label?: string;
      /** A visible table column supplies this fact; semantic views retain it. */
      readonly fieldId?: string;
    }
  | {
      readonly kind: "time";
      readonly value: string;
      readonly dateTime: string;
      readonly label?: string;
    }
  | {
      readonly kind: "link";
      readonly label: string;
      readonly href: string;
      readonly onFollow?: (
        event: MouseEvent & { readonly currentTarget: HTMLAnchorElement },
      ) => void;
    }
  | { readonly kind: "assessmentType"; readonly value: AssessmentType }
  | { readonly kind: "questionId"; readonly questionTitle: string; readonly displayId: string }
  | { readonly kind: "courseClassification"; readonly value: CourseClassification }
  | {
      readonly kind: "instructorProfile";
      readonly accountId: string;
      readonly displayName: string;
      readonly label?: string;
    }
  | {
      readonly kind: "questionAuthors";
      readonly authors: ReadonlyArray<QuestionAuthor>;
      readonly label?: string;
    };

export type RecordMedia = { readonly src: string; readonly alt: string };

type RecordActionBase = {
  readonly id: string;
  readonly label: string;
  readonly primary?: boolean;
  readonly title?: string;
};

export type RecordAction =
  | (RecordActionBase & {
      readonly kind: "link";
      readonly href: string;
      readonly onFollow?: (
        event: MouseEvent & { readonly currentTarget: HTMLAnchorElement },
      ) => void;
      readonly ref?: (element: HTMLAnchorElement) => void;
    })
  | (RecordActionBase & {
      readonly kind: "command";
      readonly onClick: (event: MouseEvent & { readonly currentTarget: HTMLButtonElement }) => void;
      readonly disabled?: boolean;
      readonly pressed?: boolean;
      readonly expanded?: boolean;
      readonly controls?: string;
      readonly ref?: (element: HTMLButtonElement) => void;
    });

export type RecordContent = {
  readonly title: string;
  readonly description?: string;
  readonly details: ReadonlyArray<RecordFact>;
  readonly media?: RecordMedia;
  /** Lazy content mounts only in presentations that show previews. */
  readonly preview?: () => JSX.Element;
  readonly actions: ReadonlyArray<RecordAction>;
};

function RecordFactView(props: { readonly fact: RecordFact }): JSX.Element {
  const content = (): JSX.Element => {
    switch (props.fact.kind) {
      case "text":
        return <span>{props.fact.value}</span>;
      case "time":
        return <time datetime={props.fact.dateTime}>{props.fact.value}</time>;
      case "link":
        return (
          <a href={props.fact.href} target="_blank" rel="noopener" onClick={props.fact.onFollow}>
            {props.fact.label}
          </a>
        );
      case "assessmentType": {
        const type = assessmentTypePresentation(props.fact.value);
        return (
          <span class="record-list__assessment-type" style={{ color: `var(${type.colorToken})` }}>
            <RibbonIcon glyph={type.icon} />
            {type.label}
          </span>
        );
      }
      case "questionId":
        return (
          <CopyableQuestionId
            questionTitle={props.fact.questionTitle}
            displayId={props.fact.displayId}
            presentation="compact"
          />
        );
      case "courseClassification":
        return <CourseClassificationSummary value={props.fact.value} />;
      case "instructorProfile":
        return (
          <InstructorProfileLink
            accountId={props.fact.accountId}
            displayName={props.fact.displayName}
          />
        );
      case "questionAuthors":
        return (
          <For each={props.fact.authors}>
            {(author, index) => (
              <>
                {index() > 0 ? ", " : ""}
                <Show when={author.accountId} fallback={author.displayName}>
                  {(accountId) => (
                    <InstructorProfileLink
                      accountId={accountId()}
                      displayName={author.displayName}
                    />
                  )}
                </Show>
              </>
            )}
          </For>
        );
    }
  };
  const labeled =
    (props.fact.kind === "text" ||
      props.fact.kind === "time" ||
      props.fact.kind === "instructorProfile" ||
      props.fact.kind === "questionAuthors") &&
    props.fact.label !== undefined;
  return (
    <div
      class="record-list__fact"
      classList={{
        "record-list__fact--course-classification": props.fact.kind === "courseClassification",
      }}
    >
      <Show when={labeled}>
        <span class="record-list__fact-label">
          {(props.fact as { readonly label?: string }).label}
        </span>
      </Show>
      {content()}
    </div>
  );
}

export function normalizeRecordActions(
  recordId: string,
  actions: ReadonlyArray<RecordAction>,
): { readonly ids: ReadonlyArray<string>; readonly byId: ReadonlyMap<string, RecordAction> } {
  const byId = new Map<string, RecordAction>();
  for (const action of actions) {
    if (byId.has(action.id))
      throw new Error(`RecordItem record ${recordId} has duplicate action ID ${action.id}.`);
    byId.set(action.id, action);
  }
  return { ids: [...byId.keys()], byId };
}

/** Canonical identity text shared by semantic records and the wrapping table. */
export function RecordIdentityContent(props: {
  readonly content: Accessor<RecordContent>;
}): JSX.Element {
  const meaningfulDescription = (): string | undefined => {
    const { title, description } = props.content();
    return description === undefined ||
      description.trim().length === 0 ||
      description.trim() === title.trim()
      ? undefined
      : description;
  };
  return (
    <div class="record-list__identity">
      <h3 class="record-list__title">{props.content().title}</h3>
      <Show when={meaningfulDescription()}>
        {(description) => <p class="record-list__description">{description()}</p>}
      </Show>
    </div>
  );
}

/** Shared preview slot; callers mount it only outside Compact. */
export function RecordPreview(props: { readonly content: Accessor<RecordContent> }): JSX.Element {
  return (
    <Show when={props.content().preview}>
      {(preview) => <div class="record-list__preview">{preview()()}</div>}
    </Show>
  );
}

/** Canonical facts, rendered once in List's Details column. */
export function RecordDetails(props: { readonly content: Accessor<RecordContent> }): JSX.Element {
  return (
    <Show when={props.content().details.length > 0}>
      <div class="record-list__facts">
        <For each={props.content().details}>{(fact) => <RecordFactView fact={fact} />}</For>
      </div>
    </Show>
  );
}

/** One record's shared semantic content for List, Cards, Visual Boxes, and editor sequences. */
export function RecordSemanticContent(props: {
  readonly recordId: string;
  readonly content: Accessor<RecordContent>;
  readonly renderBody?: () => JSX.Element;
}): JSX.Element {
  const actions = createMemo(() => normalizeRecordActions(props.recordId, props.content().actions));
  return (
    <div class="record-list__semantic-frame">
      <Show when={props.content().media}>
        {(media) => (
          <figure class="record-list__media">
            <img
              src={media().src}
              alt={media().alt}
              onLoad={(event) =>
                event.currentTarget.closest(".record-list__media")?.removeAttribute("hidden")
              }
              onError={(event) =>
                event.currentTarget.closest(".record-list__media")?.setAttribute("hidden", "")
              }
            />
          </figure>
        )}
      </Show>
      <div class="record-list__content">
        <RecordIdentityContent content={props.content} />
        <RecordPreview content={props.content} />
        <RecordDetails content={props.content} />
        <Show when={props.content().actions.length > 0}>
          <div class="record-list__actions">
            <For each={actions().ids}>
              {(actionId) => <RecordActionControl action={() => actions().byId.get(actionId)!} />}
            </For>
          </div>
        </Show>
        {props.renderBody === undefined ? undefined : (
          <div class="record-list__body">{props.renderBody()}</div>
        )}
      </div>
    </div>
  );
}
