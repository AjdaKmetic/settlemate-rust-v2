// ustvarjanje in prikaz skupin

use askama::Template;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::CookieJar;
use sea_orm::DatabaseConnection;
use serde::Deserialize;

use crate::{
    app::state::AppState,
    entities::{groups, users},
    handlers::auth::get_current_user,
    services::{
        group_service::{
            add_member_to_group,
            create_group,
            delete_group,
            find_group_by_id,
            get_group_members,
            get_groups_for_user,
            remove_member_from_group,
        },
        user_service::find_user_by_username,
    },
};

#[derive(Template)]
#[template(path = "partials/group_form.html")]
struct GroupFormTemplate;

#[derive(Template)]
#[template(path = "partials/groups.html")]
struct GroupsTemplate {
    groups: Vec<groups::Model>,
}

#[derive(Template)]
#[template(path = "partials/group_detail.html")]
struct GroupDetailTemplate {
    group: groups::Model,
    members: Vec<users::Model>,
}

#[derive(Template)]
#[template(path = "partials/group_member_form.html")]
struct GroupMemberFormTemplate {
    group: groups::Model,
}

// splošno potrditveno okno, uporabno za vse potrditve
#[derive(Template)]
#[template(path = "partials/confirm_modal.html")]
struct ConfirmModalTemplate {
    title: String,
    message: String,
    cancel_label: String,
    confirm_label: String,
    confirm_url: String,
    confirm_target: String,
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

#[derive(Deserialize)]
pub struct AddMemberForm {
    username: String,
}

impl AddMemberForm {
    fn validate(&self) -> Result<(), &'static str> {
        if self.username.trim().is_empty() {
            return Err("Uporabniško ime je obvezno.");
        }

        Ok(())
    }
}

// preveri, ali je uporabnik član skupine
async fn is_member(db: &DatabaseConnection, group_id: i32, user_id: i32) -> Result<bool, sea_orm::DbErr> {
    let members = get_group_members(db, group_id).await?;

    Ok(members.iter().any(|member| member.id == user_id))
}

// ====================================
//             handlerji
// ====================================

// prikaz obrazca
pub async fn group_form() -> Response {
    let template = GroupFormTemplate;

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu obrazca za skupino: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response()
        }
    }
}

pub async fn create_group_handler(State(state): State<AppState>, jar: CookieJar, Form(form): Form<NewGroupForm>) -> Response {
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
        return (
            StatusCode::BAD_REQUEST,
            error_message,
        )
            .into_response();
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

    // ustvarjalec postane član skupine
    if let Err(error) = add_member_to_group(&state.db, group.id, user.id).await {
        eprintln!("Napaka pri dodajanju uporabnika v skupino: {error}");

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Pri ustvarjanju skupine je prišlo do napake.",
        )
            .into_response();
    }

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

    let template = GroupsTemplate {
        groups,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu skupin: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu skupin je prišlo do napake.",
            )
                .into_response()
        }
    }
}

// prikaz skupine in njenih članov
pub async fn group_detail(State(state): State<AppState>, jar: CookieJar, Path(group_id): Path<i32>) -> Response {
    match get_current_user(&state, &jar).await {
        Ok(Some(_)) => {}

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu skupine je prišlo do napake.",
            )
                .into_response();
        }
    }

    let group = match find_group_by_id(&state.db, group_id).await {
        Ok(Some(group)) => group,

        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "Skupina ne obstaja.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri iskanju skupine: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu skupine je prišlo do napake.",
            )
                .into_response();
        }
    };

    let members = match get_group_members(&state.db, group_id).await {
        Ok(members) => members,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju članov skupine: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu skupine je prišlo do napake.",
            )
                .into_response();
        }
    };

    let template = GroupDetailTemplate {
        group,
        members,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu skupine: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu skupine je prišlo do napake.",
            )
                .into_response()
        }
    }
}

// prikaz obrazca za dodajanje člana
pub async fn group_member_form(State(state): State<AppState>, jar: CookieJar, Path(group_id): Path<i32>) -> Response {
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

    let group = match find_group_by_id(&state.db, group_id).await {
        Ok(Some(group)) => group,

        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "Skupina ne obstaja.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri iskanju skupine: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response();
        }
    };

    let template = GroupMemberFormTemplate {
        group,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu obrazca za člana: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response()
        }
    }
}

pub async fn add_group_member_handler(State(state): State<AppState>, jar: CookieJar, Path(group_id): Path<i32>, Form(form): Form<AddMemberForm>) -> Response {
    match get_current_user(&state, &jar).await {
        Ok(Some(_)) => {}

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri dodajanju člana je prišlo do napake.",
            )
                .into_response();
        }
    }

    if let Err(error_message) = form.validate() {
        return (
            StatusCode::BAD_REQUEST,
            error_message,
        )
            .into_response();
    }

    let username = form.username.trim().to_lowercase();

    let member = match find_user_by_username(&state.db, &username).await {
        Ok(Some(member)) => member,

        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "Uporabnik s tem uporabniškim imenom ne obstaja.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri iskanju uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri dodajanju člana je prišlo do napake.",
            )
                .into_response();
        }
    };

    if let Err(error) = add_member_to_group(&state.db, group_id, member.id).await {
        eprintln!("Napaka pri dodajanju uporabnika v skupino: {error}");

        return (
            StatusCode::BAD_REQUEST,
            "Uporabnika ni bilo mogoče dodati v skupino.",
        )
            .into_response();
    }

    let group = match find_group_by_id(&state.db, group_id).await {
        Ok(Some(group)) => group,

        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "Skupina ne obstaja.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri iskanju skupine: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu skupine je prišlo do napake.",
            )
                .into_response();
        }
    };

    let members = match get_group_members(&state.db, group_id).await {
        Ok(members) => members,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju članov skupine: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu skupine je prišlo do napake.",
            )
                .into_response();
        }
    };

    let template = GroupDetailTemplate {
        group,
        members,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu skupine: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu skupine je prišlo do napake.",
            )
                .into_response()
        }
    }
}

// prikaz potrditve za zapustitev skupine
pub async fn group_leave_form(State(state): State<AppState>, jar: CookieJar, Path(group_id): Path<i32>) -> Response {
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

    let group = match find_group_by_id(&state.db, group_id).await {
        Ok(Some(group)) => group,

        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "Skupina ne obstaja.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri iskanju skupine: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response();
        }
    };

    let members = match get_group_members(&state.db, group_id).await {
        Ok(members) => members,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju članov skupine: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response();
        }
    };

    let is_last_member = members.len() <= 1;

    let template = ConfirmModalTemplate {
        title: "Zapusti skupino".to_string(),

        message: if is_last_member {
            "Skupina bo izbrisana, ker si zadnji član. Nadaljujem?".to_string()
        } else {
            "Ali res želiš zapustiti to skupino?".to_string()
        },

        cancel_label: "Prekliči".to_string(),
        confirm_label: "Zapusti".to_string(),
        confirm_url: format!("/groups/{}/leave", group.id),
        confirm_target: "#groups-panel".to_string(),
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu potrditve za zapustitev skupine: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response()
        }
    }
}

// prikaz potrditve za brisanje skupine
pub async fn group_delete_form(State(state): State<AppState>, jar: CookieJar, Path(group_id): Path<i32>) -> Response {
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

    let group = match find_group_by_id(&state.db, group_id).await {
        Ok(Some(group)) => group,

        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "Skupina ne obstaja.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri iskanju skupine: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response();
        }
    };

    let template = ConfirmModalTemplate {
        title: "Izbriši skupino".to_string(),
        message: "Skupina bo trajno izbrisana za vse člane. Nadaljujem?".to_string(),
        cancel_label: "Prekliči".to_string(),
        confirm_label: "Izbriši".to_string(),
        confirm_url: format!("/groups/{}/delete", group.id),
        confirm_target: "#groups-panel".to_string(),
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu potrditve za brisanje skupine: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response()
        }
    }
}

// uporabnik zapusti skupino
pub async fn leave_group_handler(State(state): State<AppState>, jar: CookieJar, Path(group_id): Path<i32>) -> Response {
    let user = match get_current_user(&state, &jar).await {
        Ok(Some(user)) => user,

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri zapuščanju skupine je prišlo do napake.",
            )
                .into_response();
        }
    };

    match is_member(&state.db, group_id, user.id).await {
        Ok(true) => {}

        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                "Nimaš dostopa do te skupine.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju članstva v skupini: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri zapuščanju skupine je prišlo do napake.",
            )
                .into_response();
        }
    }

    let members = match get_group_members(&state.db, group_id).await {
        Ok(members) => members,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju članov skupine: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri zapuščanju skupine je prišlo do napake.",
            )
                .into_response();
        }
    };

    let was_last = members.len() <= 1;

    if let Err(error) = remove_member_from_group(&state.db, group_id, user.id).await {
        eprintln!("Napaka pri odstranjevanju uporabnika iz skupine: {error}");

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Pri zapuščanju skupine je prišlo do napake.",
        )
            .into_response();
    }

    // odšel je zadnji član, zato skupino izbrišemo
    if was_last {
        if let Err(error) = delete_group(&state.db, group_id).await {
            eprintln!("Napaka pri brisanju skupine: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri zapuščanju skupine je prišlo do napake.",
            )
                .into_response();
        }
    }

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

    let template = GroupsTemplate {
        groups,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu skupin: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu skupin je prišlo do napake.",
            )
                .into_response()
        }
    }
}

// brisanje celotne skupine
pub async fn delete_group_handler(State(state): State<AppState>, jar: CookieJar, Path(group_id): Path<i32>) -> Response {
    let user = match get_current_user(&state, &jar).await {
        Ok(Some(user)) => user,

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri brisanju skupine je prišlo do napake.",
            )
                .into_response();
        }
    };

    match is_member(&state.db, group_id, user.id).await {
        Ok(true) => {}

        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                "Nimaš dostopa do te skupine.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju članstva v skupini: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri brisanju skupine je prišlo do napake.",
            )
                .into_response();
        }
    }

    if let Err(error) = delete_group(&state.db, group_id).await {
        eprintln!("Napaka pri brisanju skupine: {error}");

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Pri brisanju skupine je prišlo do napake.",
        )
            .into_response();
    }

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

    let template = GroupsTemplate {
        groups,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu skupin: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu skupin je prišlo do napake.",
            )
                .into_response()
        }
    }
}
