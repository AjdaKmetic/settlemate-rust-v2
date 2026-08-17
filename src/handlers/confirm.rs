use askama::Template;

#[derive(Template)]
#[template(path = "partials/confirm_modal.html")]
pub struct ConfirmModalTemplate {
    pub title: String,
    pub message: String,
    pub cancel_label: String,
    pub confirm_label: String,
    pub confirm_url: String,
    pub confirm_target: String,
}
