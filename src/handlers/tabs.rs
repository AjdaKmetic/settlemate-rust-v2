// preklapljanje med zavihki

use askama::Template;

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;

use crate::{
    app::state::AppState,
    entities::users,
    handlers::{
        auth::get_current_user,
        expenses::{ActivityItem, build_activities},
    },
    services::{
        expense_service::get_expenses_for_user,
        friend_service::get_friends,
    },
};

#[derive(Template)]
#[template(path = "partials/tab_shell.html")]
struct TabShellTemplate {
    active_tab: &'static str,
    friends: Vec<users::Model>,
    activities: Vec<ActivityItem>,
}

// ====================================
//            handlerji
// ====================================

// zavihek s prijatelji
pub async fn friends_tab(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = match get_current_user(&state, &jar).await {
        Ok(Some(user)) => user,

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu zavihka je prišlo do napake.",
            )
                .into_response();
        }
    };

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

    let template = TabShellTemplate {
        active_tab: "friends",
        friends,
        activities: Vec::new(), // Askama zahteva polje activities v vseh vejah
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu zavihka: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu zavihka je prišlo do napake.",
            )
                .into_response()
        }
    }
}

// zavihek s skupinami
pub async fn groups_tab(State(state): State<AppState>, jar: CookieJar) -> Response {
    match get_current_user(&state, &jar).await {
        Ok(Some(_)) => {}

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu zavihka je prišlo do napake.",
            )
                .into_response();
        }
    }

    let template = TabShellTemplate {
        active_tab: "groups",
        friends: Vec::new(), // Askama zahteva polje friends v vseh vejah
        activities: Vec::new(), // Askama zahteva polje activities v vseh vejah
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu zavihka: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu zavihka je prišlo do napake.",
            )
                .into_response()
        }
    }
}

// zavihek z aktivnostjo
pub async fn activity_tab(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = match get_current_user(&state, &jar).await {
        Ok(Some(user)) => user,

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu zavihka je prišlo do napake.",
            )
                .into_response();
        }
    };

    let expenses = match get_expenses_for_user(&state.db, user.id).await {
        Ok(expenses) => expenses,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju aktivnosti: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu aktivnosti je prišlo do napake.",
            )
                .into_response();
        }
    };

    let template = TabShellTemplate {
        active_tab: "activity",
        friends: Vec::new(), // Askama zahteva polje friends v vseh vejah
        activities: build_activities(expenses, user.id),
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu zavihka: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu zavihka je prišlo do napake.",
            )
                .into_response()
        }
    }
}
