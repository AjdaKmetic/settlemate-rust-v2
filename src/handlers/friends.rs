// dodajanje prijateljev

use askama::Template;

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use crate::{
    app::state::AppState,
    entities::users,
    handlers::auth::get_current_user,
    services::{
        balance_service::get_balance_with_user,
        friend_service::{
            add_friend,
            get_friends,
        },
        user_service::find_user_by_username,
    },
};

#[derive(Template)]
#[template(path = "partials/friend_form.html")]
struct FriendFormTemplate;

pub struct FriendView {
    pub id: i32,
    pub name: String,
    pub username: String,
    pub balance_cents: i64,
    pub formatted_balance: String,
}

#[derive(Template)]
#[template(path = "partials/friends.html")]
struct FriendsTemplate {
    friends: Vec<FriendView>,
}

#[derive(Deserialize)]
pub struct AddFriendForm {
    username: String,
}

impl AddFriendForm {
    fn validate(&self) -> Result<(), &'static str> {
        if self.username.trim().is_empty() {
            return Err("Uporabniško ime je obvezno.");
        }

        Ok(())
    }
}

pub async fn get_friend_views(db: &sea_orm::DatabaseConnection, user_id: i32, friends: Vec<users::Model>) -> Result<Vec<FriendView>, sea_orm::DbErr> {
    let mut friend_views = Vec::new();

    for friend in friends {
        let balance_cents = get_balance_with_user(db, user_id, friend.id).await?;

        let absolute = balance_cents.unsigned_abs();
        let euros = absolute / 100;
        let cents = absolute % 100;

        friend_views.push(FriendView {
            id: friend.id,
            name: friend.name,
            username: friend.username,
            balance_cents,
            formatted_balance: format!("{euros},{cents:02} €"),
        });
    }

    Ok(friend_views)
}

// ====================================
//            handlerji
// ====================================

// prikaz obrazca
pub async fn friend_form() -> Response {
    let template = FriendFormTemplate;

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu obrazca za prijatelja: {error}");
            
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response()
        }
    }
}

pub async fn add_friend_handler(State(state): State<AppState>, jar: CookieJar, Form(form): Form<AddFriendForm>) -> Response {
    let user = match get_current_user(&state, &jar).await {
        Ok(Some(user)) => user,

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri dodajanju prijatelja je prišlo do napake.",
            )
                .into_response();
        }
    };

    if let Err(error_message) = form.validate() {
        return (
            StatusCode::BAD_REQUEST,
            error_message,
        )
            .into_response();
    }

    let username = form.username.trim().to_lowercase();

    let friend = match find_user_by_username(&state.db, &username).await {
        Ok(Some(friend)) => friend,

        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "Uporabnik s tem uporabniškim imenom ne obstaja.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri iskanju uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri dodajanju prijatelja je prišlo do napake.",
            )
                .into_response();
        }
    };

    if let Err(error) = add_friend(&state.db, user.id, friend.id).await {
        eprintln!("Napaka pri dodajanju prijatelja: {error}");

        return (
            StatusCode::BAD_REQUEST,
            error.to_string(),
        )
            .into_response();
    }

    let friends = match get_friends(&state.db, user.id).await {
        Ok(friends) => friends,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju prijateljev: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu prijateljev je prišlo do napake.",
            )
                .into_response();
        }
    };

    let friends = match get_friend_views(&state.db, user.id, friends).await {
        Ok(friends) => friends,

        Err(error) => {
            eprintln!("Napaka pri pripravi podatkov o prijateljih: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu prijateljev je prišlo do napake.",
            )
                .into_response();
        }
    };

    let template = FriendsTemplate {
        friends,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu prijateljev: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu prijateljev je prišlo do napake.",
            )
                .into_response()
        }
    }
}