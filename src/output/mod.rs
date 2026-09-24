mod accounts;
mod balances;
mod transactions;

pub use accounts::{print_api_accounts, print_db_accounts};
pub use balances::{print_api_balances, print_db_balances};
pub use transactions::{print_api_transactions, print_db_transactions};

pub enum OutputFormat {
    Table,
    Jsonl,
    Lines,
}
