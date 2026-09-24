use super::OutputFormat;
use crate::api::openapi::types::BalanceResource;
use crate::db::row::BalanceRow;
use crate::error::AppError;

pub fn print_api_balances(items: &[BalanceResource], format: OutputFormat) -> Result<(), AppError> {
    match format {
        OutputFormat::Jsonl => {
            for item in items {
                println!("{}", serde_json::to_string(item)?);
            }
        }
        OutputFormat::Lines => {
            for b in items {
                println!(
                    "{} | {} | {} | {}",
                    b.balance_type,
                    *b.balance_amount.amount,
                    b.balance_amount.currency,
                    b.reference_date.map(|d| d.to_string()).unwrap_or_default()
                );
            }
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.load_preset(comfy_table::presets::UTF8_FULL);
            table.set_header(vec!["Type", "Amount", "Currency", "Reference Date"]);
            for b in items {
                table.add_row(vec![
                    &b.balance_type.to_string(),
                    &*b.balance_amount.amount,
                    &b.balance_amount.currency,
                    &b.reference_date.map(|d| d.to_string()).unwrap_or_default(),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

pub fn print_db_balances(items: &[BalanceRow], format: OutputFormat) -> Result<(), AppError> {
    match format {
        OutputFormat::Jsonl => {
            for row in items {
                println!(
                    "{}",
                    serde_json::json!({
                        "id": row.id.to_string(),
                        "account_id": row.account_id.to_string(),
                        "inserted_at": row.inserted_at,
                        "content": &row.content.0,
                    })
                );
            }
        }
        OutputFormat::Lines => {
            for row in items {
                let b = &row.content.0;
                println!(
                    "{} | {} | {} | {} | {} | {} | {}",
                    row.id,
                    row.account_id,
                    b.balance_type,
                    *b.balance_amount.amount,
                    b.balance_amount.currency,
                    b.reference_date.map(|d| d.to_string()).unwrap_or_default(),
                    row.inserted_at.to_rfc3339()
                );
            }
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.load_preset(comfy_table::presets::UTF8_FULL);
            table.set_header(vec![
                "ID",
                "Account ID",
                "Type",
                "Amount",
                "Currency",
                "Reference Date",
                "Inserted At",
            ]);
            for row in items {
                let b = &row.content.0;
                table.add_row(vec![
                    &row.id.to_string(),
                    &row.account_id.to_string(),
                    &b.balance_type.to_string(),
                    &*b.balance_amount.amount,
                    &b.balance_amount.currency,
                    &b.reference_date.map(|d| d.to_string()).unwrap_or_default(),
                    &row.inserted_at.to_rfc3339(),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}
