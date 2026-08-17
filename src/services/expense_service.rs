use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};

use crate::entities::{expense_splits, expenses};

pub async fn create_equal_expense(
    db: &DatabaseConnection,
    description: &str,
    amount_cents: i64,
    paid_by: i32,
    friend_id: i32,
) -> Result<expenses::Model, sea_orm::DbErr> {
    let description = description.trim(); // opis stroška

    if description.is_empty() {
        // ne sme bit prazen
        return Err(sea_orm::DbErr::Custom(
            "Opis stroška ne sme biti prazen.".to_string(),
        ));
    }

    if amount_cents <= 0 {
        // ne sme bit negativen
        return Err(sea_orm::DbErr::Custom(
            "Znesek mora biti večji od nič.".to_string(),
        ));
    }

    let transaction = db.begin().await?; // začne transakcijo, če kaj do konca ne uspe, se zavrže vse

    let expense = expenses::ActiveModel {
        // strošek
        description: Set(description.to_string()),
        amount_cents: Set(amount_cents),
        paid_by: Set(paid_by),
        group_id: Set(None),
        split_type: Set("equal".to_string()),
        ..Default::default()
    }
    .insert(&transaction)
    .await?;

    let friend_share_cents = amount_cents / 2;
    let payer_share_cents = amount_cents - friend_share_cents; // pri lihem številu centov plačnik plača cent več

    let payer_split = expense_splits::ActiveModel {
        // zabeleži se plačnikov delež
        expense_id: Set(expense.id),
        user_id: Set(paid_by),
        amount_cents: Set(payer_share_cents),
        ..Default::default()
    }
    .insert(&transaction)
    .await?; // vključimo v ta paket transakcija

    let friend_split = expense_splits::ActiveModel {
        // prijateljev delež
        expense_id: Set(expense.id),
        user_id: Set(friend_id),
        amount_cents: Set(friend_share_cents),
        ..Default::default()
    }
    .insert(&transaction)
    .await?;

    transaction.commit().await?; // zaključimo transakcijo

    Ok(expense)
}

pub async fn get_expenses_for_user(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<Vec<expenses::Model>, sea_orm::DbErr> {
    expenses::Entity::find()
        .inner_join(expense_splits::Entity) // povežemo tabeli expenses in expenses_splits
        .filter(expense_splits::Column::UserId.eq(user_id)) // omejimo rezultate na prijavnega uporabnika
        .order_by_desc(expenses::Column::CreatedAt) // uredimo po času nastanka
        .all(db)
        .await
}

pub async fn find_expense_by_id(
    db: &DatabaseConnection,
    expense_id: i32,
) -> Result<Option<expenses::Model>, sea_orm::DbErr> {
    expenses::Entity::find_by_id(expense_id).one(db).await
}

// deleži posameznega stroška
pub async fn get_expense_splits(
    db: &DatabaseConnection,
    expense_id: i32,
) -> Result<Vec<expense_splits::Model>, sea_orm::DbErr> {
    expense_splits::Entity::find()
        .filter(expense_splits::Column::ExpenseId.eq(expense_id))
        .all(db)
        .await
}

// sprememba opisa stroška
pub async fn update_expense_description(
    db: &DatabaseConnection,
    expense_id: i32,
    description: &str,
) -> Result<expenses::Model, sea_orm::DbErr> {
    let description = description.trim();

    if description.is_empty() {
        // ne sme bit prazen
        return Err(sea_orm::DbErr::Custom(
            "Opis stroška ne sme biti prazen.".to_string(),
        ));
    }

    let expense = expenses::ActiveModel {
        id: Set(expense_id),
        description: Set(description.to_string()), // spremeni se samo opis
        ..Default::default()
    };

    expense.update(db).await
}

// brisanje stroška skupaj z njegovimi deleži
pub async fn delete_expense(
    db: &DatabaseConnection,
    expense_id: i32,
) -> Result<(), sea_orm::DbErr> {
    let transaction = db.begin().await?; // brez deležev strošek ne sme ostati

    expense_splits::Entity::delete_many()
        .filter(expense_splits::Column::ExpenseId.eq(expense_id))
        .exec(&transaction)
        .await?;

    expenses::Entity::delete_many()
        .filter(expenses::Column::Id.eq(expense_id))
        .exec(&transaction)
        .await?;

    transaction.commit().await?;

    Ok(())
}
