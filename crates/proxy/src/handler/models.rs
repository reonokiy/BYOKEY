//! Models listing handler — returns available models in `OpenAI` format.

use axum::{Json, extract::State};
use byokey_provider::all_models;
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::AppState;

/// OpenAI-compatible model list response.
#[derive(Serialize, ToSchema)]
pub struct ModelsResponse {
    pub object: String,
    pub data: Vec<ModelEntry>,
}

/// A single model entry.
#[derive(Serialize, ToSchema)]
pub struct ModelEntry {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
}

/// Handles `GET /v1/models` requests.
///
/// Returns an OpenAI-compatible model list from the unified registry.
/// For models available on multiple providers, both unqualified (primary)
/// and qualified (`provider/model`) forms are listed.
#[utoipa::path(
    get,
    path = "/v1/models",
    responses((status = 200, body = ModelsResponse)),
    tag = "management"
)]
pub async fn list_models(State(state): State<Arc<AppState>>) -> Json<ModelsResponse> {
    let mut data: Vec<ModelEntry> = Vec::new();
    let config = state.config.load();

    for entry in all_models() {
        let Some(primary_provider) = entry.providers.first() else {
            continue;
        };

        let primary_pc = config
            .providers
            .get(primary_provider)
            .cloned()
            .unwrap_or_default();
        let primary_enabled =
            primary_pc.enabled && !config.is_model_excluded(primary_provider, entry.id);

        // List the unqualified model under its primary provider if enabled.
        if primary_enabled {
            let aliases = config.model_alias.get(primary_provider);
            let alias_entry = aliases.and_then(|a| a.iter().find(|ae| ae.name == entry.id));

            if let Some(ae) = alias_entry {
                data.push(ModelEntry {
                    id: ae.alias.clone(),
                    object: "model".into(),
                    created: 0,
                    owned_by: primary_provider.to_string(),
                });
                if ae.fork {
                    data.push(ModelEntry {
                        id: entry.id.to_string(),
                        object: "model".into(),
                        created: 0,
                        owned_by: primary_provider.to_string(),
                    });
                }
            } else {
                data.push(ModelEntry {
                    id: entry.id.to_string(),
                    object: "model".into(),
                    created: 0,
                    owned_by: primary_provider.to_string(),
                });
            }
        }

        // Emit qualified alternatives for all providers on multi-provider
        // models (including the primary, for explicit discoverability).
        if entry.providers.len() > 1 {
            for alt_provider in entry.providers {
                let alt_pc = config
                    .providers
                    .get(alt_provider)
                    .cloned()
                    .unwrap_or_default();
                if !alt_pc.enabled {
                    continue;
                }
                if config.is_model_excluded(alt_provider, entry.id) {
                    continue;
                }
                data.push(ModelEntry {
                    id: format!("{}/{}", alt_provider, entry.id),
                    object: "model".into(),
                    created: 0,
                    owned_by: alt_provider.to_string(),
                });
            }
        }
    }

    Json(ModelsResponse {
        object: "list".into(),
        data,
    })
}
