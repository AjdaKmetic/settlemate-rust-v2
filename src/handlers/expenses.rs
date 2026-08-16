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
    entities::users,
    handlers::auth::get_current_user,
    services::{
        expense_service::{
            create_equal_expense,
            get_expenses_for_user
        },
        friend_service::get_friends,
    },
};

#[derive(Template)]
#[template(path = "partials/expense_form.html")]
struct ExpenseFormTemplate {
    friends: Vec<users::Model>,
}

struct ActivityItem {
    description: String,
    formatted_amount: String,
    paid_by_current_user: bool,
}

#[derive(Template)]
#[template(path = "partials/activity.html")]
struct ActivityTemplate {
    activities: Vec<ActivityItem>,
}

fn format_amount(amount_cents: i64) -> String {
    let euros = amount_cents / 100;
    let cents = amount_cents % 100;

    format!("{euros},{cents:02} €")
}

// podatki iz html- obrazca se pretvorijo v strukturo CreateExpenseForm
#[derive(Deserialize)]
pub struct CreateExpenseForm { 
    description: String,
    amount: f64,
    friend_id: i32,
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


pub async fn activity_tab(
    State(state): State<AppState>,
    jar: CookieJar,
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
                "Pri prikazu aktivnosti je prišlo do napake.",
            )
                .into_response();
        }
    };

    let expenses = match get_expenses_for_user(&state.db, user.id).await { // pridobivanje stroškov
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

    let activities: Vec<ActivityItem> = expenses
        .into_iter()
        .map(|expense| ActivityItem { // za usak activity se ustvari activity item
            description: expense.description,
            formatted_amount: format_amount(expense.amount_cents),
            paid_by_current_user: expense.paid_by == user.id,
        })
        .collect(); // vse elemente se zdruzi

    let template = ActivityTemplate { activities };

    match template.render() { // izris predloge
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