use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Expenses::Table)
                    .if_not_exists()
                    .col(pk_auto(Expenses::Id))
                    .col(string(Expenses::Description).not_null())
                    .col(big_integer(Expenses::AmountCents).not_null())
                    .col(integer(Expenses::PaidBy).not_null())
                    .col(integer_null(Expenses::GroupId))
                    .col(string(Expenses::SplitType).not_null())
                    .col(
                        timestamp(Expenses::CreatedAt)
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Expenses::Table, Expenses::PaidBy)
                            .to(Users::Table, Users::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Expenses::Table, Expenses::GroupId)
                            .to(Groups::Table, Groups::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Expenses::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Expenses {
    Table,
    Id,
    Description,
    AmountCents,
    PaidBy,
    GroupId,
    SplitType,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Groups {
    Table,
    Id,
}