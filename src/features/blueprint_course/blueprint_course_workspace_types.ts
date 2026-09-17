import type { BlueprintCourseClient } from "../../api/blueprint_course";
import type { BlueprintChangeProposalClient } from "../../api/blueprint_change_proposal";
import type { QuestionPickerSource, QuestionPickerSourceRepository } from "../question_picker";

export interface BlueprintCoursesWorkspaceProps {
  readonly client: BlueprintCourseClient;
  readonly proposalClient?: BlueprintCourseClient & BlueprintChangeProposalClient;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
}

export interface BlueprintCourseDetailWorkspaceProps extends BlueprintCoursesWorkspaceProps {
  readonly blueprintCourseRef: string;
}
