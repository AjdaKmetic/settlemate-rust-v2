mod app;
mod entities;
mod handlers;
mod services;

use axum::{
    Router,
    routing::{get, post},
};
use sea_orm::Database;

use app::state::AppState;
use handlers::{
    account::{account_page, change_password, update_name},
    auth::{login_form, login_user, logout_user, register_form, register_user},
    expenses::{
        close_expense_form, create_expense_handler, delete_expense_handler, expense_delete_form,
        expense_description_form, expense_detail, expense_form, expense_group_form,
        update_expense_description_handler,
    },
    friends::{
        add_friend_handler, friend_delete_form, friend_detail, friend_form, remove_friend_handler,
    },
    groups::{
        add_group_member_handler, 
        create_group_handler, 
        delete_group_handler, 
        group_delete_form,
        group_detail, 
        group_form, 
        group_leave_form, 
        group_member_form, 
        leave_group_handler,
        settle_group_debt_handler,
    },
    index::index,
    payments::settle_debt_handler,
    tabs::{activity_tab, friends_tab, groups_tab},
};

use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let db = Database::connect("sqlite://settlemate.db?mode=rwc")
        .await
        .expect("Povezava z bazo ni uspela.");

    let state = AppState { db };

    let app = Router::new()
        .route("/", get(index))
        .route("/register", get(register_form).post(register_user))
        .route("/login", get(login_form).post(login_user))
        .route("/logout", post(logout_user))
        .route("/tabs/friends", get(friends_tab))
        .route("/tabs/groups", get(groups_tab))
        .route("/tabs/activity", get(activity_tab))
        .route("/friends/form", get(friend_form))
        .route("/friends", post(add_friend_handler))
        .route("/friends/{friend_id}", get(friend_detail))
        .route("/friends/{friend_id}/delete/form", get(friend_delete_form))
        .route("/friends/{friend_id}/delete", post(remove_friend_handler))
        .route("/groups/form", get(group_form))
        .route("/groups", post(create_group_handler))
        .route("/groups/{id}", get(group_detail))
        .route("/groups/{id}/members/form", get(group_member_form))
        .route("/groups/{id}/members", post(add_group_member_handler))
        .route("/groups/{id}/leave/form", get(group_leave_form))
        .route("/groups/{id}/leave", post(leave_group_handler))
        .route("/groups/{id}/delete/form", get(group_delete_form))
        .route("/groups/{id}/delete", post(delete_group_handler))
        .route("/groups/{group_id}/members/{member_id}/settle", post(settle_group_debt_handler))
        .route("/account", get(account_page))
        .route("/account/name", post(update_name))
        .route("/account/password", post(change_password))
        .route("/expenses/form", get(expense_form))
        .route("/expenses/close", get(close_expense_form))
        .route("/expenses", post(create_expense_handler))
        .route(
            "/payments/settle/{other_user_id}",
            post(settle_debt_handler),
        )
        .route("/expenses/{id}", get(expense_detail))
        .route(
            "/expenses/{id}/description/form",
            get(expense_description_form),
        )
        .route(
            "/expenses/{id}/description",
            post(update_expense_description_handler),
        )
        .route("/expenses/{id}/delete/form", get(expense_delete_form))
        .route("/expenses/{id}/delete", post(delete_expense_handler))
        .route("/expenses/form/group", get(expense_group_form))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Strežnika ni bilo mogoče zagnati.");

    println!("SettleMate aplikacija je dostopna na http://127.0.0.1:3000");

    axum::serve(listener, app)
        .await
        .expect("Napaka v delovanju strežnika.");
}
