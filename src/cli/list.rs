use anyhow::{bail, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Table};
use serde::Serialize;

use crate::api;

#[derive(Serialize)]
struct ProviderInfo {
    id: String,
    name: String,
    models_count: usize,
}

#[derive(Serialize)]
struct ModelInfo {
    id: String,
    name: String,
    provider: String,
    context: String,
    cost: String,
    capabilities: String,
}

pub fn providers(json: bool) -> Result<()> {
    let providers = api::fetch_providers()?;

    let mut infos: Vec<ProviderInfo> = providers
        .iter()
        .map(|(id, p)| ProviderInfo {
            id: id.clone(),
            name: p.name.clone(),
            models_count: p.models.len(),
        })
        .collect();

    infos.sort_by(|a, b| a.id.cmp(&b.id));

    if json {
        println!("{}", serde_json::to_string_pretty(&infos)?);
    } else {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL_CONDENSED);
        table.set_header(vec!["ID", "Name", "Models"]);

        for info in infos {
            table.add_row(vec![info.id, info.name, info.models_count.to_string()]);
        }

        println!("{table}");
    }

    Ok(())
}

pub fn models(provider: Option<String>, json: bool) -> Result<()> {
    let providers = api::fetch_providers()?;

    let mut infos: Vec<ModelInfo> = Vec::new();

    if let Some(provider_id) = &provider {
        let p = providers
            .get(provider_id)
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", provider_id))?;

        for (model_id, model) in &p.models {
            infos.push(ModelInfo {
                id: model_id.clone(),
                name: model.name.clone(),
                provider: p.id.clone(),
                context: model.context_str(),
                cost: model.cost_str(),
                capabilities: model.capabilities_str(),
            });
        }
    } else {
        for (provider_id, p) in &providers {
            for (model_id, model) in &p.models {
                infos.push(ModelInfo {
                    id: model_id.clone(),
                    name: model.name.clone(),
                    provider: provider_id.clone(),
                    context: model.context_str(),
                    cost: model.cost_str(),
                    capabilities: model.capabilities_str(),
                });
            }
        }
    }

    infos.sort_by(|a, b| a.provider.cmp(&b.provider).then_with(|| a.id.cmp(&b.id)));

    if json {
        println!("{}", serde_json::to_string_pretty(&infos)?);
    } else {
        if infos.is_empty() {
            bail!("No models found");
        }

        let mut table = Table::new();
        table.load_preset(UTF8_FULL_CONDENSED);
        table.set_header(vec![
            "ID",
            "Name",
            "Provider",
            "Context",
            "Cost (in/out)",
            "Capabilities",
        ]);

        for info in infos {
            table.add_row(vec![
                info.id,
                info.name,
                info.provider,
                info.context,
                info.cost,
                info.capabilities,
            ]);
        }

        println!("{table}");
    }

    Ok(())
}
