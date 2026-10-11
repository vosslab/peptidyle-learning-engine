//! Exact Question Revision licensing.

use serde::{Deserialize, Serialize};

/// The exact versioned SPDX grant under which one Question Revision may be reused.
///
/// Question Library publication requires an adaptation-permitting Creative
/// Commons grant. The closed value set keeps draft validation, search, export,
/// and the database publication rule in agreement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum QuestionLicense {
    /// Creative Commons public-domain dedication 1.0.
    #[serde(rename = "CC0-1.0")]
    Cc0_1_0,
    /// Creative Commons Attribution 4.0.
    #[serde(rename = "CC-BY-4.0")]
    CcBy4_0,
    /// Creative Commons Attribution-ShareAlike 4.0.
    #[serde(rename = "CC-BY-SA-4.0")]
    CcBySa4_0,
}
