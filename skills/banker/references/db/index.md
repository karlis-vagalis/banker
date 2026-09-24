# Banker Database Reference

The `banker` CLI stores all synced financial data in a local SQLite database.

## Location

Default path: `~/.local/share/banker/data.db`

Override with `db_path` in `~/.config/banker/config.toml`:
```toml
db_path = "/custom/path/data.db"
```

Manage data via `banker account list`, `banker balance list`, `banker transaction list`.

## Schema (7 tables)

### `accounts`

| Column | Type | Notes |
|--------|------|-------|
| `id` | INTEGER | Primary key, auto-increment |
| `aspsp_name` | TEXT | Bank/ASPSP name, e.g. "Comdirect" |
| `aspsp_country` | TEXT | ISO country code, e.g. "DE" |
| `identification_hash` | TEXT | **Unique** — enables dedup across sessions |
| `content` | TEXT | JSON — full `AccountResource` from the API |
| `content_hash` | INTEGER | xxh3-64 hash of JSON content |
| `inserted_at` | TEXT | ISO 8601 UTC timestamp |
| `updated_at` | TEXT | ISO 8601 UTC timestamp |

Key JSON paths inside `content`:

| Path | Type | Meaning |
|------|------|---------|
| `$.name` | `TEXT` | Human-readable account name/description |
| `$.account_id.iban` | `TEXT` | IBAN |
| `$.uid` | `TEXT` (UUID) | API account UID |
| `$.currency` | `TEXT` | ISO currency code |
| `$.product` | `TEXT` | Product name (e.g. "Girokonto") |
| `$.cash_account_type` | `TEXT` | Account category |

### `balances`

| Column | Type | Notes |
|--------|------|-------|
| `id` | INTEGER | Primary key |
| `account_id` | INTEGER | FK → `accounts.id` |
| `balance_type` | TEXT | e.g. "closingBooked", "expected", "interimAvailable" |
| `content` | TEXT | JSON — single balance object from the API |
| `inserted_at` | TEXT | ISO 8601 UTC timestamp |

Key JSON paths inside `content`:

| Path | Type | Meaning |
|------|------|---------|
| `$.balance_amount.amount` | `TEXT` | Numeric amount (string) |
| `$.balance_amount.currency` | `TEXT` | ISO currency code |
| `$.reference_date` | `TEXT` | Date the balance applies to |

### `transactions`

| Column | Type | Notes |
|--------|------|-------|
| `id` | INTEGER | Primary key |
| `account_id` | INTEGER | FK → `accounts.id` |
| `entry_reference` | TEXT | Dedup key, nullable (some entries have no ref) |
| `content` | TEXT | JSON — full `Transaction` from the API |
| `content_hash` | INTEGER | xxh3-64 hash of JSON content |
| `inserted_at` | TEXT | ISO 8601 UTC timestamp |
| `updated_at` | TEXT | ISO 8601 UTC timestamp |

Key JSON paths inside `content`:

| Path | Type | Meaning |
|------|------|---------|
| `$.transaction_amount.amount` | `TEXT` | Numeric amount (string) |
| `$.transaction_amount.currency` | `TEXT` | ISO currency code |
| `$.booking_date` | `TEXT` | Date the transaction was booked |
| `$.value_date` | `TEXT` | Date the transaction takes effect |
| `$.remittance_information` | `TEXT[]` | Array of description lines |
| `$.debtor_name` | `TEXT` | Payer name (debit transactions) |
| `$.creditor_name` | `TEXT` | Payee name (credit transactions) |
| `$.debtor_account.iban` | `TEXT` | Payer IBAN |
| `$.creditor_account.iban` | `TEXT` | Payee IBAN |
| `$.proprietary_bank_transaction_code` | `TEXT` | Bank-specific transaction code |
| `$.entry_reference` | `TEXT` | Bank's reference for the entry |

### `metadata_schemas`

| Column | Type | Notes |
|--------|------|-------|
| `target_type` | TEXT | PK — one of `'accounts'`, `'balances'`, `'transactions'` |
| `json_schema` | TEXT | JSON Schema used to validate user metadata |
| `inserted_at` | TEXT | ISO 8601 UTC timestamp |
| `updated_at` | TEXT | ISO 8601 UTC timestamp |

### `account_metadata`

| Column | Type | Notes |
|--------|------|-------|
| `account_id` | INTEGER | PK, FK → `accounts.id` ON DELETE CASCADE |
| `user_metadata` | TEXT | JSON — user-defined metadata |
| `inserted_at` | TEXT | ISO 8601 UTC timestamp |
| `updated_at` | TEXT | ISO 8601 UTC timestamp |

### `balance_metadata`

| Column | Type | Notes |
|--------|------|-------|
| `balance_id` | INTEGER | PK, FK → `balances.id` ON DELETE CASCADE |
| `user_metadata` | TEXT | JSON — user-defined metadata |
| `inserted_at` | TEXT | ISO 8601 UTC timestamp |
| `updated_at` | TEXT | ISO 8601 UTC timestamp |

### `transaction_metadata`

| Column | Type | Notes |
|--------|------|-------|
| `transaction_id` | INTEGER | PK, FK → `transactions.id` ON DELETE CASCADE |
| `user_metadata` | TEXT | JSON — user-defined metadata |
| `inserted_at` | TEXT | ISO 8601 UTC timestamp |
| `updated_at` | TEXT | ISO 8601 UTC timestamp |
