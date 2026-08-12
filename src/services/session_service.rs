use sea_orm::{
    ActiveModelTrait,
    ColumnTrait,
    DatabaseConnection,
    DeleteResult,
    EntityTrait,
    QueryFilter,
    Set,
};
use uuid::Uuid;

use crate::entities::sessions;

// ustvarjanje nove seje za uporabnika
pub async fn create_session(db: &DatabaseConnection, user_id: i32) -> Result<String, sea_orm::DbErr> {
    let token = Uuid::new_v4().to_string();

    let session = sessions::ActiveModel {
        token: Set(token.clone()),
        user_id: Set(user_id),
        ..Default::default()
    };

    session.insert(db).await?;

    Ok(token)
}

// iskanje seje glede na token
pub async fn find_session_by_token(db: &DatabaseConnection, token: &str) -> Result<Option<sessions::Model>, sea_orm::DbErr> {
    sessions::Entity::find()
        .filter(sessions::Column::Token.eq(token))
        .one(db)
        .await
}

// brisanje seje iz baze
pub async fn delete_session(db: &DatabaseConnection, token: &str) -> Result<DeleteResult, sea_orm::DbErr> {
    sessions::Entity::delete_many()
        .filter(sessions::Column::Token.eq(token))
        .exec(db)
        .await
}