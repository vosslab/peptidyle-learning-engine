//! Strict declaration of untrusted, browser-isolated native Question behavior.
//!
//! This module deliberately records source only. It neither parses nor runs
//! JavaScript, exposes a seed, nor grants access to PLE APIs or browser state.
//! The browser sandbox is the separate C303 boundary.

use std::collections::HashSet;

use question_model::{AuthorContentLibraryId, AuthorContentPresentation};
use serde::{Deserialize, Serialize};

use super::super::{PleQuestionJsonError, invalid, validate_bounded_text};

const MAX_AUTHOR_SCRIPT_CHARS: usize = 65_536;
const MAX_AUTHOR_SCRIPT_LIBRARIES: usize = 16;
const MAX_AUTHOR_SCRIPT_LIBRARY_ID_CHARS: usize = 128;

/// One author-declared, untrusted JavaScript source and its reviewed runtime
/// library names. `rdkit` is a supported declaration for chemistry rendering.
/// The server-owned registry, rather than the source document, selects any
/// reviewed runtime assets.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PleQuestionJsonAuthorScript {
    source: String,
    #[serde(default)]
    libraries: Vec<PleQuestionJsonAuthorScriptLibrary>,
}

/// Closed, reviewable browser-library names available to native author code.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum PleQuestionJsonAuthorScriptLibrary {
    Rdkit,
}

pub(super) fn validate_author_script(
    script: Option<&PleQuestionJsonAuthorScript>,
) -> Result<(), PleQuestionJsonError> {
    let Some(script) = script else {
        return Ok(());
    };
    validate_bounded_text(
        "author script source",
        &script.source,
        MAX_AUTHOR_SCRIPT_CHARS,
    )?;
    if script.libraries.len() > MAX_AUTHOR_SCRIPT_LIBRARIES {
        return invalid("author script library count is outside the supported range");
    }
    let mut unique = HashSet::new();
    for library in &script.libraries {
        // The stable serialized spelling is part of the strict source shape;
        // validating it here keeps a future enum expansion deliberately bounded.
        let id = match library {
            PleQuestionJsonAuthorScriptLibrary::Rdkit => "rdkit",
        };
        validate_bounded_text(
            "author script library",
            id,
            MAX_AUTHOR_SCRIPT_LIBRARY_ID_CHARS,
        )?;
        if !unique.insert(library) {
            return invalid("author script libraries must be unique");
        }
    }
    Ok(())
}

pub(super) fn compile_author_content(
    script: Option<&PleQuestionJsonAuthorScript>,
) -> Result<Option<AuthorContentPresentation>, PleQuestionJsonError> {
    let Some(script) = script else {
        return Ok(None);
    };
    AuthorContentPresentation::new(
        script.source.clone(),
        script
            .libraries
            .iter()
            .map(|library| match library {
                PleQuestionJsonAuthorScriptLibrary::Rdkit => AuthorContentLibraryId::Rdkit,
            })
            .collect(),
    )
    .map(Some)
    .map_err(|message| PleQuestionJsonError::InvalidDocument(message.to_string()))
}
