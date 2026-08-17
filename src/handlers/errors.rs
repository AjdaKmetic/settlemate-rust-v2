// skupni pomočniki za obravnavo napak v handlerjih

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;

// enotno ravnanje pri napaki 500 (zabeleži tehnično napako, uporabniku vrne prijazno sporočilo)
pub fn internal_error(
    context: &str,
    error: impl std::fmt::Display,
    message: &'static str,
) -> Response {
    eprintln!("{context}: {error}");

    (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
}

// izlušči uporabniku namenjeno sporočilo iz DbErr: Custom nosi prijazno besedilo,
// ostale napake pa zabeležimo in vrnemo nadomestno sporočilo
pub fn db_error_message(context: &str, error: DbErr, fallback: &str) -> String {
    match error {
        DbErr::Custom(message) => message,

        other => {
            eprintln!("{context}: {other}");

            fallback.to_string()
        }
    }
}
