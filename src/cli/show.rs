use anyhow::Result;
use serde::Serialize;

use crate::api;
use crate::data::Model;

#[derive(Serialize)]
struct ModelDetail {
    id: String,
    name: String,
    provider_id: String,
    provider_name: String,
    family: Option<String>,
    context: String,
    output: String,
    input_cost: Option<f64>,
    output_cost: Option<f64>,
    cache_read_cost: Option<f64>,
    cache_write_cost: Option<f64>,
    reasoning: bool,
    tool_call: bool,
    attachment: bool,
    modalities: String,
    release_date: Option<String>,
    last_updated: Option<String>,
    knowledge_cutoff: Option<String>,
    open_weights: bool,
    status: Option<String>,
}

pub fn model(model_id: &str, json: bool) -> Result<()> {
    let providers = api::fetch_providers()?;

    // Search for the model across all providers
    let mut found: Option<(String, String, &Model)> = None;

    for (provider_id, provider) in &providers {
        if let Some(model) = provider.models.get(model_id) {
            found = Some((provider_id.clone(), provider.name.clone(), model));
            break;
        }
    }

    let (provider_id, provider_name, model) = found
        .ok_or_else(|| anyhow::anyhow!("Model '{}' not found", model_id))?;

    let detail = ModelDetail {
        id: model.id.clone(),
        name: model.name.clone(),
        provider_id,
        provider_name,
        family: model.family.clone(),
        context: model.context_str(),
        output: model.output_str(),
        input_cost: model.cost.as_ref().and_then(|c| c.input),
        output_cost: model.cost.as_ref().and_then(|c| c.output),
        cache_read_cost: model.cost.as_ref().and_then(|c| c.cache_read),
        cache_write_cost: model.cost.as_ref().and_then(|c| c.cache_write),
        reasoning: model.reasoning,
        tool_call: model.tool_call,
        attachment: model.attachment,
        modalities: model.modalities_str(),
        release_date: model.release_date.clone(),
        last_updated: model.last_updated.clone(),
        knowledge_cutoff: model.knowledge.clone(),
        open_weights: model.open_weights,
        status: model.status.clone(),
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&detail)?);
    } else {
        print_detail(&detail);
    }

    Ok(())
}

fn print_detail(d: &ModelDetail) {
    println!("{}", d.name);
    println!("{}", "=".repeat(d.name.len()));
    println!();
    println!("ID:          {}", d.id);
    println!("Provider:    {} ({})", d.provider_name, d.provider_id);
    if let Some(family) = &d.family {
        println!("Family:      {}", family);
    }
    println!();

    println!("Limits");
    println!("------");
    println!("Context:     {} tokens", d.context);
    println!("Max Output:  {} tokens", d.output);
    println!();

    println!("Pricing (per million tokens)");
    println!("----------------------------");
    if let Some(input) = d.input_cost {
        println!("Input:       ${:.2}", input);
    }
    if let Some(output) = d.output_cost {
        println!("Output:      ${:.2}", output);
    }
    if let Some(cache_read) = d.cache_read_cost {
        println!("Cache Read:  ${:.2}", cache_read);
    }
    if let Some(cache_write) = d.cache_write_cost {
        println!("Cache Write: ${:.2}", cache_write);
    }
    println!();

    println!("Capabilities");
    println!("------------");
    println!("Reasoning:   {}", if d.reasoning { "Yes" } else { "No" });
    println!("Tool Use:    {}", if d.tool_call { "Yes" } else { "No" });
    println!("Attachments: {}", if d.attachment { "Yes" } else { "No" });
    println!("Modalities:  {}", d.modalities);
    println!();

    println!("Metadata");
    println!("--------");
    if let Some(date) = &d.release_date {
        println!("Released:    {}", date);
    }
    if let Some(date) = &d.last_updated {
        println!("Updated:     {}", date);
    }
    if let Some(date) = &d.knowledge_cutoff {
        println!("Knowledge:   {}", date);
    }
    println!("Open Weights: {}", if d.open_weights { "Yes" } else { "No" });
    if let Some(status) = &d.status {
        println!("Status:      {}", status);
    }
}
