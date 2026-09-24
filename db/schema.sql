CREATE TABLE accounts (
    id INTEGER PRIMARY KEY,
    aspsp_name TEXT NOT NULL,
    aspsp_country TEXT NOT NULL,
    identification_hash TEXT UNIQUE,
    content TEXT NOT NULL CHECK(json_valid(content)), -- serialized JSON
    content_hash INTEGER NOT NULL, -- hash of serialized JSON (content)
    inserted_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE transactions (
    id INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL,
    entry_reference TEXT,
    content TEXT NOT NULL CHECK(json_valid(content)), -- serialized JSON
    content_hash INTEGER NOT NULL, -- hash of serialized JSON (content)
    inserted_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL,
    FOREIGN KEY(account_id) REFERENCES accounts(id)
) STRICT;

CREATE INDEX idx_transactions_account_id ON transactions(account_id);

CREATE UNIQUE INDEX idx_transactions_account_entry_ref 
ON transactions(account_id, entry_reference) 
WHERE entry_reference IS NOT NULL;

CREATE TABLE balances (
    id INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL,
    balance_type TEXT NOT NULL,
    content TEXT NOT NULL CHECK(json_valid(content)), -- serialized JSON
    inserted_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY(account_id) REFERENCES accounts(id)
) STRICT;

CREATE INDEX idx_balances_lookup ON balances(account_id, balance_type, inserted_at DESC);

CREATE TABLE metadata_schemas (
    target_type TEXT PRIMARY KEY CHECK(target_type IN ('transactions', 'accounts', 'balances')),
    json_schema TEXT NOT NULL CHECK(json_valid(json_schema)),
    inserted_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE transaction_metadata (
    transaction_id INTEGER PRIMARY KEY,
    user_metadata TEXT NOT NULL CHECK(json_valid(user_metadata)),
    inserted_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL,
    FOREIGN KEY(transaction_id) REFERENCES transactions(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE account_metadata (
    account_id INTEGER PRIMARY KEY,
    user_metadata TEXT NOT NULL CHECK(json_valid(user_metadata)),
    inserted_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL,
    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE balance_metadata (
    balance_id INTEGER PRIMARY KEY,
    user_metadata TEXT NOT NULL CHECK(json_valid(user_metadata)),
    inserted_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL,
    FOREIGN KEY(balance_id) REFERENCES balances(id) ON DELETE CASCADE
) STRICT;