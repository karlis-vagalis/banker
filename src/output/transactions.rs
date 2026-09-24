use super::OutputFormat;
use crate::api::openapi::types::Transaction;
use crate::db::row::TransactionRow;
use crate::error::AppError;

fn wrap_text(text: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut line_len = 0;
    for word in text.split(' ') {
        if line_len > 0 && line_len + word.len() + 1 > max_width {
            result.push('\n');
            line_len = 0;
        }
        if line_len > 0 {
            result.push(' ');
            line_len += 1;
        }
        result.push_str(word);
        line_len += word.len();
    }
    result
}

fn remittance_first(t: &Transaction) -> String {
    t.remittance_information
        .as_ref()
        .map(|v| v.join(". "))
        .unwrap_or_default()
}

fn debtor_creditor(t: &Transaction) -> &str {
    match t.credit_debit_indicator.to_string().as_str() {
        "CRDT" => t
            .debtor
            .as_ref()
            .and_then(|p| p.name.as_deref())
            .unwrap_or("-"),
        "DBIT" => t
            .creditor
            .as_ref()
            .and_then(|p| p.name.as_deref())
            .unwrap_or("-"),
        _ => "-",
    }
}

pub fn print_api_transactions(items: &[Transaction], format: OutputFormat) -> Result<(), AppError> {
    match format {
        OutputFormat::Jsonl => {
            for item in items {
                println!("{}", serde_json::to_string(item)?);
            }
        }
        OutputFormat::Lines => {
            for t in items {
                println!(
                    "{} | {} | {} | {} | {}",
                    t.booking_date.map(|d| d.to_string()).unwrap_or_default(),
                    *t.transaction_amount.amount,
                    t.transaction_amount.currency,
                    remittance_first(t),
                    debtor_creditor(t)
                );
            }
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.load_preset(comfy_table::presets::UTF8_FULL);
            table.set_header(vec![
                "Booking Date",
                "Amount",
                "Currency",
                "Description",
                "From/To",
            ]);
            for t in items {
                table.add_row(vec![
                    &t.booking_date.map(|d| d.to_string()).unwrap_or_default(),
                    &t.transaction_amount.amount.to_string(),
                    &t.transaction_amount.currency,
                    &wrap_text(&remittance_first(t), 60),
                    debtor_creditor(t),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

pub fn print_db_transactions(
    items: &[TransactionRow],
    format: OutputFormat,
) -> Result<(), AppError> {
    match format {
        OutputFormat::Jsonl => {
            for row in items {
                println!(
                    "{}",
                    serde_json::json!({
                        "id": row.id.to_string(),
                        "account_id": row.account_id.to_string(),
                        "updated_at": row.updated_at,
                        "content": &row.content.0,
                    })
                );
            }
        }
        OutputFormat::Lines => {
            for row in items {
                let t = &row.content.0;
                let sign = match t.credit_debit_indicator.to_string().as_str() {
                    "CRDT" => "+",
                    "DBIT" => "-",
                    _ => "",
                };
                println!(
                    "{} | {} | {} | {}{} | {} | {} | {} | {}",
                    row.id,
                    row.account_id,
                    t.booking_date.map(|d| d.to_string()).unwrap_or_default(),
                    sign,
                    *t.transaction_amount.amount,
                    t.transaction_amount.currency,
                    remittance_first(t),
                    debtor_creditor(t),
                    row.updated_at.to_rfc3339()
                );
            }
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.load_preset(comfy_table::presets::UTF8_FULL);
            table.set_header(vec![
                "ID",
                "Account ID",
                "Booking Date",
                "Amount",
                "Currency",
                "Description",
                "From/To",
                "Updated At",
            ]);
            for row in items {
                let t = &row.content.0;
                let sign = match t.credit_debit_indicator.to_string().as_str() {
                    "CRDT" => "+",
                    "DBIT" => "-",
                    _ => "",
                };
                let amount = format!("{}{}", sign, *t.transaction_amount.amount);
                table.add_row(vec![
                    &row.id.to_string(),
                    &row.account_id.to_string(),
                    &t.booking_date.map(|d| d.to_string()).unwrap_or_default(),
                    &amount,
                    &t.transaction_amount.currency,
                    &wrap_text(&remittance_first(t), 60),
                    debtor_creditor(t),
                    &row.updated_at.to_rfc3339(),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}
