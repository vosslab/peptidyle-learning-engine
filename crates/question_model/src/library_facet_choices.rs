//! Read-only Question Library choice continuation contract.

use serde::{Deserialize, Serialize};

/// Closed set of enumerated Library filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibraryFacetField {
    Kind,
    AuthorNames,
    Backends,
    Tags,
    NameSubjects,
    NameTopics,
    DisciplineUuids,
    SubjectUuids,
    TopicUuids,
    SubtopicUuids,
    BloomCognitiveProcesses,
    BloomKnowledgeDimensions,
    QuestionTypes,
    Capabilities,
    QuestionLicenses,
}

impl LibraryFacetField {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kind => "kind",
            Self::AuthorNames => "authorNames",
            Self::Backends => "backends",
            Self::Tags => "tags",
            Self::NameSubjects => "nameSubjects",
            Self::NameTopics => "nameTopics",
            Self::DisciplineUuids => "disciplineUuids",
            Self::SubjectUuids => "subjectUuids",
            Self::TopicUuids => "topicUuids",
            Self::SubtopicUuids => "subtopicUuids",
            Self::BloomCognitiveProcesses => "bloomCognitiveProcesses",
            Self::BloomKnowledgeDimensions => "bloomKnowledgeDimensions",
            Self::QuestionTypes => "questionTypes",
            Self::Capabilities => "capabilities",
            Self::QuestionLicenses => "questionLicenses",
        }
    }
}

impl std::str::FromStr for LibraryFacetField {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "kind" => Self::Kind,
            "authorNames" => Self::AuthorNames,
            "backends" => Self::Backends,
            "tags" => Self::Tags,
            "nameSubjects" => Self::NameSubjects,
            "nameTopics" => Self::NameTopics,
            "disciplineUuids" => Self::DisciplineUuids,
            "subjectUuids" => Self::SubjectUuids,
            "topicUuids" => Self::TopicUuids,
            "subtopicUuids" => Self::SubtopicUuids,
            "bloomCognitiveProcesses" => Self::BloomCognitiveProcesses,
            "bloomKnowledgeDimensions" => Self::BloomKnowledgeDimensions,
            "questionTypes" => Self::QuestionTypes,
            "capabilities" => Self::Capabilities,
            "questionLicenses" => Self::QuestionLicenses,
            _ => return Err(()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryFacetChoice {
    pub key: String,
    pub label: String,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryFacetChoicePage {
    pub choices: Vec<LibraryFacetChoice>,
    pub selected: Vec<LibraryFacetChoice>,
    pub next_cursor: Option<String>,
}
