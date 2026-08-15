use sea_orm::{
    ActiveModelTrait,
    ColumnTrait,
    DatabaseConnection,
    EntityTrait,
    QueryFilter,
    Set,
};

use crate::entities::{
    group_members,
    groups,
    users,
};

pub async fn create_group(db: &DatabaseConnection, name: &str) -> Result<groups::Model, sea_orm::DbErr> {
    let group = groups::ActiveModel {
        name: Set(name.to_string()),
        ..Default::default()
    };

    group.insert(db).await
}

pub async fn add_member_to_group(db: &DatabaseConnection, group_id: i32, user_id: i32) -> Result<group_members::Model, sea_orm::DbErr> {
    let member = group_members::ActiveModel {
        group_id: Set(group_id),
        user_id: Set(user_id),
        ..Default::default()
    };

    member.insert(db).await
}

// iskanje vseh skupin, v katerih je uporabnik
pub async fn get_groups_for_user(db: &DatabaseConnection, user_id: i32) -> Result<Vec<groups::Model>, sea_orm::DbErr> {
    groups::Entity::find()
        .inner_join(group_members::Entity) //združi obe tabeli
        .filter(group_members::Column::UserId.eq(user_id))
        .all(db)
        .await
}

pub async fn get_group_members(db: &DatabaseConnection, group_id: i32) -> Result<Vec<users::Model>, sea_orm::DbErr> {
    users::Entity::find()
        .inner_join(group_members::Entity)
        .filter(group_members::Column::GroupId.eq(group_id))
        .all(db)
        .await
}

pub async fn find_group_by_id(db: &DatabaseConnection, group_id: i32) -> Result<Option<groups::Model>, sea_orm::DbErr> {
    groups::Entity::find_by_id(group_id)
        .one(db)
        .await
}

/*
delete_group(...)
update_group_name(...)
remove_member_from_group(...)
*/