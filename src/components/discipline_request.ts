// Course Discipline requests. A request stores a name. It does not create a Discipline.

import type { ContentDisciplineRequest } from "../api/content_classification";
import type { ContentClassificationItem } from "../api/content_classification";
import { DecodeError } from "../api/decoder";
import { decodeContentDisciplineName } from "../api/decoders/content_classification";

export interface DisciplineRequestSender {
  readonly requestContentDiscipline: (name: string) => Promise<ContentDisciplineRequest>;
}

export interface DisciplineRequestFulfillment {
  readonly fulfillDisciplineRequest: (uuid: string) => Promise<ContentClassificationItem>;
}

export class DisciplineRequestInputError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "DisciplineRequestInputError";
  }
}

/** Records the requested name for a Sysadmin. The caller does not create the Discipline. */
export async function submitDisciplineRequest(
  client: DisciplineRequestSender,
  name: string,
): Promise<string> {
  let requestedName: string;
  try {
    // ASVS 2.2.1: reject an empty, oversized, or control-bearing name before transport.
    requestedName = decodeContentDisciplineName(name);
  } catch (error) {
    if (error instanceof DecodeError) {
      throw new DisciplineRequestInputError(
        "Enter a Discipline name of 1 through 120 characters without control characters.",
      );
    }
    throw error;
  }
  const receipt = await client.requestContentDiscipline(requestedName);
  return `Discipline request recorded for ${receipt.requestedName}. A Sysadmin creates Disciplines.`;
}

/** A Sysadmin fulfills a request in one server transaction. */
export async function createDisciplineFromRequest(
  client: DisciplineRequestFulfillment,
  request: ContentDisciplineRequest,
): Promise<ContentClassificationItem> {
  return client.fulfillDisciplineRequest(request.uuid);
}
