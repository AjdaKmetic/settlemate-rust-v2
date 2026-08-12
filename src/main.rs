mod app;
mod entities;
mod handlers;
mod services;

use axum::{
    routing::{get, post},
    Router,
};
use sea_orm::Database;

use app::state::AppState;
use handlers::auth::{
    login_form,
    login_user,
    logout_user,
    register_form,
    register_user,
};

use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let db = Database::connect("sqlite://settlemate.db?mode=rwc")
        .await
        .expect("Povezava z bazo ni uspela.");

    let state = AppState { db };

    let app = Router::new()
        .route("/register", get(register_form).post(register_user))
        .route("/login", get(login_form).post(login_user))
        .route("/logout", post(logout_user))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Strežnika ni bilo mogoče zagnati.");

    println!("SettleMate teče na http://127.0.0.1:3000");

    axum::serve(listener, app)
        .await
        .expect("Napaka v delovanju strežnika.");
}
