use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GroupMembers::Table)
                    .if_not_exists()
                    .col(pk_auto(GroupMembers::Id))
                    .col(integer(GroupMembers::GroupId).not_null())
                    .col(integer(GroupMembers::UserId).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(GroupMembers::Table, GroupMembers::GroupId)
                            .to(Groups::Table, Groups::Id)
                            .on_delete(ForeignKeyAction::Cascade), // pove, kaj se zgodi z vrstico v tabeli group_members, če se izbriše vrstica v tabeli groups (Cascade, SetNull, Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(GroupMembers::Table, GroupMembers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx-group-members-group-user")
                            .col(GroupMembers::GroupId)
                            .col(GroupMembers::UserId)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GroupMembers::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum GroupMembers {
    Table,
    Id,
    GroupId,
    UserId,
}

#[derive(DeriveIden)]
enum Groups {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}