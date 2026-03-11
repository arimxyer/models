use anyhow::Result;

pub fn model(model_id: &str, json: bool) -> Result<()> {
    super::models::show(model_id, json)
}
