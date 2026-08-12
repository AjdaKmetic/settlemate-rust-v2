// handlerji za avtentikacijo uporabnika (registracija, prijava, odjava, ali je uporabnik prijavljen, ustvarjanje seje)

use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;

use crate::{
    app::state::AppState,
    entities::users,
    services::{
        session_service::{
            create_session,
            delete_session,
            find_session_by_token,
        },   
        user_service::{
            create_user,
            find_user_by_email,
            find_user_by_id,
            find_user_by_username,
            verify_user_credentials,
        },
    },
};

// ====================================
//      REGISTRACIJA UPORABNIKA
// ====================================

#[derive(Template)] // Askama - doda metodo render() za izris HTML predloge
#[template(path = "register.html")]
struct RegisterTemplate {
    has_error: bool,
    error_message: String,
    created: bool,
}

// iz podatkov pripravi HTML odgovor za brskalnik
fn render_register(status: StatusCode, error_message: &str, created: bool) -> Response {
    let template = RegisterTemplate {
        has_error: !error_message.is_empty(),
        error_message: error_message.to_string(),
        created,
    };

    match template.render() { // Askama (odpre register.html, vstavi vrednosti iz template,) vrne String, ki je HTML - vrne Result<String, Error>
        Ok(html) => (status, Html(html)).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu registracijske strani: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu registracijske strani je prišlo do napake.",
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)] // Axum pretvori podatke, ki jih brskalnik pošlje strežniku, v Rust struct RegisterForm
pub struct RegisterForm {
    name: String,
    username: String,
    email: String,
    password: String,
    password_confirmation: String,
}

// ====================================
//             handlerja
// ====================================

pub async fn register_form() -> Response {
    render_register(StatusCode::OK, "", false) // izdela register.html z obrazcem za registracijo
}

pub async fn register_user(State(state): State<AppState>, Form(form): Form<RegisterForm>) -> Response {
    let name = form.name.trim(); // &str
    let username = form.username.trim().to_lowercase(); // String
    let email = form.email.trim().to_lowercase();

    // pregledamo, če so vpisani podatki veljavni
    if name.is_empty() {
        return render_register(
            StatusCode::BAD_REQUEST,
            "Ime je obvezno.",
            false,
        );
    }

    if username.is_empty() {
        return render_register(
            StatusCode::BAD_REQUEST,
            "Uporabniško ime je obvezno.",
            false,
        );
    }

    if !email.contains('@') {
        return render_register(
            StatusCode::BAD_REQUEST,
            "Vnesi veljaven e-poštni naslov.",
            false,
        );
    }

    if form.password.chars().count() < 8 {
        return render_register(
            StatusCode::BAD_REQUEST,
            "Geslo mora vsebovati najmanj 8 znakov.",
            false,
        );
    }

    if form.password != form.password_confirmation {
        return render_register(
            StatusCode::BAD_REQUEST,
            "Gesli se ne ujemata.",
            false,
        );
    }

    // preverimo, ali ta uporabnik že obstaja
    match find_user_by_username(&state.db, &username).await {
        // uporabnik obstaja
        Ok(Some(_)) => {
            return render_register(
                StatusCode::CONFLICT,
                "To uporabniško ime je že zasedeno.",
                false,
            );
        }

        // uporabnik s tem username-om ne obstaja
        Ok(None) => {}

        // poizvedba po bazi ni uspela
        Err(error) => {
            eprintln!("Napaka pri registraciji uporabnika: {error}");

            return render_register(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri registraciji je prišlo do napake. Poskusite znova.",
                false,
            );
        }
    }

    match find_user_by_email(&state.db, &email).await {
        Ok(Some(_)) => {
            return render_register(
                StatusCode::CONFLICT,
                "Uporabnik s tem e-poštnim naslovom že obstaja.",
                false,
            );
        }

        Ok(None) => {}

        Err(error) => {
            eprintln!("Napaka pri registraciji uporabnika: {error}");

            return render_register(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri registraciji je prišlo do napake. Poskusite znova.",
                false,
            );
        }
    }

    match create_user(&state.db, name, &username, &email, &form.password).await {
        // registracija je uspešna
        Ok(_) => render_register(StatusCode::CREATED, "", true),

        Err(error) => {
            eprintln!("Napaka pri ustvarjanju uporabnika: {error}");

            render_register(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri registraciji je prišlo do napake. Poskusite znova.",
                false,
            )
        }
    }

}

// ====================================
//         PRIJAVA UPORABNIKA
// ====================================

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    has_error: bool,
    error_message: String,
}

fn render_login(status: StatusCode, error_message: &str) -> Response {
    let template = LoginTemplate {
        has_error: !error_message.is_empty(),
        error_message: error_message.to_string(),
    };

    match template.render() {
        Ok(html) => (status, Html(html)).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu prijavne strani: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu prijavne strani je prišlo do napake.",
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct LoginForm {
    login: String, // ujema se z imenom atributa v HTML obrazcu
    password: String,
}

// ====================================
//             handlerja
// ====================================

pub async fn login_form() -> Response {
    render_login(StatusCode::OK, "")
}

pub async fn login_user(State(state): State<AppState>, jar: CookieJar, Form(form): Form<LoginForm>) -> Response {
    let login = form.login.trim().to_lowercase();

    let user = match verify_user_credentials(&state.db, &login, &form.password).await {
        Ok(Some(user)) => user,

        Ok(None) => {
            return render_login(
                StatusCode::UNAUTHORIZED,
                "Napačno uporabniško ime, e-poštni naslov ali geslo.",
            );
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavnih podatkov: {error}");

            return render_login(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prijavi je prišlo do napake. Poskusite znova.",
            );
        }
    };

    let token = match create_session(&state.db, user.id).await {
        Ok(token) => token,

        Err(error) => {
            eprintln!("Napaka pri ustvarjanju seje: {error}");

            return render_login(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prijavi je prišlo do napake. Poskusite znova.",
            );
        }
    };

    let cookie = Cookie::build(("settlemate_session", token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax);

    (jar.add(cookie), Redirect::to("/")).into_response()
}

// ====================================
//    TRENUTNO PRIJAVLJEN UPORABNIK
// ====================================

pub async fn get_current_user(state: &AppState, jar: &CookieJar) -> Result<Option<users::Model>, sea_orm::DbErr> {
    let cookie = match jar.get("settlemate_session") {
        Some(cookie) => cookie,
        None => return Ok(None),
    };

    let token = cookie.value();

    let session = match find_session_by_token(&state.db, token).await? {
        Some(session) => session,
        None => return Ok(None),
    };

    find_user_by_id(&state.db, session.user_id).await
}

// ====================================
//          ODJAVA UPORABNIKA
// ====================================

// ====================================
//              handler
// ====================================

pub async fn logout_user(State(state): State<AppState>, jar: CookieJar) -> Response {
    let cookie = match jar.get("settlemate_session") {
        Some(cookie) => cookie,
        None => return Redirect::to("/login").into_response(),
    };

    let token = cookie.value();

    // brisanje seje iz baze
    if let Err(error) = delete_session(&state.db, token).await {
        eprintln!("Napaka pri brisanju seje: {error}");

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Pri odjavi je prišlo do napake. Poskusite znova.",
        )
            .into_response();
    }

    // odstranitev cookie-ja iz brskalnika
    let jar = jar.remove(Cookie::from("settlemate_session"));

    (jar, Redirect::to("/login")).into_response()
    
}