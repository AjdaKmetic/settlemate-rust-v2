use askama::Template;
use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;

use serde::Deserialize;

use crate::{
    app::state::AppState,
    handlers::auth::get_current_user,
    services::{
        password_service::verify_password,
        user_service::{update_user_name, update_user_password},
    },
};

// podatki za html predlogo
#[derive(Template)]
#[template(path = "account.html")]
struct AccountTemplate {
    name: String,
    username: String,
    email: String,
}

#[derive(Deserialize)]
pub struct UpdateNameForm {
    name: String,
}

#[derive(Deserialize)]
pub struct UpdatePasswordForm {
    current_password: String,
    new_password: String,
    new_password_confirmation: String,
}

pub async fn account_page(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = match get_current_user(&state, &jar).await {
        Ok(Some(user)) => user, // user je prijavljen, shranimo njegovo vrednost

        Ok(None) => {
            return Redirect::to("/login").into_response(); // user ni prijavljen -> preusmeritev na "login" stran
        }

        Err(error) => {
            //napaka v bazi
            eprintln!("Napaka pri prikazu računa: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu računa je prišlo do napake.",
            )
                .into_response();
        }
    };

    let template = AccountTemplate {
        name: user.name,
        username: user.username,
        email: user.email,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(), // izris uspe

        Err(error) => {
            // ce se predloga ne more izrisati
            eprintln!("Napaka pri izrisu strani računa: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu računa je prišlo do napake.",
            )
                .into_response()
        }
    }
}

pub async fn update_name(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<UpdateNameForm>,
) -> Response {
    let user = match get_current_user(&state, &jar).await {
        // poisce se prijavljen uporabnik
        Ok(Some(user)) => user, // uporabnik je prijavljen. njegov model shranimo v spremenljivko user

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Prišlo je do napake. Poskusite znova.",
            )
                .into_response();
        }
    };
    // preverjanje novega imena - da ni prazno
    if form.name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Ime ne sme biti prazno.").into_response();
    }

    // posodobimo ime v bazi
    match update_user_name(&state.db, user.id, &form.name).await {
        Ok(_) => Redirect::to("/account").into_response(),

        Err(error) => {
            eprintln!("Napaka pri spreminjanju imena: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Imena ni bilo mogoče spremeniti.",
            )
                .into_response()
        }
    }
}

// sprememba gesla
pub async fn change_password(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<UpdatePasswordForm>,
) -> Response {
    let user = match get_current_user(&state, &jar).await {
        Ok(Some(user)) => user,

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Prišlo je do napake. Poskusite znova.",
            )
                .into_response();
        }
    };

    if !verify_password(
        // a je trenutno geslo pravilno?
        &form.current_password,
        &user.password_hash,
    ) {
        return (StatusCode::UNAUTHORIZED, "Trenutno geslo ni pravilno.").into_response();
    }

    if form.new_password.chars().count() < 8 {
        // dovolj znakov
        return (
            StatusCode::BAD_REQUEST,
            "Novo geslo mora vsebovati najmanj 8 znakov.",
        )
            .into_response();
    }

    if form.new_password // gesli se morata ujemat
        != form.new_password_confirmation
    {
        return (StatusCode::BAD_REQUEST, "Novi gesli se ne ujemata.").into_response();
    }

    match update_user_password(
        // shrani se novo geslo/hash
        &state.db,
        user.id,
        &form.new_password,
    )
    .await
    {
        Ok(_) => Redirect::to("/account").into_response(),

        Err(error) => {
            eprintln!("Napaka pri spreminjanju gesla: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Gesla ni bilo mogoče spremeniti.",
            )
                .into_response()
        }
    }
}
