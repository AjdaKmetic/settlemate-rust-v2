use sea_orm::{
    ColumnTrait,
    DatabaseConnection,
    EntityTrait,
    QueryFilter,
};

use crate::entities::{
    expense_splits,
    expenses,
    payments,
};

// izračun skupnega stanja uporabnika
pub async fn get_balance(db: &DatabaseConnection, user_id: i32) -> Result<i64, sea_orm::DbErr> {

    // stroški, ki jih je uporabnik plačal
    let paid_expenses = expenses::Entity::find()
        .filter(expenses::Column::PaidBy.eq(user_id))
        .all(db)
        .await?;

    // deleži stroškov, ki pripadajo uporabniku
    let user_splits = expense_splits::Entity::find()
        .filter(expense_splits::Column::UserId.eq(user_id))
        .all(db)
        .await?;

    // plačila, ki jih je uporabnik poslal
    let sent_payments = payments::Entity::find()
        .filter(payments::Column::FromUser.eq(user_id))
        .all(db)
        .await?;

    // plačila, ki jih je uporabnik prejel
    let received_payments = payments::Entity::find()
        .filter(payments::Column::ToUser.eq(user_id))
        .all(db)
        .await?;

    let paid_cents: i64 = paid_expenses
        .iter()
        .map(|expense| expense.amount_cents)
        .sum();

    let owed_cents: i64 = user_splits
        .iter()
        .map(|split| split.amount_cents)
        .sum();

    let sent_cents: i64 = sent_payments
        .iter()
        .map(|payment| payment.amount_cents)
        .sum();

    let received_cents: i64 = received_payments
        .iter()
        .map(|payment| payment.amount_cents)
        .sum();

    Ok(paid_cents - owed_cents + sent_cents - received_cents)
}