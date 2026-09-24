use crate::cli::IdAction;
use crate::error::AppError;
use crate::models::{AccountId, BalanceId, Resource, ResourceType, TransactionId};

pub async fn run(action: IdAction) -> Result<(), AppError> {
    match action {
        IdAction::Encode { kind, id } => {
            let encoded = match kind {
                ResourceType::Account => AccountId::from(id).to_string(),
                ResourceType::Balance => BalanceId::from(id).to_string(),
                ResourceType::Transaction => TransactionId::from(id).to_string(),
            };
            println!("{encoded}");
        }
        IdAction::Decode { resource } => {
            let raw = match resource {
                Resource::Account(ref id) => i64::try_from(id).unwrap(),
                Resource::Balance(ref id) => i64::try_from(id).unwrap(),
                Resource::Transaction(ref id) => i64::try_from(id).unwrap(),
            };
            println!("{raw}");
        }
    }
    Ok(())
}
