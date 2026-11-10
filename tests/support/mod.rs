//! Deterministic, isolated database fixtures shared by integration tests.
use anyhow::{Context, Result, ensure};
use banker::{
    db::Database,
    models::{AccountId, ContentHash, TransactionId},
};
use chrono::{Days, NaiveDate};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU8,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

const TIMESTAMP: &str = "2026-03-01T12:00:00Z";
const BATCH_SIZE: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TestBank {
    Birch,
    Linden,
    Oak,
    Pine,
}

impl TestBank {
    pub fn key(self) -> &'static str {
        match self {
            Self::Birch => "birch",
            Self::Linden => "linden",
            Self::Oak => "oak",
            Self::Pine => "pine",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Birch => "Fixture Birch",
            Self::Linden => "Fixture Linden",
            Self::Oak => "Fixture Oak",
            Self::Pine => "Fixture Pine",
        }
    }
    pub fn country(self) -> &'static str {
        match self {
            Self::Birch | Self::Pine => "DE",
            Self::Linden => "LV",
            Self::Oak => "NL",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BankSeed {
    bank: TestBank,
    accounts: NonZeroU8,
    transactions: u32,
}

impl BankSeed {
    /// Transaction count is per bank, distributed as evenly as possible across its accounts.
    /// Zero transactions is allowed, so empty accounts and banks can be tested.
    pub fn new(bank: TestBank, accounts: NonZeroU8, transactions: u32) -> Self {
        Self {
            bank,
            accounts,
            transactions,
        }
    }
}

/// Nonempty layout with unique bank identities, proved once at construction.
#[derive(Debug, Clone)]
pub struct SeedSpec(Vec<BankSeed>);

impl SeedSpec {
    pub fn new(banks: impl IntoIterator<Item = BankSeed>) -> Result<Self> {
        let banks: Vec<_> = banks.into_iter().collect();
        ensure!(
            !banks.is_empty(),
            "a seed specification needs at least one bank"
        );
        let mut identities = BTreeSet::new();
        for bank in &banks {
            ensure!(
                identities.insert(bank.bank),
                "duplicate seed bank: {}",
                bank.bank.key()
            );
        }
        Ok(Self(banks))
    }
}

impl Default for SeedSpec {
    fn default() -> Self {
        let accounts = NonZeroU8::new(2).unwrap();
        Self::new([
            BankSeed::new(TestBank::Birch, accounts, 10),
            BankSeed::new(TestBank::Linden, accounts, 100),
            BankSeed::new(TestBank::Oak, accounts, 1_000),
            BankSeed::new(TestBank::Pine, accounts, 10_000),
        ])
        .expect("the default seed layout is nonempty and has unique banks")
    }
}

#[derive(Debug)]
pub struct SeededAccount {
    pub id: AccountId,
    pub currency: &'static str,
    pub transaction_count: u32,
}

#[derive(Debug)]
pub struct SeededBank {
    pub bank: TestBank,
    pub accounts: Vec<SeededAccount>,
}

/// Owns an isolated temporary directory; dropping it removes the DB, WAL and config.
/// No open pool is retained, so subprocesses see committed, checkpointed data.
pub struct SeededDatabase {
    directory: TempDir,
    pub banks: Vec<SeededBank>,
}

impl SeededDatabase {
    pub fn db_path(&self) -> PathBuf {
        self.directory.path().join("data.db")
    }
    pub fn config_path(&self) -> PathBuf {
        self.directory.path().join("config.toml")
    }
    pub fn directory(&self) -> &Path {
        self.directory.path()
    }
    pub fn transaction_count(&self) -> u64 {
        self.banks
            .iter()
            .flat_map(|b| &b.accounts)
            .map(|a| u64::from(a.transaction_count))
            .sum()
    }
    /// A command rooted in this fixture's config, never the user's configuration.
    pub fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_banker"));
        command.arg("--config").arg(self.config_path());
        command
    }
}

struct TransactionSeed {
    id: TransactionId,
    reference: Option<String>,
    content: String,
    metadata: Option<Value>,
}

fn transaction_seed(
    bank: TestBank,
    account: &AccountId,
    index: u32,
    raw_id: i64,
    currency: &str,
) -> Result<TransactionSeed> {
    let credit = index.is_multiple_of(5);
    let pending = index.is_multiple_of(17);
    let reference = (!index.is_multiple_of(7))
        .then(|| format!("fixture-{}-{}-{index}", bank.key(), account.raw()));
    let cents = 100 + (u64::from(index) * 97 + account.raw() as u64 * 211) % 250_000;
    let amount = format!("{}.{:02}", cents / 100, cents % 100);
    let date = NaiveDate::from_ymd_opt(2025, 1, 1)
        .unwrap()
        .checked_add_days(Days::new(u64::from(index % 400)))
        .unwrap();
    let (category, merchant, description) = match index % 4 {
        0 => (
            "groceries",
            "Rīgas veikals",
            "Groceries 100% | weekly\ncard payment",
        ),
        1 => ("transport", "City Rail", "Monthly_ticket\\receipt"),
        2 => ("utilities", "Example Energy", "Electricity — household"),
        _ => ("shopping", "Bücher & Café", "Books and coffee"),
    };
    let content = json!({
        "entry_reference": reference,
        "booking_date": (!pending).then_some(date),
        "value_date": date,
        "transaction_date": date,
        "transaction_amount": {"amount": amount, "currency": currency},
        "credit_debit_indicator": if credit { "CRDT" } else { "DBIT" },
        "status": if pending { "PDNG" } else { "BOOK" },
        "creditor": {"name": merchant},
        "debtor": {"name": "Fixture Employer"},
        "remittance_information": [description, format!("{} account {} payment {index}", bank.key(), account.raw())],
    }).to_string();
    let metadata = match index % 4 {
        0 => Some(
            json!({"category":category, "tags":["fixture", "reviewed"], "review":null,
            "nested":{"weight":0.8}, "note":"Pārbaudīts | checked\nsecond line"}),
        ),
        1 => None,
        2 => Some(json!({})),
        _ => Some(
            json!({"category":category, "tags":[], "review":{"approved":false}, "nested":{"weight":0.25}}),
        ),
    };
    Ok(TransactionSeed {
        id: TransactionId::try_from(raw_id).map_err(anyhow::Error::msg)?,
        reference,
        content,
        metadata,
    })
}

/// Create a migrated, deterministic database and matching bank config, ready for E2E commands.
/// Data uses fixed dates, exact decimal amounts, four currencies and mixed annotations.
/// Inserts are batched inside one transaction, so larger custom layouts stay practical.
pub async fn seeded_database(spec: SeedSpec) -> Result<SeededDatabase> {
    let directory = tempfile::Builder::new()
        .prefix("banker-seeded-")
        .tempdir()?;
    let mut fixture = SeededDatabase {
        directory,
        banks: Vec::new(),
    };
    let database = Database::open(&fixture.db_path()).await?;
    let result = async {
        let mut tx = database.pool().begin().await?;
        let mut banks = Vec::new();
        let mut bank_config = BTreeMap::new();
        let mut next_account = 1i64;
        let mut next_transaction = 1i64;
        for seed in spec.0 {
            bank_config.insert(seed.bank.key(), json!({"name":seed.bank.name(), "country":seed.bank.country()}));
            let mut accounts = Vec::new();
            for account_index in 0..u32::from(seed.accounts.get()) {
                let id = AccountId::try_from(next_account).map_err(anyhow::Error::msg)?;
                next_account += 1;
                let currency = ["EUR", "USD", "GBP", "CHF"][((id.raw() - 1) % 4) as usize];
                let identification = format!("fixture-{}-{account_index}", seed.bank.key());
                let content = json!({"name":format!("{} account {}", seed.bank.name(), account_index + 1),
                    "cash_account_type":"CACC", "currency":currency,
                    "identification_hash":identification, "identification_hashes":[identification]}).to_string();
                sqlx::query("INSERT INTO accounts (id, aspsp_name, aspsp_country, identification_hash, content, content_hash, inserted_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                    .bind(id.clone()).bind(seed.bank.name()).bind(seed.bank.country()).bind(&identification)
                    .bind(&content).bind(ContentHash::new(content.as_bytes())).bind(TIMESTAMP).bind(TIMESTAMP)
                    .execute(&mut *tx).await?;
                let transaction_count = seed.transactions / u32::from(seed.accounts.get())
                    + u32::from(account_index < seed.transactions % u32::from(seed.accounts.get()));
                for start in (0..transaction_count).step_by(BATCH_SIZE as usize) {
                    let end = start.saturating_add(BATCH_SIZE).min(transaction_count);
                    let mut rows = Vec::with_capacity((end - start) as usize);
                    for index in start..end {
                        rows.push(transaction_seed(seed.bank, &id, index, next_transaction, currency)?);
                        next_transaction += 1;
                    }
                    let mut insert = sqlx::QueryBuilder::new("INSERT INTO transactions (id, account_id, entry_reference, content, content_hash, inserted_at, updated_at) ");
                    insert.push_values(&rows, |mut values, row| {
                        values.push_bind(row.id.clone()).push_bind(id.clone()).push_bind(row.reference.clone())
                            .push_bind(row.content.clone()).push_bind(ContentHash::new(row.content.as_bytes()))
                            .push_bind(TIMESTAMP).push_bind(TIMESTAMP);
                    });
                    insert.build().execute(&mut *tx).await?;
                    let annotated: Vec<_> = rows.iter().filter_map(|row| row.metadata.as_ref().map(|metadata| (&row.id, metadata))).collect();
                    if !annotated.is_empty() {
                        let mut insert = sqlx::QueryBuilder::new("INSERT INTO transaction_metadata (transaction_id, user_metadata, inserted_at, updated_at) ");
                        insert.push_values(annotated, |mut values, (id, metadata)| {
                            values.push_bind(id.clone()).push_bind(metadata.to_string()).push_bind(TIMESTAMP).push_bind(TIMESTAMP);
                        });
                        insert.build().execute(&mut *tx).await?;
                    }
                }
                accounts.push(SeededAccount { id, currency, transaction_count });
            }
            banks.push(SeededBank { bank: seed.bank, accounts });
        }
        tx.commit().await?;
        fixture.banks = banks;
        let config = json!({"db_path":fixture.db_path(), "api_base_url":"http://127.0.0.1:9",
            "application":{"fixture":{"id":"fixture-only", "key_file":"unused-fixture-key.pem"}}, "bank":bank_config});
        std::fs::write(fixture.config_path(), toml::to_string_pretty(&config)?)
            .context("write isolated fixture config")?;
        database.checkpoint().await?;
        Ok::<_, anyhow::Error>(())
    }.await;
    database.close().await;
    result?;
    Ok(fixture)
}
