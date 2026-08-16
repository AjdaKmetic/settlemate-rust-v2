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
    auth::{
        login_form,
        login_user,
        logout_user,
        register_form,
        register_user,
    },
    friends::{
        add_friend_handler,
        friend_form,
    },
    index::index,
    tabs::{
        activity_tab,
        friends_tab,
        groups_tab,
    },
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
        .route("/account", get(account_page))
        .route("/account/name", post(update_name))
        .route("/account/password", post(change_password))
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
