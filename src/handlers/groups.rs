// ustvarjanje in prikaz skupin

/*
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
    handlers::auth::get_current_user,
    services::group_service::{
        add_member_to_group,
        create_group,
    },
};

// ====================================
//            NOVA SKUPINA
// ====================================

#[derive(Template)]
#[template(path = "new_group.html")]
struct NewGroupTemplate {
    has_error: bool,
    error_message: String,
}

#[derive(Deserialize)]
pub struct NewGroupForm {
    name: String,
}

impl NewGroupForm {
    fn validate(&self) -> Result<(), &'static str> {
        if self.name.trim().is_empty() {
            return Err("Ime skupine je obvezno.");
        }

        Ok(())
    }
}

// ====================================
//             handlerja
// ====================================

// prikaz obrazca za novo skupino
pub async fn new_group_form(State(state): State<AppState>, jar: CookieJar) -> Response {
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

    let template = NewGroupTemplate {
        has_error: false,
        error_message: String::new(),
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu obrazca za novo skupino: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response()
        }
    }
}

// ustvarjanje nove skupine
pub async fn create_group_handler(State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<NewGroupForm>,
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
                "Pri ustvarjanju skupine je prišlo do napake.",
            )
                .into_response();
        }
    };

    if let Err(error_message) = form.validate() {
        let template = NewGroupTemplate {
            has_error: true,
            error_message: error_message.to_string(),
        };

        return match template.render() {
            Ok(html) => (StatusCode::BAD_REQUEST, Html(html)).into_response(),

            Err(error) => {
                eprintln!("Napaka pri izrisu obrazca za novo skupino: {error}");

                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        };
    }

    let name = form.name.trim();

    let group = match create_group(&state.db, name).await {
        Ok(group) => group,

        Err(error) => {
            eprintln!("Napaka pri ustvarjanju skupine: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri ustvarjanju skupine je prišlo do napake.",
            )
                .into_response();
        }
    };

    if let Err(error) = add_member_to_group(&state.db, group.id, user.id).await {
        eprintln!("Napaka pri dodajanju uporabnika v skupino: {error}");

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Pri ustvarjanju skupine je prišlo do napake.",
        )
            .into_response();
    }

    Redirect::to("/").into_response()
}

*/