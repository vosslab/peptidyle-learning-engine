//! Conversion between durable WeBWorK controls and one issued presentation.
//!
//! The mapping keys server-only upstream field/value controls by learner-visible
//! rendered IDs. This narrow private bridge restores durable controls for a
//! grade from the exact receipt-bound presentation, never from a rerender.

use std::collections::BTreeMap;

use question_model::presentation::{PresentationV1, RenderedItemIdV1, RenderedItemRoleV1};
use question_model::response::ChoiceId;

use crate::run::RunBackendError;

/// Converts issuance-only durable item identities into the exact rendered IDs
/// minted for one presentation before the private mapping is persisted.
pub(crate) fn persist_replay_mapping(
    replay: adapter_webwork::renderer_contract::WebworkReplayMappingV1,
    presentation: &PresentationV1,
) -> Result<learning_data_access::WebworkReplayMappingV1, RunBackendError> {
    use adapter_webwork::renderer_contract::WebworkReplayMappingV1 as AdapterReplay;
    use learning_data_access::{WebworkReplayControlV1, WebworkReplayMatchPromptV1};

    match replay {
        AdapterReplay::SingleChoice { controls } => {
            let mut items = Vec::with_capacity(controls.len());
            for (choice, control) in controls {
                items.push(WebworkReplayControlV1 {
                    item: rendered_id_for(presentation, &choice, RenderedItemRoleV1::Choice)?,
                    field: control.field,
                    value: control.value,
                });
            }
            Ok(learning_data_access::WebworkReplayMappingV1::SingleChoice { items })
        }
        AdapterReplay::Matching { prompts } => {
            let mut items = Vec::with_capacity(prompts.len());
            for (prompt, mapping) in prompts {
                let mut choices = Vec::with_capacity(mapping.choices.len());
                for (choice, value) in mapping.choices {
                    choices.push(WebworkReplayControlV1 {
                        item: rendered_id_for(
                            presentation,
                            &choice,
                            RenderedItemRoleV1::MatchChoice,
                        )?,
                        field: mapping.field.clone(),
                        value,
                    });
                }
                items.push(WebworkReplayMatchPromptV1 {
                    prompt: rendered_id_for(
                        presentation,
                        &prompt,
                        RenderedItemRoleV1::MatchPrompt,
                    )?,
                    field: mapping.field,
                    choices,
                });
            }
            Ok(learning_data_access::WebworkReplayMappingV1::Matching { items })
        }
    }
}

fn rendered_id_for(
    presentation: &PresentationV1,
    durable: &ChoiceId,
    role: RenderedItemRoleV1,
) -> Result<RenderedItemIdV1, RunBackendError> {
    let mut matches = presentation
        .item_bindings
        .iter()
        .filter(|binding| binding.role == role && binding.durable_id == durable.as_str());
    let Some(binding) = matches.next() else {
        return Err(RunBackendError::Invalid(
            "WeBWorK replay item is absent from the issued presentation".into(),
        ));
    };
    if matches.next().is_some() {
        return Err(RunBackendError::Invalid(
            "WeBWorK replay item is ambiguous in the issued presentation".into(),
        ));
    }
    Ok(binding.rendered.clone())
}

pub(crate) fn restore_replay_mapping(
    replay: learning_data_access::WebworkReplayMappingV1,
    presentation: &PresentationV1,
) -> Result<adapter_webwork::renderer_contract::WebworkReplayMappingV1, RunBackendError> {
    use adapter_webwork::renderer_contract::{
        UpstreamControlV1, UpstreamMatchPromptV1, WebworkReplayMappingV1,
    };

    match replay {
        learning_data_access::WebworkReplayMappingV1::SingleChoice { items } => {
            let mut controls = BTreeMap::new();
            for item in items {
                let choice = durable_id_for(presentation, &item.item, RenderedItemRoleV1::Choice)?;
                if controls
                    .insert(
                        choice,
                        UpstreamControlV1 {
                            field: item.field,
                            value: item.value,
                        },
                    )
                    .is_some()
                {
                    return Err(RunBackendError::Unavailable(
                        "WeBWorK replay repeats a durable choice".into(),
                    ));
                }
            }
            Ok(WebworkReplayMappingV1::SingleChoice { controls })
        }
        learning_data_access::WebworkReplayMappingV1::Matching { items } => {
            let mut prompts = BTreeMap::new();
            for item in items {
                let prompt =
                    durable_id_for(presentation, &item.prompt, RenderedItemRoleV1::MatchPrompt)?;
                let mut choices = BTreeMap::new();
                for choice in item.choices {
                    let durable = durable_id_for(
                        presentation,
                        &choice.item,
                        RenderedItemRoleV1::MatchChoice,
                    )?;
                    if choices.insert(durable, choice.value).is_some() {
                        return Err(RunBackendError::Unavailable(
                            "WeBWorK replay repeats a durable matching choice".into(),
                        ));
                    }
                }
                if prompts
                    .insert(
                        prompt,
                        UpstreamMatchPromptV1 {
                            field: item.field,
                            choices,
                        },
                    )
                    .is_some()
                {
                    return Err(RunBackendError::Unavailable(
                        "WeBWorK replay repeats a durable matching prompt".into(),
                    ));
                }
            }
            Ok(WebworkReplayMappingV1::Matching { prompts })
        }
    }
}

fn durable_id_for(
    presentation: &PresentationV1,
    rendered: &RenderedItemIdV1,
    role: RenderedItemRoleV1,
) -> Result<ChoiceId, RunBackendError> {
    let mut matches = presentation
        .item_bindings
        .iter()
        .filter(|binding| binding.role == role && binding.rendered == *rendered);
    let Some(binding) = matches.next() else {
        return Err(RunBackendError::Unavailable(
            "stored WeBWorK rendered item is absent from its presentation".into(),
        ));
    };
    if matches.next().is_some() {
        return Err(RunBackendError::Unavailable(
            "stored WeBWorK rendered item is ambiguous in its presentation".into(),
        ));
    }
    Ok(ChoiceId::new(binding.durable_id.clone()))
}
