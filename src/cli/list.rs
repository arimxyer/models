use anyhow::Result;

pub fn providers(json: bool) -> Result<()> {
    super::models::providers(json)
}

pub fn models(provider: Option<String>, json: bool) -> Result<()> {
    super::models::list(provider.as_deref(), json)
}
