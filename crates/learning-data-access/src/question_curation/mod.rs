//! Server-owned curation aggregates for the shared question catalog.
//!
//! Each child module owns one durable relation with a distinct lifecycle and
//! persistence boundary. Browser-safe projections and protected operations
//! belong to later layers.

mod collection;
mod collection_share;
mod saved_search;
mod star;
mod watch;

pub use collection::{
    NamedQuestionCollection, NamedQuestionCollectionError, NamedQuestionCollectionId,
    NamedQuestionCollectionReplacementOutcome, NamedQuestionCollectionRevision,
};
pub use collection_share::{
    NamedQuestionCollectionShare, NamedQuestionCollectionShareError,
    NamedQuestionCollectionShareGrantOutcome, NamedQuestionCollectionShareRevokeOutcome,
    NamedQuestionCollectionShareState,
};
pub use saved_search::{
    NamedQuestionSavedSearch, NamedQuestionSavedSearchError, NamedQuestionSavedSearchId,
    NamedQuestionSavedSearchReplacementOutcome, NamedQuestionSavedSearchRevision,
};
pub use star::QuestionStar;
pub use watch::{QuestionWatch, QuestionWatchNoticeKind, QuestionWatchTarget};
