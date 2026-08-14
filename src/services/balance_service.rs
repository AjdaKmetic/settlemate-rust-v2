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

fn calculate_balance(
    paid_cents: i64,
    owed_cents: i64,
    sent_cents: i64,
    received_cents: i64,
) -> i64 {
    paid_cents - owed_cents + sent_cents - received_cents
}

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

    Ok(calculate_balance(
        paid_cents, 
        owed_cents, 
        sent_cents, 
        received_cents))
}


// TESTI

#[cfg(test)]
mod tests {
    use super::calculate_balance;

    #[test]
    fn brez_stroskov_je_stanje_nic() {
        let balance = calculate_balance(0, 0, 0, 0);

        assert_eq!(balance, 0);
    }

    #[test]
    fn placnik_dobi_pozitivno_stanje() {
        // Uporabnik je plačal 100 €, njegov delež pa je 40 €.
        let balance = calculate_balance(10000, 4000, 0, 0);

        assert_eq!(balance, 6000);
    }

    #[test]
    fn dolznik_dobi_negativno_stanje() {
        // Uporabnik ni plačal nič, dolguje pa 60 €.
        let balance = calculate_balance(0, 6000, 0, 0);

        assert_eq!(balance, -6000);
    }

    #[test]
    fn poslano_placilo_zmanjsa_dolg() {
        // Uporabnik dolguje 60 € in je že poslal 25 €.
        let balance = calculate_balance(0, 6000, 2500, 0);

        assert_eq!(balance, -3500);
    }

    #[test]
    fn prejeto_placilo_zmanjsa_terjatev() {
        // Uporabniku dolgujejo 60 €, prejel pa je že 25 €.
        let balance = calculate_balance(10000, 4000, 0, 2500);

        assert_eq!(balance, 3500);
    }
}