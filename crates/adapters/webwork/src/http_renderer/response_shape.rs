//! Fixed upstream JSON member set for the shipped renderer response.

/// The renderer response is closed so unexpected upstream material refuses.
pub(super) const RESPONSE_KEYS: &[&str] = &[
    "head_part001",
    "head_part010",
    "head_part300",
    "head_part400",
    "head_part999",
    "body_part001",
    "body_part100",
    "body_part300",
    "body_part500",
    "body_part530",
    "body_part550",
    "body_part590",
    "body_part650",
    "body_part700",
    "body_part999",
    "hidden_input_field",
    "score",
    "real_webwork_SITE_URL",
    "real_webwork_FORM_ACTION_URL",
    "internal_problem_lang_and_dir",
];
