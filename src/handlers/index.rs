// prikaz glavne strani prijavljenega uporabnika

use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;

use crate::{
    app::state::AppState,
    handlers::auth::get_current_user,
    services::balance_service::get_balance,
};

// Askama
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    username: String,
    balance_state_class: &'static str,
    balance_label: &'static str,
    formatted_balance: String,
}

impl IndexTemplate {
    fn new(username: String, balance_cents: i64) -> Self {
        let (balance_state_class, balance_label) =
            if balance_cents > 0 {
                (
                    "balance-positive",
                    "Dolgujejo ti",
                )
            } else if balance_cents < 0 {
                (
                    "balance-negative",
                    "Dolguješ",
                )
            } else {
                (
                    "balance-neutral",
                    "Vse štima", // Brez dolgov
                )
            };

        let absolute = balance_cents.unsigned_abs();
        let euros = absolute / 100;
        let cents = absolute % 100;

        let formatted_balance = format!("{euros},{cents:02} €");

        Self {
            username,
            balance_state_class,
            balance_label,
            formatted_balance,
        }
    }
}

// ====================================
//              handler
// ====================================

pub async fn index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = match get_current_user(&state, &jar).await {
        Ok(Some(user)) => user,

        // če uporabnik ni prijavljen, ne prikazujemo index.html
        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu začetne strani je prišlo do napake.",
            )
                .into_response();
        }
    };

    let balance_cents = match get_balance(&state.db, user.id).await {
        Ok(balance) => balance,

        Err(error) => {
            eprintln!("Napaka pri izračunu stanja uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri izračunu stanja uporabnika je prišlo do napake.",
            )
                .into_response();
        }
    };

    let template = IndexTemplate::new(user.username, balance_cents);

    match template.render() {
        Ok(html) => Html(html).into_response(), // = (StatusCode::OK, Html(html)).into_response()

        Err(error) => {
            eprintln!("Napaka pri izrisu začetne strani: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu začetne strani je prišlo do napake.",
            )
                .into_response()
        }
    }
}