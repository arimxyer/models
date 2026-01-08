use anyhow::{Context, Result};

use crate::data::ProvidersMap;

const API_URL: &str = "https://models.dev/api.json";

pub fn fetch_providers() -> Result<ProvidersMap> {
    let response = reqwest::blocking::get(API_URL)
        .context("Failed to fetch data from models.dev API")?;

    let providers: ProvidersMap = response
        .json()
        .context("Failed to parse API response")?;

    Ok(providers)
}
