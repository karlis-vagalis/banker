use crate::cli::IdAction;
use crate::error::AppError;
use crate::models::{AccountId, BalanceId, Resource, ResourceType, TransactionId};

pub async fn run(action: IdAction) -> Result<(), AppError> {
    match action {
        IdAction::Encode { kind, id } => {
            let encoded = match kind {
                ResourceType::Account => AccountId::try_from(id)
                    .map_err(|e| AppError::Other(e.into()))?
                    .to_string(),
                ResourceType::Balance => BalanceId::try_from(id)
                    .map_err(|e| AppError::Other(e.into()))?
                    .to_string(),
                ResourceType::Transaction => TransactionId::try_from(id)
                    .map_err(|e| AppError::Other(e.into()))?
                    .to_string(),
            };
            println!("{encoded}");
        }
        IdAction::Decode { resource } => {
            let raw = match resource {
                Resource::Account(ref id) => id.raw(),
                Resource::Balance(ref id) => id.raw(),
                Resource::Transaction(ref id) => id.raw(),
            };
            println!("{raw}");
        }
    }
    Ok(())
}
