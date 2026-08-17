use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

use crate::{entities::payments, services::balance_service::get_balance_with_user};

pub async fn settle_debt(
    db: &DatabaseConnection,
    user_id: i32,
    other_user_id: i32,
) -> Result<payments::Model, sea_orm::DbErr> {
    let balance = get_balance_with_user(db, user_id, other_user_id).await?;

    // "varovalka" za backend
    if balance >= 0 {
        return Err(sea_orm::DbErr::Custom("Nimaš dolga.".to_string()));
    }

    let payment = payments::ActiveModel {
        from_user: Set(user_id),
        to_user: Set(other_user_id),
        amount_cents: Set(-balance),
        group_id: Set(None),
        ..Default::default()
    };

    payment.insert(db).await
}
