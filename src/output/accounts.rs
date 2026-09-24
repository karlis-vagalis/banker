use super::OutputFormat;
use crate::api::openapi::types::AccountResource;
use crate::db::row::AccountRow;
use crate::error::AppError;

pub fn print_api_accounts(items: &[AccountResource], format: OutputFormat) -> Result<(), AppError> {
    match format {
        OutputFormat::Jsonl => {
            for item in items {
                println!("{}", serde_json::to_string(item)?);
            }
        }
        OutputFormat::Lines => {
            for account in items {
                let name = account.name.as_deref().unwrap_or("-");
                let desc = account.details.as_deref().unwrap_or("-");
                let iban = account
                    .account_id
                    .as_ref()
                    .and_then(|id| id.iban.as_deref())
                    .unwrap_or("-");
                let product = account.product.as_deref().unwrap_or("-");
                println!(
                    "{name} | {desc} | {iban} | {} | {product}",
                    account.currency
                );
            }
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.load_preset(comfy_table::presets::UTF8_FULL);
            table.set_header(vec!["Name", "Description", "IBAN", "Currency", "Product"]);
            for account in items {
                let name = account.name.as_deref().unwrap_or("-");
                let desc = account.details.as_deref().unwrap_or("-");
                let iban = account
                    .account_id
                    .as_ref()
                    .and_then(|id| id.iban.as_deref())
                    .unwrap_or("-");
                let product = account.product.as_deref().unwrap_or("-");
                table.add_row(vec![name, desc, iban, &account.currency, product]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

pub fn print_db_accounts(items: &[AccountRow], format: OutputFormat) -> Result<(), AppError> {
    match format {
        OutputFormat::Jsonl => {
            for row in items {
                println!(
                    "{}",
                    serde_json::json!({
                        "id": row.id.to_string(),
                        "updated_at": row.updated_at,
                        "content": &row.content.0,
                    })
                );
            }
        }
        OutputFormat::Lines => {
            for row in items {
                let account = &row.content.0;
                let name = account.name.as_deref().unwrap_or("-");
                let desc = account.details.as_deref().unwrap_or("-");
                let iban = account
                    .account_id
                    .as_ref()
                    .and_then(|id| id.iban.as_deref())
                    .unwrap_or("-");
                let product = account.product.as_deref().unwrap_or("-");
                println!(
                    "{} | {name} | {desc} | {iban} | {} | {product} | {}",
                    row.id,
                    account.currency,
                    row.updated_at.to_rfc3339()
                );
            }
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.load_preset(comfy_table::presets::UTF8_FULL);
            table.set_header(vec![
                "ID",
                "Name",
                "Description",
                "IBAN",
                "Currency",
                "Product",
                "Updated At",
            ]);
            for row in items {
                let account = &row.content.0;
                let name = account.name.as_deref().unwrap_or("-");
                let desc = account.details.as_deref().unwrap_or("-");
                let iban = account
                    .account_id
                    .as_ref()
                    .and_then(|id| id.iban.as_deref())
                    .unwrap_or("-");
                let product = account.product.as_deref().unwrap_or("-");
                table.add_row(vec![
                    &row.id.to_string(),
                    name,
                    desc,
                    iban,
                    &account.currency,
                    product,
                    &row.updated_at.to_rfc3339(),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}
