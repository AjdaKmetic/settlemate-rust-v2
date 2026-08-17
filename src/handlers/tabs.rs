// preklapljanje med zavihki

use askama::Template;

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use std::collections::HashMap;

use crate::{
    app::state::AppState,
    entities::users,
    handlers::{
        auth::get_current_user,
        expenses::{ActivityItem, build_activities},
        friends::{FriendView, get_friend_views},
        groups::{GroupView, get_group_views},
    },
    services::{
        expense_service::get_expenses_for_user, friend_service::get_friends,
        group_service::get_groups_for_user, user_service::find_users_by_ids,
    },
};

#[derive(Template)]
#[template(path = "partials/tab_shell.html")]
struct TabShellTemplate {
    active_tab: &'static str,
    friends: Vec<FriendView>,
    groups: Vec<GroupView>,
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

    let template = TabShellTemplate {
        active_tab: "friends",
        friends,
        groups: Vec::new(),     // Askama zahteva polje groups v vseh vejah
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

    let groups = match get_groups_for_user(&state.db, user.id).await {
        Ok(groups) => groups,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju skupin: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu skupin je prišlo do napake.",
            )
                .into_response();
        }
    };

    let groups = match get_group_views(&state.db, user.id, groups).await {
        Ok(groups) => groups,

        Err(error) => {
            eprintln!("Napaka pri pripravi podatkov o skupinah: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu skupin je prišlo do napake.",
            )
                .into_response();
        }
    };

    let template = TabShellTemplate {
        active_tab: "groups",
        friends: Vec::new(), // Askama zahteva polje friends v vseh vejah
        groups,
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

    // imena plačnikov pridobimo s poizvedbo
    let mut payer_ids: Vec<i32> = expenses.iter().map(|expense| expense.paid_by).collect();
    payer_ids.sort();
    payer_ids.dedup();

    let payers = match find_users_by_ids(&state.db, payer_ids).await {
        Ok(payers) => payers,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju plačnikov: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu aktivnosti je prišlo do napake.",
            )
                .into_response();
        }
    };

    let payer_names: HashMap<i32, String> = payers
        .into_iter()
        .map(|payer| (payer.id, payer.name))
        .collect();

    let template = TabShellTemplate {
        active_tab: "activity",
        friends: Vec::new(), // Askama zahteva polje friends v vseh vejah
        groups: Vec::new(),  // Askama zahteva polje groups v vseh vejah
        activities: build_activities(expenses, user.id, &payer_names),
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
