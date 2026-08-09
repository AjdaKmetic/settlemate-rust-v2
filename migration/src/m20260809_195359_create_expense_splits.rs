use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ExpenseSplits::Table)
                    .if_not_exists()
                    .col(pk_auto(ExpenseSplits::Id))
                    .col(integer(ExpenseSplits::ExpenseId).not_null())
                    .col(integer(ExpenseSplits::UserId).not_null())
                    .col(big_integer(ExpenseSplits::AmountCents).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(ExpenseSplits::Table, ExpenseSplits::ExpenseId)
                            .to(Expenses::Table, Expenses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ExpenseSplits::Table, ExpenseSplits::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ExpenseSplits::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum ExpenseSplits {
    Table,
    Id,
    ExpenseId,
    UserId,
    AmountCents,
}

#[derive(DeriveIden)]
enum Expenses {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}