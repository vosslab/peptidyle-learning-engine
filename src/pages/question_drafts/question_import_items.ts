// Each converted item saves its original source, then binds its own supplied media to its Draft.

import type { NamedBytes } from "../../../assets/qpm_wasm/src/index.js";
import type { QuestionDraftClient } from "../../api/question_draft_client";
import { ApiRequestError } from "../../api/http_client/error";
import {
  PleQuestionJsonRequestError,
  type PleQuestionJsonClient,
} from "../../features/ple_question_json_authoring/question_json_client";
import { isQuestionSourceRelativeName } from "../../question_image_source";

export type QuestionImportResult = {
  readonly name: string;
  readonly status: "created" | "rejected" | "failed";
  readonly message: string;
  readonly href?: string;
};

function record(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

/** Inventory only actual display fields; externalResources and Answer Keys are not bindings. */
export function questionImportImageNames(source: string): ReadonlyArray<string> {
  const question = record(JSON.parse(source) as unknown);
  const response = record(question.response);
  const feedback = record(question.feedback);
  const display = [
    question.prompt,
    question.questionHint,
    feedback.correct,
    feedback.incorrect,
    response.unit,
  ];
  for (const key of ["choices", "prompts", "items", "blanks", "regions"]) {
    const items: unknown = response[key];
    if (!Array.isArray(items)) continue;
    for (const item of items) {
      const fields = record(item);
      display.push(fields.text, fields.label, fields.feedback);
    }
  }
  const names = new Set<string>();
  for (const html of display) {
    if (typeof html !== "string") continue;
    // ASVS 1.3.1: inspect an inert template without attaching authored HTML to a document.
    const template = document.createElement("template");
    template.innerHTML = html;
    for (const image of template.content.querySelectorAll("img[src]")) {
      const name = image.getAttribute("src") ?? "";
      if (!isQuestionSourceRelativeName(name))
        throw new Error(
          `Image reference ${JSON.stringify(name)} is not a content-relative filename.`,
        );
      names.add(name);
    }
  }
  return Array.from(names);
}

function imageMediaType(name: string): string {
  const extension = name.slice(name.lastIndexOf(".") + 1).toLowerCase();
  switch (extension) {
    case "png":
      return "image/png";
    case "jpg":
    case "jpeg":
      return "image/jpeg";
    case "webp":
      return "image/webp";
    case "svg":
      return "image/svg+xml";
    default:
      throw new Error(`Image ${name} must be a PNG, JPEG, WebP, or SVG file.`);
  }
}

/** A failed media binding retains the saved Draft and later items continue independently. */
export async function saveQuestionImportItem(
  item: NamedBytes,
  files: ReadonlyArray<NamedBytes>,
  drafts: QuestionDraftClient,
  images: Pick<PleQuestionJsonClient, "uploadQuestionImage">,
): Promise<QuestionImportResult> {
  let href: string | undefined;
  try {
    const source = new TextDecoder("utf-8", { fatal: true }).decode(item.bytes);
    // ASVS 2.2.2: ordinary Draft storage owns validation and authorization. Preserve exact source.
    const draft = await drafts.createDraft({ format: "pleQuestionJson", source });
    href = `/authoring/drafts/${encodeURIComponent(draft.draftQuestionId)}`;
    for (const name of questionImportImageNames(source)) {
      const matches = files.filter((file) => file.name === name);
      if (matches.length !== 1)
        throw new Error(
          `Image ${name} ${matches.length === 0 ? "was not supplied" : "was supplied more than once"}.`,
        );
      const media = matches[0]!;
      const image = new Blob([new Uint8Array(media.bytes)], { type: imageMediaType(name) });
      try {
        await images.uploadQuestionImage(
          draft.draftQuestionId,
          image,
          draft.draftQuestionEditNumber,
          undefined,
          name,
        );
      } catch (error: unknown) {
        throw Object.assign(
          new Error(
            `Image ${name} could not be bound: ${error instanceof Error ? error.message : "upload failed"}.`,
          ),
          { cause: error },
        );
      }
    }
    return { name: item.name, status: "created", message: "Saved as a Draft", href };
  } catch (error: unknown) {
    return {
      name: item.name,
      status:
        href === undefined &&
        (error instanceof ApiRequestError || error instanceof PleQuestionJsonRequestError)
          ? "rejected"
          : "failed",
      message: `${href === undefined ? "" : "Draft saved; "}${error instanceof Error ? error.message : "This question could not be saved."}`,
      ...(href === undefined ? {} : { href }),
    };
  }
}
