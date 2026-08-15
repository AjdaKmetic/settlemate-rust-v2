use sea_orm::{
    ActiveModelTrait,
    ColumnTrait,
    DatabaseConnection,
    EntityTrait,
    JoinType,
    QueryFilter,
    QuerySelect,
    RelationTrait,
    Set,
    TransactionTrait,
};

use crate::entities::{
    friendships,
    users,
};

pub async fn are_friends(db: &DatabaseConnection, user_id: i32, friend_id: i32) -> Result<bool, sea_orm::DbErr> {
    let friendship = friendships::Entity::find()
        .filter(friendships::Column::UserId.eq(user_id))
        .filter(friendships::Column::FriendId.eq(friend_id))
        .one(db)
        .await?;

    Ok(friendship.is_some()) // če je Some vrne true, sicer false
}

pub async fn add_friend(db: &DatabaseConnection, user_id: i32, friend_id: i32) -> Result<(), sea_orm::DbErr> {
    if user_id == friend_id {
        return Err(sea_orm::DbErr::Custom(
            "Ne moreš dodati samega sebe za prijatelja.".to_string(),
        ));
    }

    if are_friends(db, user_id, friend_id).await? {
        return Err(sea_orm::DbErr::Custom(
            "Ta uporabnik je že tvoj prijatelj.".to_string(),
        ));
    }

    let transaction = db.begin().await?;

    let friendship_1 = friendships::ActiveModel {
        user_id: Set(user_id),
        friend_id: Set(friend_id),
        ..Default::default()
    };

    friendship_1.insert(&transaction).await?;

    let friendship_2 = friendships::ActiveModel {
        user_id: Set(friend_id),
        friend_id: Set(user_id),
        ..Default::default()
    };

    friendship_2.insert(&transaction).await?;

    transaction.commit().await?;

    Ok(())
}

pub async fn get_friends(db: &DatabaseConnection, user_id: i32) -> Result<Vec<users::Model>, sea_orm::DbErr> {
    users::Entity::find()
        .join(
            JoinType::InnerJoin,
            friendships::Relation::Users2.def().rev(),
        )
        .filter(friendships::Column::UserId.eq(user_id))
        .all(db)
        .await
}

/*
remove_friend()
*/