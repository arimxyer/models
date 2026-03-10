use anyhow::Result;

pub fn search(query: &str, json: bool) -> Result<()> {
    super::models::search(query, json)
}
