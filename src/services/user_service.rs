use crate::services::db::auth_service::{hash_password, verify_password};
use crate::{entities::users, models::user::UserId};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DeleteResult, EntityTrait, IntoActiveModel,
    QueryFilter, Set,
};
use users::Entity as Users;

// registracija uporabnika
pub async fn create_user(
    db: &DatabaseConnection,
    name: &str,
    username: &str,
    email: &str,
    password: &str,
) -> Result<users::Model, sea_orm::DbErr> {
    let password_hash = hash_password(password);
    let new_user = users::ActiveModel {
        name: Set(name.to_string()),
        username: Set(username.to_string()),
        email: Set(email.to_string()),
        password_hash: Set(password_hash),
        ..Default::default()
    };

    new_user.insert(db).await
}

// iskanje uporabnika v bazi
pub async fn find_user_by_username(
    db: &DatabaseConnection,
    username: &str,
) -> Result<Option<users::Model>, sea_orm::DbErr> {
    Users::find()
        .filter(users::Column::Username.eq(username))
        .one(db)
        .await
}

pub async fn find_user_by_id(
    db: &DatabaseConnection,
    id: UserId,
) -> Result<Option<users::Model>, sea_orm::DbErr> {
    Users::find_by_id(id).one(db).await
}

pub async fn find_user_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<users::Model>, sea_orm::DbErr> {
    Users::find()
        .filter(users::Column::Email.eq(email))
        .one(db)
        .await
}

// log in (z emailom ali uporabniškim imenom)
pub async fn login_user(
    db: &DatabaseConnection,
    login: &str,
    password: &str,
) -> Result<Option<users::Model>, sea_orm::DbErr> {
    let user = if login.contains('@') {
        find_user_by_email(db, login).await?
    } else {
        find_user_by_username(db, login).await?
    };

    // če user obstaja, preveri password (vrni user ali None)
    match user {
        Some(user) => {
            if verify_password(password, &user.password_hash) {
                Ok(Some(user))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}
