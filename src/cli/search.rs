use anyhow::Result;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Table};
use serde::Serialize;

use crate::api;

#[derive(Serialize)]
struct SearchResult {
    id: String,
    name: String,
    provider: String,
    context: String,
    cost: String,
}

pub fn search(query: &str, json: bool) -> Result<()> {
    let providers = api::fetch_providers()?;
    let query_lower = query.to_lowercase();

    let mut results: Vec<SearchResult> = Vec::new();

    for (provider_id, provider) in &providers {
        let provider_matches = provider_id.to_lowercase().contains(&query_lower)
            || provider.name.to_lowercase().contains(&query_lower);

        for (model_id, model) in &provider.models {
            let model_matches = model_id.to_lowercase().contains(&query_lower)
                || model.name.to_lowercase().contains(&query_lower);

            if model_matches || provider_matches {
                results.push(SearchResult {
                    id: model_id.clone(),
                    name: model.name.clone(),
                    provider: provider_id.clone(),
                    context: model.context_str(),
                    cost: model.cost_str(),
                });
            }
        }
    }

    results.sort_by(|a, b| {
        a.provider
            .cmp(&b.provider)
            .then_with(|| a.id.cmp(&b.id))
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        if results.is_empty() {
            println!("No models found matching '{}'", query);
            return Ok(());
        }

        println!("Found {} models matching '{}':\n", results.len(), query);

        let mut table = Table::new();
        table.load_preset(UTF8_FULL_CONDENSED);
        table.set_header(vec!["ID", "Name", "Provider", "Context", "Cost (in/out)"]);

        for result in results {
            table.add_row(vec![
                result.id,
                result.name,
                result.provider,
                result.context,
                result.cost,
            ]);
        }

        println!("{table}");
    }

    Ok(())
}
