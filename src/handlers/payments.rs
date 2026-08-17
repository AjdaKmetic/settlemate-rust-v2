use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;

use crate::{
    app::state::AppState,
    handlers::{
        auth::get_current_user,
        errors::internal_error,
        friends::{FriendView, get_friend_views},
    },
    services::{
        balance_service::get_balance, friend_service::get_friends, payment_service::settle_debt,
    },
};

#[derive(Template)]
#[template(path = "partials/payment_result.html")]
struct PaymentResultTemplate {
    friends: Vec<FriendView>,
    balance_state_class: &'static str,
    balance_label: &'static str,
    formatted_balance: String,
}

pub async fn settle_debt_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(other_user_id): Path<i32>,
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
                "Pri poravnavi dolga je prišlo do napake.",
            )
                .into_response();
        }
    };

    if let Err(error) = settle_debt(&state.db, user.id, other_user_id, None).await {
        eprintln!("Napaka pri poravnavi dolga: {error}");

        return (StatusCode::BAD_REQUEST, "Dolga ni bilo mogoče poravnati.").into_response();
    }

    let friends = match get_friends(&state.db, user.id).await {
        Ok(friends) => friends,

        Err(error) => {
            return internal_error(
                "Napaka pri pridobivanju prijateljev",
                error,
                "Pri prikazu prijateljev je prišlo do napake.",
            );
        }
    };

    let friends = match get_friend_views(&state.db, user.id, friends).await {
        Ok(friends) => friends,

        Err(error) => {
            return internal_error(
                "Napaka pri pripravi podatkov o prijateljih",
                error,
                "Pri prikazu prijateljev je prišlo do napake.",
            );
        }
    };

    let balance_cents = match get_balance(&state.db, user.id).await {
        Ok(balance) => balance,

        Err(error) => {
            return internal_error(
                "Napaka pri izračunu stanja uporabnika",
                error,
                "Pri izračunu stanja uporabnika je prišlo do napake.",
            );
        }
    };

    let (balance_state_class, balance_label) = if balance_cents > 0 {
        ("balance-positive", "Prejmeš")
    } else if balance_cents < 0 {
        ("balance-negative", "Dolguješ")
    } else {
        ("balance-neutral", "Vse štima")
    };

    let absolute = balance_cents.unsigned_abs();
    let euros = absolute / 100;
    let cents = absolute % 100;

    let template = PaymentResultTemplate {
        friends,
        balance_state_class,
        balance_label,
        formatted_balance: format!("{euros},{cents:02} €"),
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => internal_error(
            "Napaka pri izrisu poravnave dolga",
            error,
            "Pri poravnavi dolga je prišlo do napake.",
        ),
    }
}
