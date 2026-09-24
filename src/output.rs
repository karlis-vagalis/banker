use crate::api::openapi::types::{AccountResource, BalanceResource, Transaction};
use crate::error::AppError;
use chrono::{DateTime, Utc};

pub enum OutputFormat {
    Table,
    Jsonl,
    Lines,
}

pub fn print_api_accounts(items: &[AccountResource], format: OutputFormat) -> Result<(), AppError> {
    match format {
        OutputFormat::Jsonl => {
            for item in items {
                println!("{}", serde_json::to_string(item)?);
            }
        }
        OutputFormat::Lines => {
            for a in items {
                let name = a.name.as_deref().unwrap_or("-");
                let desc = a.details.as_deref().unwrap_or("-");
                let iban = a
                    .account_id
                    .as_ref()
                    .and_then(|id| id.iban.as_deref())
                    .unwrap_or("-");
                let product = a.product.as_deref().unwrap_or("-");
                println!("{name} | {desc} | {iban} | {} | {product}", a.currency);
            }
        }
        OutputFormat::Table => {
            let mut table = comfy_table::Table::new();
            table.load_preset(comfy_table::presets::UTF8_FULL);
            table.set_header(vec!["Name", "Description", "IBAN", "Currency", "Product"]);
            for a in items {
                let name = a.name.as_deref().unwrap_or("-");
                let desc = a.details.as_deref().unwrap_or("-");
                let iban = a
                    .account_id
                    .as_ref()
                    .and_then(|id| id.iban.as_deref())
                    .unwrap_or("-");
                let product = a.product.as_deref().unwrap_or("-");
                table.add_row(vec![name, desc, iban, &a.currency, product]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

pub fn print_db_accounts(
    items: &[&AccountResource],
    ids: &[String],
    updated_ats: &[DateTime<Utc>],
    format: OutputFormat,
) -> Result<(), AppError> {
    match format {
        OutputFormat::Jsonl => {
            for ((a, _id), _ts) in items.iter().zip(ids.iter()).zip(updated_ats.iter()) {
                println!("{}", serde_json::to_string(a)?);
            }
        }
        OutputFormat::Lines => {
            for ((a, id), ts) in items.iter().zip(ids.iter()).zip(updated_ats.iter()) {
                let name = a.name.as_deref().unwrap_or("-");
                let desc = a.details.as_deref().unwrap_or("-");
                let iban = a
                    .account_id
                    .as_ref()
                    .and_then(|id| id.iban.as_deref())
                    .unwrap_or("-");
                let product = a.product.as_deref().unwrap_or("-");
                println!("{id} | {name} | {desc} | {iban} | {} | {product} | {}", a.currency, ts.to_rfc3339());
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
            for ((a, id), ts) in items.iter().zip(ids.iter()).zip(updated_ats.iter()) {
                let name = a.name.as_deref().unwrap_or("-");
                let desc = a.details.as_deref().unwrap_or("-");
                let iban = a
                    .account_id
                    .as_ref()
                    .and_then(|id| id.iban.as_deref())
                    .unwrap_or("-");
                let product = a.product.as_deref().unwrap_or("-");
                table.add_row(vec![
                    id.as_str(),
                    name,
                    desc,
                    iban,
                    &a.currency,
                    product,
                    &ts.to_rfc3339(),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

pub fn print_api_balances(items: &[BalanceResource], format: OutputFormat) -> Result<(), AppError> {
    match format {
        OutputFormat::Jsonl => {
            for item in items {
                println!("{}", serde_json::to_string(item)?);
            }
        }
        OutputFormat::Lines => {
            for b in items {
                println!("{} | {} | {} | {}",
                    b.balance_type.to_string(),
                    b.balance_amount.amount.to_string(),
                    b.balance_amount.currency,
                    b.reference_date.map(|d| d.to_string()).unwrap_or_default(),
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
                    &b.balance_amount.amount.to_string(),
                    &b.balance_amount.currency,
                    &b.reference_date.map(|d| d.to_string()).unwrap_or_default(),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

pub fn print_db_balances(
    items: &[&BalanceResource],
    ids: &[String],
    account_ids: &[String],
    inserted_ats: &[DateTime<Utc>],
    format: OutputFormat,
) -> Result<(), AppError> {
    match format {
        OutputFormat::Jsonl => {
            for b in items {
                println!("{}", serde_json::to_string(b)?);
            }
        }
        OutputFormat::Lines => {
            for (b, id, aid, ts) in items.iter().zip(ids.iter()).zip(account_ids.iter()).zip(inserted_ats.iter()).map(|(((b, id), aid), ts)| (b, id, aid, ts)) {
                println!("{id} | {aid} | {} | {} | {} | {} | {}",
                    b.balance_type.to_string(),
                    b.balance_amount.amount.to_string(),
                    b.balance_amount.currency,
                    b.reference_date.map(|d| d.to_string()).unwrap_or_default(),
                    ts.to_rfc3339(),
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
            for (((b, id), aid), ts) in items
                .iter()
                .zip(ids.iter())
                .zip(account_ids.iter())
                .zip(inserted_ats.iter())
            {
                table.add_row(vec![
                    id.as_str(),
                    aid.as_str(),
                    &b.balance_type.to_string(),
                    &b.balance_amount.amount.to_string(),
                    &b.balance_amount.currency,
                    &b.reference_date.map(|d| d.to_string()).unwrap_or_default(),
                    &ts.to_rfc3339(),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

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
        .unwrap_or_else(|| "".to_string())
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
                println!("{} | {} | {} | {} | {}",
                    t.booking_date.map(|d| d.to_string()).unwrap_or_default(),
                    t.transaction_amount.amount.to_string(),
                    t.transaction_amount.currency,
                    remittance_first(t),
                    debtor_creditor(t),
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
    items: &[&Transaction],
    ids: &[String],
    account_ids: &[String],
    updated_ats: &[DateTime<Utc>],
    format: OutputFormat,
) -> Result<(), AppError> {
    match format {
        OutputFormat::Jsonl => {
            for t in items {
                println!("{}", serde_json::to_string(t)?);
            }
        }
        OutputFormat::Lines => {
            for (((t, id), aid), ts) in items.iter().zip(ids.iter()).zip(account_ids.iter()).zip(updated_ats.iter()) {
                let sign = match t.credit_debit_indicator.to_string().as_str() {
                    "CRDT" => "+",
                    "DBIT" => "-",
                    _ => "",
                };
                println!("{id} | {aid} | {} | {}{} | {} | {} | {} | {}",
                    t.booking_date.map(|d| d.to_string()).unwrap_or_default(),
                    sign,
                    t.transaction_amount.amount.to_string(),
                    t.transaction_amount.currency,
                    remittance_first(t),
                    debtor_creditor(t),
                    ts.to_rfc3339(),
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
            for (((t, id), aid), ts) in items
                .iter()
                .zip(ids.iter())
                .zip(account_ids.iter())
                .zip(updated_ats.iter())
            {
                let sign = match t.credit_debit_indicator.to_string().as_str() {
                    "CRDT" => "+",
                    "DBIT" => "-",
                    _ => "",
                };
                let amount = format!("{}{}", sign, t.transaction_amount.amount.to_string());
                table.add_row(vec![
                    id.as_str(),
                    aid.as_str(),
                    &t.booking_date.map(|d| d.to_string()).unwrap_or_default(),
                    &amount,
                    &t.transaction_amount.currency,
                    &wrap_text(&remittance_first(t), 60),
                    debtor_creditor(t),
                    &ts.to_rfc3339(),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}
