// dodajanje prijateljev

use askama::Template;

use axum::{
    Form,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use std::collections::HashMap;

use crate::{
    app::state::AppState,
    entities::users,
    handlers::{
        auth::get_current_user,
        confirm::ConfirmModalTemplate,
        expenses::{ActivityItem, build_activities},
    },
    services::{
        balance_service::get_balance_with_user,
        expense_service::get_shared_expenses,
        friend_service::{add_friend, are_friends, get_friends, remove_friend},
        user_service::{find_user_by_id, find_user_by_username, find_users_by_ids},
    },
};

#[derive(Template)]
#[template(path = "partials/friend_form.html")]
struct FriendFormTemplate {
    has_error: bool,
    error_message: String,
}

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

#[derive(Template)]
#[template(path = "partials/friend_detail.html")]
struct FriendDetailTemplate {
    friend: users::Model,
    balance_cents: i64,
    formatted_balance: String,
    expenses: Vec<ActivityItem>,
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

// zapis stanja brez predznaka, ki ga dopolni oznaka v predlogi
fn format_balance(balance_cents: i64) -> String {
    let absolute = balance_cents.unsigned_abs();
    let euros = absolute / 100;
    let cents = absolute % 100;

    format!("{euros},{cents:02} €")
}

pub async fn get_friend_views(
    db: &sea_orm::DatabaseConnection,
    user_id: i32,
    friends: Vec<users::Model>,
) -> Result<Vec<FriendView>, sea_orm::DbErr> {
    let mut friend_views = Vec::new();

    for friend in friends {
        let balance_cents = get_balance_with_user(db, user_id, friend.id).await?;

        friend_views.push(FriendView {
            id: friend.id,
            name: friend.name,
            username: friend.username,
            balance_cents,
            formatted_balance: format_balance(balance_cents),
        });
    }

    Ok(friend_views)
}

// ====================================
//            handlerji
// ====================================

// izris obrazca; ob napaki vrne 200, da ga htmx zamenja
fn render_friend_form(error_message: &str) -> Response {
    let template = FriendFormTemplate {
        has_error: !error_message.is_empty(),
        error_message: error_message.to_string(),
    };

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

// prikaz obrazca
pub async fn friend_form() -> Response {
    render_friend_form("")
}

pub async fn add_friend_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<AddFriendForm>,
) -> Response {
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
        return render_friend_form(error_message);
    }

    let username = form.username.trim().to_lowercase();

    let friend = match find_user_by_username(&state.db, &username).await {
        Ok(Some(friend)) => friend,

        Ok(None) => {
            return render_friend_form("Uporabnik s tem uporabniškim imenom ne obstaja.");
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
        return match error {
            // sporočilo iz servisa prikažemo v obrazcu
            sea_orm::DbErr::Custom(message) => render_friend_form(&message),

            other => {
                eprintln!("Napaka pri dodajanju prijatelja: {other}");

                render_friend_form("Pri dodajanju prijatelja je prišlo do napake.")
            }
        };
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

    let template = FriendsTemplate { friends };

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

// prikaz prijatelja in stroškov, ki jih delita
pub async fn friend_detail(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(friend_id): Path<i32>,
) -> Response {
    let user = match get_current_user(&state, &jar).await {
        Ok(Some(user)) => user,

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu prijatelja je prišlo do napake.",
            )
                .into_response();
        }
    };

    let friend = match find_user_by_id(&state.db, friend_id).await {
        Ok(Some(friend)) => friend,

        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Prijatelj ne obstaja.").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri iskanju uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu prijatelja je prišlo do napake.",
            )
                .into_response();
        }
    };

    match are_friends(&state.db, user.id, friend.id).await {
        Ok(true) => {}

        Ok(false) => {
            return (StatusCode::FORBIDDEN, "Nimaš dostopa do tega uporabnika.").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijateljstva: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu prijatelja je prišlo do napake.",
            )
                .into_response();
        }
    }

    let balance_cents = match get_balance_with_user(&state.db, user.id, friend.id).await {
        Ok(balance) => balance,

        Err(error) => {
            eprintln!("Napaka pri izračunu stanja med prijateljema: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu prijatelja je prišlo do napake.",
            )
                .into_response();
        }
    };

    let expenses = match get_shared_expenses(&state.db, user.id, friend.id).await {
        Ok(expenses) => expenses,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju skupnih stroškov: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu skupnih stroškov je prišlo do napake.",
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
                "Pri prikazu skupnih stroškov je prišlo do napake.",
            )
                .into_response();
        }
    };

    let payer_names: HashMap<i32, String> = payers
        .into_iter()
        .map(|payer| (payer.id, payer.name))
        .collect();

    let template = FriendDetailTemplate {
        expenses: build_activities(expenses, user.id, &payer_names),
        balance_cents,
        formatted_balance: format_balance(balance_cents),
        friend,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu prijatelja: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu prijatelja je prišlo do napake.",
            )
                .into_response()
        }
    }
}

// prikaz potrditve za odstranitev prijatelja
pub async fn friend_delete_form(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(friend_id): Path<i32>,
) -> Response {
    match get_current_user(&state, &jar).await {
        Ok(Some(_)) => {}

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response();
        }
    }

    let template = ConfirmModalTemplate {
        title: "Odstrani prijatelja".to_string(),
        message: "Ali res želiš odstraniti tega prijatelja?".to_string(),
        cancel_label: "Prekliči".to_string(),
        confirm_label: "Odstrani".to_string(),
        confirm_url: format!("/friends/{friend_id}/delete"),
        confirm_target: "#friends-panel".to_string(),
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu potrditve za odstranitev prijatelja: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response()
        }
    }
}

// odstranitev prijatelja
pub async fn remove_friend_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(friend_id): Path<i32>,
) -> Response {
    let user = match get_current_user(&state, &jar).await {
        Ok(Some(user)) => user,

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri odstranjevanju prijatelja je prišlo do napake.",
            )
                .into_response();
        }
    };

    let balance = match get_balance_with_user(&state.db, user.id, friend_id).await {
        Ok(balance) => balance,

        Err(error) => {
            eprintln!("Napaka pri izračunu stanja med prijateljema: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri odstranjevanju prijatelja je prišlo do napake.",
            )
                .into_response();
        }
    };

    if balance != 0 {
        return (
            StatusCode::BAD_REQUEST,
            "Pred odstranitvijo prijatelja morajo biti dolgovi poravnani.",
        )
            .into_response();
    }

    if let Err(error) = remove_friend(&state.db, user.id, friend_id).await {
        eprintln!("Napaka pri odstranjevanju prijatelja: {error}");

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Pri odstranjevanju prijatelja je prišlo do napake.",
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

    let template = FriendsTemplate { friends };

    match template.render() {
        Ok(html) => {
            let response = format!(r#"{html}<div id="modal-root" hx-swap-oob="innerHTML"></div>"#);

            Html(response).into_response()
        }

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
