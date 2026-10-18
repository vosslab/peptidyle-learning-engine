import type { BlueprintCourseClient } from "../../api/blueprint_course";
import type { ContentClassificationClient } from "../../api/content_classification";
import type { BlueprintChangeProposalClient } from "../../api/blueprint_change_proposal";
import type {
  QuestionPickerProps,
  QuestionPickerSource,
  QuestionPickerSourceRepository,
} from "../question_picker";

export interface BlueprintCoursesWorkspaceProps {
  readonly client: BlueprintCourseClient & ContentClassificationClient;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
  readonly loadQuestionInspection: QuestionPickerProps["loadQuestionInspection"];
  readonly questionRevisionPreviewDocumentUrl: QuestionPickerProps["questionRevisionPreviewDocumentUrl"];
  readonly questionImageUrl: QuestionPickerProps["questionImageUrl"];
}

export interface BlueprintCourseDetailWorkspaceProps extends BlueprintCoursesWorkspaceProps {
  readonly proposalClient?: BlueprintCourseClient & BlueprintChangeProposalClient;
  readonly blueprintCourseId: string;
}
