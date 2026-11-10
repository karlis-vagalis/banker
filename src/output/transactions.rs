use super::OutputFormat;
use crate::api::openapi::types::Transaction;
use crate::db::transactions::TransactionRecord;
use crate::error::AppError;
use crate::models::transaction_query::{FieldSelection, TransactionField};
use serde_json::{Value, json};
use std::io::Write;

fn description(t: &Transaction) -> String {
    t.remittance_information
        .as_ref()
        .map(|v| v.join(". "))
        .unwrap_or_default()
}

fn counterparty(t: &Transaction) -> Option<&str> {
    match t.credit_debit_indicator.to_string().as_str() {
        "CRDT" => t.debtor.as_ref().and_then(|p| p.name.as_deref()),
        "DBIT" => t.creditor.as_ref().and_then(|p| p.name.as_deref()),
        _ => None,
    }
}

fn amount(t: &Transaction) -> String {
    let raw = t.transaction_amount.amount.to_string();
    let magnitude = raw.trim_start_matches(['+', '-']);
    match t.credit_debit_indicator.to_string().as_str() {
        "CRDT" => format!("+{magnitude}"),
        "DBIT" => format!("-{magnitude}"),
        _ => raw,
    }
}

pub fn full_record(row: &TransactionRecord) -> Value {
    json!({
        "id": row.id.to_string(),
        "account_id": row.account_id.to_string(),
        "updated_at": row.updated_at,
        "content": row.content.0,
        "metadata": row.metadata.as_ref().map(|m| &m.0),
    })
}

/// None means absent; Some(Null) means explicitly stored JSON null.
fn field_value(row: &TransactionRecord, field: &TransactionField) -> Option<Value> {
    let t = &row.content.0;
    Some(match field {
        TransactionField::Id => json!(row.id.to_string()),
        TransactionField::AccountId => json!(row.account_id.to_string()),
        TransactionField::Date => json!(t.booking_date?),
        TransactionField::Amount => json!(amount(t)),
        TransactionField::Currency => json!(t.transaction_amount.currency),
        TransactionField::Counterparty => json!(counterparty(t)?),
        TransactionField::Description => json!(description(t)),
        TransactionField::UpdatedAt => json!(row.updated_at),
        TransactionField::Metadata => row.metadata.as_ref()?.0.clone(),
        TransactionField::MetadataValue(path) => {
            let mut value = &row.metadata.as_ref()?.0;
            for segment in path.segments() {
                value = value.as_object()?.get(segment)?;
            }
            value.clone()
        }
    })
}

fn projected_record(row: &TransactionRecord, fields: &FieldSelection) -> Value {
    let mut object = serde_json::Map::new();
    for field in fields.fields() {
        // Omitted keys preserve the distinction between missing and explicit null.
        if let Some(value) = field_value(row, field) {
            object.insert(field.name(), value);
        }
    }
    Value::Object(object)
}

fn cell(value: Option<Value>) -> String {
    match value {
        None => "—".into(),
        Some(Value::String(s)) => s,
        Some(value) => value.to_string(),
    }
}

fn compact_cell(text: &str) -> String {
    let clean: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let mut chars = clean.chars();
    let mut result: String = chars.by_ref().take(40).collect();
    if chars.next().is_some() {
        result.push('…');
    }
    result
}

fn line_cell(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Tables buffer only their explicitly bounded selection; streaming formats retain no rows.
pub struct TransactionRenderer<W: Write> {
    writer: W,
    format: OutputFormat,
    fields: FieldSelection,
    projected: bool,
    table: Option<comfy_table::Table>,
}

impl<W: Write> TransactionRenderer<W> {
    pub fn new(writer: W, format: OutputFormat, fields: Option<FieldSelection>) -> Self {
        let projected = fields.is_some();
        let fields = fields.unwrap_or_default();
        let table = if matches!(format, OutputFormat::Table) {
            let mut table = comfy_table::Table::new();
            table.load_preset(comfy_table::presets::UTF8_FULL);
            table.set_header(fields.fields().iter().map(TransactionField::name));
            Some(table)
        } else {
            None
        };
        Self {
            writer,
            format,
            fields,
            projected,
            table,
        }
    }

    pub fn write_record(&mut self, row: &TransactionRecord) -> Result<(), AppError> {
        match self.format {
            OutputFormat::Jsonl => {
                let value = if self.projected {
                    projected_record(row, &self.fields)
                } else {
                    full_record(row)
                };
                // Serialize before writing so I/O errors remain AppError::Io (not wrapped JSON errors).
                writeln!(self.writer, "{}", serde_json::to_string(&value)?)?;
            }
            OutputFormat::Lines => {
                let cells: Vec<_> = self
                    .fields
                    .fields()
                    .iter()
                    .map(|field| line_cell(&cell(field_value(row, field))))
                    .collect();
                writeln!(self.writer, "{}", cells.join(" | "))?;
            }
            OutputFormat::Table => {
                let cells: Vec<_> = self
                    .fields
                    .fields()
                    .iter()
                    .map(|field| compact_cell(&cell(field_value(row, field))))
                    .collect();
                if let Some(table) = &mut self.table {
                    table.add_row(cells);
                }
            }
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), AppError> {
        if let Some(table) = self.table {
            writeln!(self.writer, "{table}")?;
        }
        self.writer.flush()?;
        Ok(())
    }
}

pub fn write_detail(
    writer: &mut impl Write,
    row: &TransactionRecord,
    format: OutputFormat,
) -> Result<(), AppError> {
    match format {
        OutputFormat::Jsonl => writeln!(writer, "{}", serde_json::to_string(&full_record(row))?)?,
        OutputFormat::Lines => writeln!(writer, "{}", serde_json::to_string(&full_record(row))?)?,
        OutputFormat::Table => writeln!(
            writer,
            "{}",
            serde_json::to_string_pretty(&full_record(row))?
        )?,
    }
    writer.flush()?;
    Ok(())
}

// Live API output intentionally retains its existing contract.
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
