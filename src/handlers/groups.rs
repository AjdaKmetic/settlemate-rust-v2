// ustvarjanje in prikaz skupin

use askama::Template;

use axum::{
    Form,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use sea_orm::DatabaseConnection;
use serde::Deserialize;

use crate::{
    app::state::AppState,
    entities::{groups, users},
    handlers::{
        auth::get_current_user,
        confirm::ConfirmModalTemplate,
        errors::{db_error_message, internal_error},
    },
    services::{
        balance_service::{get_balance_in_group, get_balance_with_user_in_group},
        group_service::{
            add_member_to_group, create_group, delete_group, find_group_by_id, get_group_members,
            get_groups_for_user, remove_member_from_group,
        },
        payment_service::settle_debt,
        user_service::find_user_by_username,
    },
};

#[derive(Template)]
#[template(path = "partials/group_form.html")]
struct GroupFormTemplate;

pub struct GroupView {
    pub id: i32,
    pub name: String,
    pub balance_cents: i64,
    pub formatted_balance: String,
    pub members_summary: String,
}

#[derive(Template)]
#[template(path = "partials/groups.html")]
struct GroupsTemplate {
    groups: Vec<GroupView>,
}

pub struct GroupMemberView {
    pub id: i32,
    pub name: String,
    pub username: String,
    pub balance_cents: i64,
    pub formatted_balance: String,
    pub is_current_user: bool,
}

#[derive(Template)]
#[template(path = "partials/group_detail.html")]
struct GroupDetailTemplate {
    group: groups::Model,
    members: Vec<GroupMemberView>,
}

#[derive(Template)]
#[template(path = "partials/group_member_form.html")]
struct GroupMemberFormTemplate {
    group: groups::Model,
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

pub async fn get_group_views(
    db: &DatabaseConnection,
    user_id: i32,
    groups: Vec<groups::Model>,
) -> Result<Vec<GroupView>, sea_orm::DbErr> {
    let mut group_views = Vec::new();

    for group in groups {
        let balance_cents = get_balance_in_group(db, user_id, group.id).await?;

        let absolute = balance_cents.unsigned_abs();
        let euros = absolute / 100;
        let cents = absolute % 100;

        let mut members = get_group_members(db, group.id).await?;

        // trenutno prijavljen uporabnik je zadnji
        members.sort_by_key(|member| member.id == user_id);

        let members_summary = format_members_summary(&members);

        group_views.push(GroupView {
            id: group.id,
            name: group.name,
            balance_cents,
            formatted_balance: format!("{euros},{cents:02} €"),
            members_summary,
        });
    }

    Ok(group_views)
}

pub async fn get_group_member_views(
    db: &DatabaseConnection,
    user_id: i32,
    group_id: i32,
    members: Vec<users::Model>,
) -> Result<Vec<GroupMemberView>, sea_orm::DbErr> {
    let mut member_views = Vec::new();

    for member in members {
        let balance_cents =
            get_balance_with_user_in_group(db, user_id, member.id, group_id).await?;

        let absolute = balance_cents.unsigned_abs();
        let euros = absolute / 100;
        let cents = absolute % 100;

        member_views.push(GroupMemberView {
            id: member.id,
            name: member.name,
            username: member.username,
            balance_cents,
            formatted_balance: format!("{euros},{cents:02} €"),
            is_current_user: member.id == user_id,
        });
    }

    member_views.sort_by_key(|member| member.id != user_id);

    Ok(member_views)
}

fn format_members_summary(members: &[users::Model]) -> String {
    let names: Vec<&str> = members
        .iter()
        .map(|member| member.name.as_str())
        .collect();

    match names.len() {
        0 => String::new(),
        1 => names[0].to_string(),
        2 => format!("{} in {}", names[0], names[1]),
        3 => format!("{}, {} in {}", names[0], names[1], names[2]),
        _ => format!(
            "{}, {}, {} in še {}",
            names[0],
            names[1],
            names[2],
            names.len() - 3
        ),
    }
}

// izris obrazca za člana; ob napaki vrne 200, da ga htmx zamenja
fn render_group_member_form(group: groups::Model, error_message: &str) -> Response {
    let template = GroupMemberFormTemplate {
        group,
        has_error: !error_message.is_empty(),
        error_message: error_message.to_string(),
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => internal_error(
            "Napaka pri izrisu obrazca za člana",
            error,
            "Pri prikazu obrazca je prišlo do napake.",
        ),
    }
}

// preveri, ali je uporabnik član skupine
async fn is_member(
    db: &DatabaseConnection,
    group_id: i32,
    user_id: i32,
) -> Result<bool, sea_orm::DbErr> {
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

        Err(error) => internal_error(
            "Napaka pri izrisu obrazca za skupino",
            error,
            "Pri prikazu obrazca je prišlo do napake.",
        ),
    }
}

pub async fn create_group_handler(
    State(state): State<AppState>,
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
        return (StatusCode::BAD_REQUEST, error_message).into_response();
    }

    let name = form.name.trim();

    let group = match create_group(&state.db, name).await {
        Ok(group) => group,

        Err(error) => {
            return internal_error(
                "Napaka pri ustvarjanju skupine",
                error,
                "Pri ustvarjanju skupine je prišlo do napake.",
            );
        }
    };

    // ustvarjalec postane član skupine
    if let Err(error) = add_member_to_group(&state.db, group.id, user.id).await {
        return internal_error(
            "Napaka pri dodajanju uporabnika v skupino",
            error,
            "Pri ustvarjanju skupine je prišlo do napake.",
        );
    }

    let groups = match get_groups_for_user(&state.db, user.id).await {
        Ok(groups) => groups,

        Err(error) => {
            return internal_error(
                "Napaka pri pridobivanju skupin",
                error,
                "Pri prikazu skupin je prišlo do napake.",
            );
        }
    };

    let groups = match get_group_views(&state.db, user.id, groups).await {
        Ok(groups) => groups,

        Err(error) => {
            return internal_error(
                "Napaka pri pripravi podatkov o skupinah",
                error,
                "Pri prikazu skupin je prišlo do napake.",
            );
        }
    };

    let template = GroupsTemplate { groups };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => internal_error(
            "Napaka pri izrisu skupin",
            error,
            "Pri prikazu skupin je prišlo do napake.",
        ),
    }
}

// prikaz skupine in njenih članov
pub async fn group_detail(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(group_id): Path<i32>,
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
                "Pri prikazu skupine je prišlo do napake.",
            )
                .into_response();
        }
    };

    let group = match find_group_by_id(&state.db, group_id).await {
        Ok(Some(group)) => group,

        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Skupina ne obstaja.").into_response();
        }

        Err(error) => {
            return internal_error(
                "Napaka pri iskanju skupine",
                error,
                "Pri prikazu skupine je prišlo do napake.",
            );
        }
    };

    let members = match get_group_members(&state.db, group_id).await {
        Ok(members) => members,

        Err(error) => {
            return internal_error(
                "Napaka pri pridobivanju članov skupine",
                error,
                "Pri prikazu skupine je prišlo do napake.",
            );
        }
    };

    let members = match get_group_member_views(&state.db, user.id, group_id, members).await {
        Ok(members) => members,

        Err(error) => {
            return internal_error(
                "Napaka pri pripravi podatkov o članih skupine",
                error,
                "Pri prikazu skupine je prišlo do napake.",
            );
        }
    };

    let template = GroupDetailTemplate { group, members };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => internal_error(
            "Napaka pri izrisu skupine",
            error,
            "Pri prikazu skupine je prišlo do napake.",
        ),
    }
}

// prikaz obrazca za dodajanje člana
pub async fn group_member_form(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(group_id): Path<i32>,
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

    let group = match find_group_by_id(&state.db, group_id).await {
        Ok(Some(group)) => group,

        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Skupina ne obstaja.").into_response();
        }

        Err(error) => {
            return internal_error(
                "Napaka pri iskanju skupine",
                error,
                "Pri prikazu obrazca je prišlo do napake.",
            );
        }
    };

    render_group_member_form(group, "")
}

pub async fn add_group_member_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(group_id): Path<i32>,
    Form(form): Form<AddMemberForm>,
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
                "Pri dodajanju člana je prišlo do napake.",
            )
                .into_response();
        }
    };

    // skupino potrebujemo tudi za ponovni izris obrazca ob napaki
    let group = match find_group_by_id(&state.db, group_id).await {
        Ok(Some(group)) => group,

        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Skupina ne obstaja.").into_response();
        }

        Err(error) => {
            return internal_error(
                "Napaka pri iskanju skupine",
                error,
                "Pri prikazu skupine je prišlo do napake.",
            );
        }
    };

    if let Err(error_message) = form.validate() {
        return render_group_member_form(group, error_message);
    }

    let username = form.username.trim().to_lowercase();

    let member = match find_user_by_username(&state.db, &username).await {
        Ok(Some(member)) => member,

        Ok(None) => {
            return render_group_member_form(
                group,
                "Uporabnik s tem uporabniškim imenom ne obstaja.",
            );
        }

        Err(error) => {
            return internal_error(
                "Napaka pri iskanju uporabnika",
                error,
                "Pri dodajanju člana je prišlo do napake.",
            );
        }
    };

    if let Err(error) = add_member_to_group(&state.db, group_id, member.id).await {
        // sporočilo iz servisa prikažemo v obrazcu
        let message = db_error_message(
            "Napaka pri dodajanju uporabnika v skupino",
            error,
            "Uporabnika ni bilo mogoče dodati v skupino.",
        );

        return render_group_member_form(group, &message);
    }

    let members = match get_group_members(&state.db, group_id).await {
        Ok(members) => members,

        Err(error) => {
            return internal_error(
                "Napaka pri pridobivanju članov skupine",
                error,
                "Pri prikazu skupine je prišlo do napake.",
            );
        }
    };

    let members = match get_group_member_views(&state.db, user.id, group_id, members).await {
        Ok(members) => members,

        Err(error) => {
            return internal_error(
                "Napaka pri pripravi podatkov o članih skupine",
                error,
                "Pri prikazu skupine je prišlo do napake.",
            );
        }
    };

    let template = GroupDetailTemplate { group, members };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => internal_error(
            "Napaka pri izrisu skupine",
            error,
            "Pri prikazu skupine je prišlo do napake.",
        ),
    }
}

// prikaz potrditve za zapustitev skupine
pub async fn group_leave_form(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(group_id): Path<i32>,
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

    let group = match find_group_by_id(&state.db, group_id).await {
        Ok(Some(group)) => group,

        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Skupina ne obstaja.").into_response();
        }

        Err(error) => {
            return internal_error(
                "Napaka pri iskanju skupine",
                error,
                "Pri prikazu obrazca je prišlo do napake.",
            );
        }
    };

    let members = match get_group_members(&state.db, group_id).await {
        Ok(members) => members,

        Err(error) => {
            return internal_error(
                "Napaka pri pridobivanju članov skupine",
                error,
                "Pri prikazu obrazca je prišlo do napake.",
            );
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

        Err(error) => internal_error(
            "Napaka pri izrisu potrditve za zapustitev skupine",
            error,
            "Pri prikazu obrazca je prišlo do napake.",
        ),
    }
}

// prikaz potrditve za brisanje skupine
pub async fn group_delete_form(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(group_id): Path<i32>,
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

    let group = match find_group_by_id(&state.db, group_id).await {
        Ok(Some(group)) => group,

        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Skupina ne obstaja.").into_response();
        }

        Err(error) => {
            return internal_error(
                "Napaka pri iskanju skupine",
                error,
                "Pri prikazu obrazca je prišlo do napake.",
            );
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

        Err(error) => internal_error(
            "Napaka pri izrisu potrditve za brisanje skupine",
            error,
            "Pri prikazu obrazca je prišlo do napake.",
        ),
    }
}

// uporabnik zapusti skupino
pub async fn leave_group_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(group_id): Path<i32>,
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
                "Pri zapuščanju skupine je prišlo do napake.",
            )
                .into_response();
        }
    };

    match is_member(&state.db, group_id, user.id).await {
        Ok(true) => {}

        Ok(false) => {
            return (StatusCode::FORBIDDEN, "Nimaš dostopa do te skupine.").into_response();
        }

        Err(error) => {
            return internal_error(
                "Napaka pri preverjanju članstva v skupini",
                error,
                "Pri zapuščanju skupine je prišlo do napake.",
            );
        }
    }

    let members = match get_group_members(&state.db, group_id).await {
        Ok(members) => members,

        Err(error) => {
            return internal_error(
                "Napaka pri pridobivanju članov skupine",
                error,
                "Pri zapuščanju skupine je prišlo do napake.",
            );
        }
    };

    let was_last = members.len() <= 1;

    if let Err(error) = remove_member_from_group(&state.db, group_id, user.id).await {
        return internal_error(
            "Napaka pri odstranjevanju uporabnika iz skupine",
            error,
            "Pri zapuščanju skupine je prišlo do napake.",
        );
    }

    // odšel je zadnji član, zato skupino izbrišemo
    if was_last {
        if let Err(error) = delete_group(&state.db, group_id).await {
            return internal_error(
                "Napaka pri brisanju skupine",
                error,
                "Pri zapuščanju skupine je prišlo do napake.",
            );
        }
    }

    let groups = match get_groups_for_user(&state.db, user.id).await {
        Ok(groups) => groups,

        Err(error) => {
            return internal_error(
                "Napaka pri pridobivanju skupin",
                error,
                "Pri prikazu skupin je prišlo do napake.",
            );
        }
    };

    let groups = match get_group_views(&state.db, user.id, groups).await {
        Ok(groups) => groups,

        Err(error) => {
            return internal_error(
                "Napaka pri pripravi podatkov o skupinah",
                error,
                "Pri prikazu skupin je prišlo do napake.",
            );
        }
    };

    let template = GroupsTemplate { groups };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => internal_error(
            "Napaka pri izrisu skupin",
            error,
            "Pri prikazu skupin je prišlo do napake.",
        ),
    }
}

// brisanje celotne skupine
pub async fn delete_group_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(group_id): Path<i32>,
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
                "Pri brisanju skupine je prišlo do napake.",
            )
                .into_response();
        }
    };

    match is_member(&state.db, group_id, user.id).await {
        Ok(true) => {}

        Ok(false) => {
            return (StatusCode::FORBIDDEN, "Nimaš dostopa do te skupine.").into_response();
        }

        Err(error) => {
            return internal_error(
                "Napaka pri preverjanju članstva v skupini",
                error,
                "Pri brisanju skupine je prišlo do napake.",
            );
        }
    }

    if let Err(error) = delete_group(&state.db, group_id).await {
        return internal_error(
            "Napaka pri brisanju skupine",
            error,
            "Pri brisanju skupine je prišlo do napake.",
        );
    }

    let groups = match get_groups_for_user(&state.db, user.id).await {
        Ok(groups) => groups,

        Err(error) => {
            return internal_error(
                "Napaka pri pridobivanju skupin",
                error,
                "Pri prikazu skupin je prišlo do napake.",
            );
        }
    };

    let groups = match get_group_views(&state.db, user.id, groups).await {
        Ok(groups) => groups,

        Err(error) => {
            return internal_error(
                "Napaka pri pripravi podatkov o skupinah",
                error,
                "Pri prikazu skupin je prišlo do napake.",
            );
        }
    };

    let template = GroupsTemplate { groups };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => internal_error(
            "Napaka pri izrisu skupin",
            error,
            "Pri prikazu skupin je prišlo do napake.",
        ),
    }
}

// poravnava dolga s članom znotraj skupine
pub async fn settle_group_debt_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Path((group_id, member_id)): Path<(i32, i32)>,
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

    if let Err(error) =
        settle_debt(&state.db, user.id, member_id, Some(group_id)).await
    {
        eprintln!("Napaka pri poravnavi dolga: {error}");

        return (
            StatusCode::BAD_REQUEST,
            "Dolga ni bilo mogoče poravnati.",
        )
            .into_response();
    }

    // ponovno pridobimo skupino
    let group = match find_group_by_id(&state.db, group_id).await {
        Ok(Some(group)) => group,

        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Skupina ne obstaja.").into_response();
        }

        Err(error) => {
            return internal_error(
                "Napaka pri iskanju skupine",
                error,
                "Pri prikazu skupine je prišlo do napake.",
            );
        }
    };

    // ponovno pridobimo člane, da se stanje takoj osveži
    let members = match get_group_members(&state.db, group_id).await {
        Ok(members) => members,

        Err(error) => {
            return internal_error(
                "Napaka pri pridobivanju članov skupine",
                error,
                "Pri prikazu skupine je prišlo do napake.",
            );
        }
    };

    let members =
        match get_group_member_views(&state.db, user.id, group_id, members).await {
            Ok(members) => members,

            Err(error) => {
                return internal_error(
                    "Napaka pri pripravi podatkov o članih skupine",
                    error,
                    "Pri prikazu skupine je prišlo do napake.",
                );
            }
        };

    let template = GroupDetailTemplate { group, members };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => internal_error(
            "Napaka pri izrisu skupine",
            error,
            "Pri prikazu skupine je prišlo do napake.",
        ),
    }
}