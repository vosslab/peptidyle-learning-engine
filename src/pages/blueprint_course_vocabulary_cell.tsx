// Names for optional Blueprint classification columns, sharing each parent vocabulary request.
import { createResource, type JSX } from "solid-js";
import type {
  ContentClassificationClient,
  ContentClassificationItem,
} from "../api/content_classification";

type Level = "discipline" | "subject" | "topic" | "subtopic";

export function blueprintVocabularyCell(
  client: ContentClassificationClient,
): (level: Level, parentUuid: string | null, uuid: string | null) => JSX.Element {
  const lists = new Map<string, Promise<ReadonlyArray<ContentClassificationItem>>>();
  function load(
    level: Level,
    parentUuid: string | null,
  ): Promise<ReadonlyArray<ContentClassificationItem>> {
    const key = `${level}:${parentUuid}`;
    const existing = lists.get(key);
    if (existing) return existing;
    const pending =
      level === "discipline"
        ? client.listDisciplinesIncludingRetired()
        : level === "subject"
          ? client.listSubjects(parentUuid!)
          : level === "topic"
            ? client.listTopics(parentUuid!)
            : client.listSubtopics(parentUuid!);
    lists.set(key, pending);
    return pending;
  }
  return (level, parentUuid, uuid): JSX.Element => {
    if ((parentUuid === null && level !== "discipline") || uuid === null) return "";
    const parent = parentUuid;
    const selected = uuid;
    function Cell(): JSX.Element {
      const [items] = createResource(
        () => load(level, parent),
        (pending) => pending,
      );
      const item = (): ContentClassificationItem | undefined =>
        items()?.find((candidate) => candidate.uuid === selected);
      return <>{item()?.name ?? (items.loading ? "Loading..." : "Name unavailable")}</>;
    }
    return <Cell />;
  };
}
