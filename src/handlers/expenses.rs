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
use std::collections::HashMap;

use crate::{
    app::state::AppState,
    entities::{expenses, users},
    handlers::{
        auth::get_current_user,
        groups::ConfirmModalTemplate,
    },
    services::{
        expense_service::{
            create_equal_expense,
            delete_expense,
            find_expense_by_id,
            get_expense_splits,
            get_expenses_for_user,
            update_expense_description,
        },
        friend_service::get_friends,
        user_service::find_users_by_ids,
    },
};

#[derive(Template)]
#[template(path = "partials/expense_form.html")]
struct ExpenseFormTemplate {
    friends: Vec<users::Model>,
}

// posamezen delež za prikaz razdelitve
struct SplitRow {
    name: String,
    amount: String,
}

#[derive(Template)]
#[template(path = "partials/expense_detail.html")]
struct ExpenseDetailTemplate {
    expense: expenses::Model,
    payer_label: String,
    formatted_amount: String,
    splits: Vec<SplitRow>,
}

#[derive(Template)]
#[template(path = "partials/expense_description_form.html")]
struct ExpenseDescriptionFormTemplate {
    expense: expenses::Model,
}

#[derive(Template)]
#[template(path = "partials/activity.html")]
struct ActivityPanelTemplate {
    activities: Vec<ActivityItem>,
}

pub struct ActivityItem {
    pub id: i32,
    pub description: String,
    pub formatted_amount: String,
    pub payer_label: String,
}

fn format_amount(amount_cents: i64) -> String {
    let euros = amount_cents / 100;
    let cents = amount_cents % 100;

    format!("{euros},{cents:02} €")
}

// pretvorba stroškov v elemente za prikaz aktivnosti
pub fn build_activities(
    expenses: Vec<expenses::Model>,
    user_id: i32,
    payer_names: &HashMap<i32, String>,
) -> Vec<ActivityItem> {
    expenses
        .into_iter()
        .map(|expense| {
            let payer_name = payer_names
                .get(&expense.paid_by)
                .cloned()
                .unwrap_or_else(|| "Neznan uporabnik".to_string());

            ActivityItem {
                id: expense.id,
                description: expense.description,
                formatted_amount: format_amount(expense.amount_cents),

                payer_label: if expense.paid_by == user_id {
                    format!("{payer_name} (ti)")
                } else {
                    payer_name
                },
            }
        })
        .collect() // vse elemente se zdruzi
}

// podatki iz html- obrazca se pretvorijo v strukturo CreateExpenseForm
#[derive(Deserialize)]
pub struct CreateExpenseForm {
    description: String,
    amount: f64,
    friend_id: i32,
}

#[derive(Deserialize)]
pub struct UpdateDescriptionForm {
    description: String,
}

// preveri, ali je uporabnik udeležen v strošku
async fn is_participant(db: &DatabaseConnection, expense_id: i32, user_id: i32) -> Result<bool, sea_orm::DbErr> {
    let splits = get_expense_splits(db, expense_id).await?;

    Ok(splits.iter().any(|split| split.user_id == user_id))
}

// sestavljanje in izris podrobnosti stroška (uporabljeno pri prikazu in po urejanju)
async fn render_expense_detail(state: &AppState, expense_id: i32, user_id: i32) -> Response {
    let expense = match find_expense_by_id(&state.db, expense_id).await {
        Ok(Some(expense)) => expense,

        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "Strošek ne obstaja.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri iskanju stroška: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu stroška je prišlo do napake.",
            )
                .into_response();
        }
    };

    let splits = match get_expense_splits(&state.db, expense_id).await {
        Ok(splits) => splits,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju deležev: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu stroška je prišlo do napake.",
            )
                .into_response();
        }
    };

    // imena udeležencev in plačnika pridobimo s poizvedbo
    let mut user_ids: Vec<i32> = splits.iter().map(|split| split.user_id).collect();
    user_ids.push(expense.paid_by);
    user_ids.sort();
    user_ids.dedup();

    let members = match find_users_by_ids(&state.db, user_ids).await {
        Ok(members) => members,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju uporabnikov: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu stroška je prišlo do napake.",
            )
                .into_response();
        }
    };

    let names: HashMap<i32, String> = members
        .into_iter()
        .map(|member| (member.id, member.name))
        .collect();

    let payer_name = names
        .get(&expense.paid_by)
        .cloned()
        .unwrap_or_else(|| "Neznan uporabnik".to_string());

    let payer_label = if expense.paid_by == user_id {
        format!("{payer_name} (ti)")
    } else {
        payer_name
    };

    let split_rows: Vec<SplitRow> = splits
        .iter()
        .map(|split| SplitRow {
            name: names
                .get(&split.user_id)
                .cloned()
                .unwrap_or_else(|| "Neznan uporabnik".to_string()),

            amount: format_amount(split.amount_cents),
        })
        .collect();

    let formatted_amount = format_amount(expense.amount_cents);

    let template = ExpenseDetailTemplate {
        expense,
        payer_label,
        formatted_amount,
        splits: split_rows,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu stroška: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu stroška je prišlo do napake.",
            )
                .into_response()
        }
    }
}


pub async fn expense_form(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = match get_current_user(&state, &jar).await { // preverimo prijavo
        Ok(Some(user)) => user,

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca za strošek je prišlo do napake.",
            )
                .into_response();
        }
    };

    let friends = match get_friends(&state.db, user.id).await { // pridobimo prijatelje
        Ok(friends) => friends,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju prijateljev: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca za strošek je prišlo do napake.",
            )
                .into_response();
        }
    };

    let template = ExpenseFormTemplate { friends };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu obrazca za strošek: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                 "Pri prikazu obrazca za strošek je prišlo do napake.",
            )
                .into_response()
        }
    }
}

pub async fn close_expense_form() -> Html<&'static str> {
    Html("")
}


pub async fn create_expense_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<CreateExpenseForm>,
) -> Response {
    let user = match get_current_user(&state, &jar).await { // preveri prijavljenega uporabnika
        Ok(Some(user)) => user,

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri shranjevanju stroška je prišlo do napake.",
            )
                .into_response();
        }
    };

    let amount_cents = (form.amount * 100.0).round() as i64;

    match create_equal_expense(
        &state.db,
        &form.description,
        amount_cents,
        user.id,
        form.friend_id,
    )
    .await
    {
        Ok(_) => Redirect::to("/").into_response(),

        Err(error) => {
            eprintln!("Napaka pri shranjevanju stroška: {error}");

            (
                StatusCode::BAD_REQUEST,
                error.to_string(),
            )
                .into_response()
        }
    }
}

// prikaz podrobnosti stroška
pub async fn expense_detail(State(state): State<AppState>, jar: CookieJar, Path(expense_id): Path<i32>) -> Response {
    let user = match get_current_user(&state, &jar).await {
        Ok(Some(user)) => user,

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu stroška je prišlo do napake.",
            )
                .into_response();
        }
    };

    render_expense_detail(&state, expense_id, user.id).await
}

// prikaz obrazca za urejanje opisa
pub async fn expense_description_form(State(state): State<AppState>, jar: CookieJar, Path(expense_id): Path<i32>) -> Response {
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

    let expense = match find_expense_by_id(&state.db, expense_id).await {
        Ok(Some(expense)) => expense,

        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "Strošek ne obstaja.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri iskanju stroška: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response();
        }
    };

    let template = ExpenseDescriptionFormTemplate {
        expense,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu obrazca za opis stroška: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response()
        }
    }
}

// shranjevanje spremenjenega opisa
pub async fn update_expense_description_handler(State(state): State<AppState>, jar: CookieJar, Path(expense_id): Path<i32>, Form(form): Form<UpdateDescriptionForm>) -> Response {
    let user = match get_current_user(&state, &jar).await {
        Ok(Some(user)) => user,

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri spreminjanju opisa je prišlo do napake.",
            )
                .into_response();
        }
    };

    match is_participant(&state.db, expense_id, user.id).await {
        Ok(true) => {}

        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                "Nimaš dostopa do tega stroška.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju udeležbe v strošku: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri spreminjanju opisa je prišlo do napake.",
            )
                .into_response();
        }
    }

    if let Err(error) = update_expense_description(&state.db, expense_id, &form.description).await {
        eprintln!("Napaka pri spreminjanju opisa stroška: {error}");

        return (
            StatusCode::BAD_REQUEST,
            error.to_string(),
        )
            .into_response();
    }

    render_expense_detail(&state, expense_id, user.id).await
}

// prikaz potrditve za brisanje stroška
pub async fn expense_delete_form(State(state): State<AppState>, jar: CookieJar, Path(expense_id): Path<i32>) -> Response {
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

    let expense = match find_expense_by_id(&state.db, expense_id).await {
        Ok(Some(expense)) => expense,

        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "Strošek ne obstaja.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri iskanju stroška: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response();
        }
    };

    let template = ConfirmModalTemplate {
        title: "Izbriši strošek".to_string(),
        message: "Strošek bo trajno izbrisan. Nadaljujem?".to_string(),
        cancel_label: "Prekliči".to_string(),
        confirm_label: "Izbriši".to_string(),
        confirm_url: format!("/expenses/{}/delete", expense.id),
        confirm_target: "#activity-panel".to_string(),
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu potrditve za brisanje stroška: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu obrazca je prišlo do napake.",
            )
                .into_response()
        }
    }
}

// brisanje stroška
pub async fn delete_expense_handler(State(state): State<AppState>, jar: CookieJar, Path(expense_id): Path<i32>) -> Response {
    let user = match get_current_user(&state, &jar).await {
        Ok(Some(user)) => user,

        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju prijavljenega uporabnika: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri brisanju stroška je prišlo do napake.",
            )
                .into_response();
        }
    };

    match is_participant(&state.db, expense_id, user.id).await {
        Ok(true) => {}

        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                "Nimaš dostopa do tega stroška.",
            )
                .into_response();
        }

        Err(error) => {
            eprintln!("Napaka pri preverjanju udeležbe v strošku: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri brisanju stroška je prišlo do napake.",
            )
                .into_response();
        }
    }

    if let Err(error) = delete_expense(&state.db, expense_id).await {
        eprintln!("Napaka pri brisanju stroška: {error}");

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Pri brisanju stroška je prišlo do napake.",
        )
            .into_response();
    }

    // po brisanju izrišemo osvežen seznam aktivnosti
    let expenses = match get_expenses_for_user(&state.db, user.id).await {
        Ok(expenses) => expenses,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju aktivnosti: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu aktivnosti je prišlo do napake.",
            )
                .into_response();
        }
    };

    let mut payer_ids: Vec<i32> = expenses.iter().map(|expense| expense.paid_by).collect();
    payer_ids.sort();
    payer_ids.dedup();

    let payers = match find_users_by_ids(&state.db, payer_ids).await {
        Ok(payers) => payers,

        Err(error) => {
            eprintln!("Napaka pri pridobivanju plačnikov: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu aktivnosti je prišlo do napake.",
            )
                .into_response();
        }
    };

    let payer_names: HashMap<i32, String> = payers
        .into_iter()
        .map(|payer| (payer.id, payer.name))
        .collect();

    let template = ActivityPanelTemplate {
        activities: build_activities(expenses, user.id, &payer_names),
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),

        Err(error) => {
            eprintln!("Napaka pri izrisu aktivnosti: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Pri prikazu aktivnosti je prišlo do napake.",
            )
                .into_response()
        }
    }
}