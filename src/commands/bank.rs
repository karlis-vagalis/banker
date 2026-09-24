use crate::api::EnableBankingClient;
use crate::config::{ApplicationConfig, Config};
use crate::error::AppError;

pub async fn run(
    config: &Config,
    app_cfg: &ApplicationConfig,
    country: Option<crate::api::openapi::types::Country>,
    search: Option<String>,
) -> Result<(), AppError> {
    let client = EnableBankingClient::new(config, app_cfg).await?;
    let aspsps = client.list_aspsps(country.as_ref()).await?;

    let needle = search.map(|s| s.to_lowercase());
    let filtered: Vec<_> = aspsps
        .into_iter()
        .filter(|a| {
            needle
                .as_ref()
                .map(|s| a.name.to_lowercase().contains(s))
                .unwrap_or(true)
        })
        .collect();

    if filtered.is_empty() {
        println!("No banks found.");
        return Ok(());
    }

    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.set_header(vec!["Name", "Country"]);
    for a in &filtered {
        table.add_row(vec![a.name.clone(), a.country.clone()]);
    }
    println!("{table}");
    Ok(())
}
