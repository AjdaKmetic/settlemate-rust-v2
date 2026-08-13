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
};

// Askama
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    username: String,
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

    let template = IndexTemplate {
        username: user.username,
    };

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