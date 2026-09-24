-- Create "accounts" table
CREATE TABLE `accounts` (
  `id` integer NULL,
  `aspsp_name` text NOT NULL,
  `aspsp_country` text NOT NULL,
  `identification_hash` text NULL,
  `content` text NOT NULL,
  `content_hash` integer NOT NULL,
  `inserted_at` text NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
  `updated_at` text NOT NULL,
  PRIMARY KEY (`id`),
  CHECK (json_valid(content))
) STRICT;
-- Create index "accounts_identification_hash" to table: "accounts"
CREATE UNIQUE INDEX `accounts_identification_hash` ON `accounts` (`identification_hash`);
-- Create "transactions" table
CREATE TABLE `transactions` (
  `id` integer NULL,
  `account_id` integer NOT NULL,
  `entry_reference` text NULL,
  `content` text NOT NULL,
  `content_hash` integer NOT NULL,
  `inserted_at` text NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
  `updated_at` text NOT NULL,
  PRIMARY KEY (`id`),
  CONSTRAINT `0` FOREIGN KEY (`account_id`) REFERENCES `accounts` (`id`) ON UPDATE NO ACTION ON DELETE NO ACTION,
  CHECK (json_valid(content))
) STRICT;
-- Create index "idx_transactions_account_id" to table: "transactions"
CREATE INDEX `idx_transactions_account_id` ON `transactions` (`account_id`);
-- Create index "idx_transactions_account_entry_ref" to table: "transactions"
CREATE UNIQUE INDEX `idx_transactions_account_entry_ref` ON `transactions` (`account_id`, `entry_reference`) WHERE entry_reference IS NOT NULL;
-- Create "balances" table
CREATE TABLE `balances` (
  `id` integer NULL,
  `account_id` integer NOT NULL,
  `balance_type` text NOT NULL,
  `content` text NOT NULL,
  `inserted_at` text NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
  PRIMARY KEY (`id`),
  CONSTRAINT `0` FOREIGN KEY (`account_id`) REFERENCES `accounts` (`id`) ON UPDATE NO ACTION ON DELETE NO ACTION,
  CHECK (json_valid(content))
) STRICT;
-- Create index "idx_balances_lookup" to table: "balances"
CREATE INDEX `idx_balances_lookup` ON `balances` (`account_id`, `balance_type`, `inserted_at` DESC);
-- Create "metadata_schemas" table
CREATE TABLE `metadata_schemas` (
  `target_type` text NOT NULL,
  `json_schema` text NOT NULL,
  `inserted_at` text NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
  `updated_at` text NOT NULL,
  PRIMARY KEY (`target_type`),
  CHECK (target_type IN ('transactions', 'accounts', 'balances')),
  CHECK (json_valid(json_schema))
) STRICT;
-- Create "transaction_metadata" table
CREATE TABLE `transaction_metadata` (
  `transaction_id` integer NULL,
  `user_metadata` text NOT NULL,
  `inserted_at` text NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
  `updated_at` text NOT NULL,
  PRIMARY KEY (`transaction_id`),
  CONSTRAINT `0` FOREIGN KEY (`transaction_id`) REFERENCES `transactions` (`id`) ON UPDATE NO ACTION ON DELETE CASCADE,
  CHECK (json_valid(user_metadata))
) STRICT;
-- Create "account_metadata" table
CREATE TABLE `account_metadata` (
  `account_id` integer NULL,
  `user_metadata` text NOT NULL,
  `inserted_at` text NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
  `updated_at` text NOT NULL,
  PRIMARY KEY (`account_id`),
  CONSTRAINT `0` FOREIGN KEY (`account_id`) REFERENCES `accounts` (`id`) ON UPDATE NO ACTION ON DELETE CASCADE,
  CHECK (json_valid(user_metadata))
) STRICT;
-- Create "balance_metadata" table
CREATE TABLE `balance_metadata` (
  `balance_id` integer NULL,
  `user_metadata` text NOT NULL,
  `inserted_at` text NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
  `updated_at` text NOT NULL,
  PRIMARY KEY (`balance_id`),
  CONSTRAINT `0` FOREIGN KEY (`balance_id`) REFERENCES `balances` (`id`) ON UPDATE NO ACTION ON DELETE CASCADE,
  CHECK (json_valid(user_metadata))
) STRICT;
