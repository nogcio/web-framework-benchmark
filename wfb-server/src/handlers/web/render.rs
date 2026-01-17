use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use std::fmt;
use std::time::Instant;

pub struct HtmlTemplate<T>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct RenderDuration {
    start: Instant,
}

impl RenderDuration {
    pub fn new(start: Instant) -> Self {
        Self { start }
    }
}

impl fmt::Display for RenderDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ms = self.start.elapsed().as_secs_f64() * 1000.0;
        if ms >= 1.0 {
            write!(f, "{:.0}", ms.round())
        } else {
            write!(f, "{:.2}", ms)
        }
    }
}
