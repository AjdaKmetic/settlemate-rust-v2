use crate::entities::users;
use crate::services::password_service::{hash_password, verify_password};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use users::Entity as Users;

// ========================
// funkcije za delo z bazo:
// ========================

// shranjevanje uporabnika v bazo
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
    id: i32,
) -> Result<Option<users::Model>, sea_orm::DbErr> {
    Users::find_by_id(id).one(db).await
}

// iskanje več uporabnikov naenkrat po seznamu id-jev
pub async fn find_users_by_ids(
    db: &DatabaseConnection,
    ids: Vec<i32>,
) -> Result<Vec<users::Model>, sea_orm::DbErr> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    Users::find()
        .filter(users::Column::Id.is_in(ids))
        .all(db)
        .await
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

// preverjanje identitete uporabnika (z emailom ali uporabniškim imenom)
pub async fn verify_user_credentials(
    db: &DatabaseConnection,
    login: &str,
    password: &str,
) -> Result<Option<users::Model>, sea_orm::DbErr> {
    let user = if login.contains('@') {
        find_user_by_email(db, login).await? // ? vrne Option<users::Model>, če je Result = Ok(...), sicer vrne Err(e)
    } else {
        find_user_by_username(db, login).await?
    };

    // če user obstaja, preveri ali je password pravilen
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

// sprememba imena uporabnika
pub async fn update_user_name(
    db: &DatabaseConnection,
    user_id: i32,
    name: &str,
) -> Result<users::Model, sea_orm::DbErr> {
    let user = users::ActiveModel {
        id: Set(user_id),
        name: Set(name.trim().to_string()),
        ..Default::default()
    };

    user.update(db).await
}

// sprememba gesla uporabnika
pub async fn update_user_password(
    db: &DatabaseConnection,
    user_id: i32,
    new_password: &str, // prejme novo geslo
) -> Result<users::Model, sea_orm::DbErr> {
    let password_hash = hash_password(new_password); // pretvori v hash

    let user = users::ActiveModel {
        // poisce se uporabnik
        id: Set(user_id),
        password_hash: Set(password_hash), // spremeni se samo geslo
        ..Default::default()
    };

    user.update(db).await
}
