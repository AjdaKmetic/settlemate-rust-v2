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
    entities::groups,
    handlers::{
        auth::get_current_user,
        expenses::ActivityItem,
        friends::{FriendView, get_friend_views},
    },
    services::{
        balance_service::get_balance,
        friend_service::get_friends,
    },
};

// Askama
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    username: String,
    balance_state_class: &'static str,
    balance_label: &'static str,
    formatted_balance: String,
    friends: Vec<FriendView>,
    groups: Vec<groups::Model>,
    active_tab: &'static str,
    activities: Vec<ActivityItem>,
}

impl IndexTemplate {
    fn new(username: String, balance_cents: i64, friends: Vec<FriendView>) -> Self {
        let (balance_state_class, balance_label) =
            if balance_cents > 0 {
                (
                    "balance-positive",
                    "Prejmeš",
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
            friends,
            groups: Vec::new(), // Askama zahteva polje groups v vseh vejah
            active_tab: "friends",
            activities: Vec::new(), // Askama zahteva polje activities v vseh vejah
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

    let friends = match get_friends(&state.db, user.id).await {
        Ok(friends) => friends,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju prijateljev: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu začetne strani je prišlo do napake.",
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

    let template = IndexTemplate::new(user.username, balance_cents, friends);

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

// TESTI

#[cfg(test)]
mod tests {
    use super::IndexTemplate;

    #[test]
    fn pozitivno_stanje_je_pravilno_prikazano() {
        let template = IndexTemplate::new(
            "ajda".to_string(),
            12345,
            Vec::new(),
        );

        assert_eq!(template.balance_state_class, "balance-positive");
        assert_eq!(template.balance_label, "Prejmeš");
        assert_eq!(template.formatted_balance, "123,45 €");
    }

    #[test]
    fn negativno_stanje_je_pravilno_prikazano() {
        let template = IndexTemplate::new(
            "ajda".to_string(),
            -123,
            Vec::new(),
        );
        assert_eq!(template.balance_state_class, "balance-negative");
        assert_eq!(template.balance_label, "Dolguješ");
        assert_eq!(template.formatted_balance, "1,23 €");
    }

    #[test]
    fn nicelno_stanje_je_pravilno_prikazano() {
        let template = IndexTemplate::new(
            "ajda".to_string(),
            0,
            Vec::new(),
        );
        assert_eq!(template.balance_state_class, "balance-neutral");
        assert_eq!(template.balance_label, "Vse štima");
        assert_eq!(template.formatted_balance, "0,00 €");
    }
}
