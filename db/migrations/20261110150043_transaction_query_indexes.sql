-- Create index "idx_transactions_booking_date" to table: "transactions"
CREATE INDEX `idx_transactions_booking_date` ON `transactions` ((json_extract(content, '$.booking_date')) DESC, `id` DESC);
-- Create index "idx_transactions_account_booking_date" to table: "transactions"
CREATE INDEX `idx_transactions_account_booking_date` ON `transactions` (`account_id`, (json_extract(content, '$.booking_date')) DESC, `id` DESC);
