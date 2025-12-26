use reqwest::StatusCode;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]    
pub struct Error {
    pub code: StatusCode,
    pub response_body: Option<String>,
    pub assertion: String,

    pub transport_error: Option<reqwest::Error>,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Expected status code: {}, Response body: {:?}, Assertion: {}, Transport error: {:?}",
            self.code,
            self.response_body.as_ref().map_or("N/A".to_string(), |_| "Received".to_string()),
            self.assertion,
            self.transport_error,
        )
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error {
            code: err.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            response_body: None,
            assertion: "Transport error".to_string(),
            transport_error: Some(err),
        }
    }
}

pub struct ResponseChecker {
    pub status: StatusCode,
    pub headers: reqwest::header::HeaderMap,
    pub body: String,
}

impl ResponseChecker {
    pub async fn new(response: reqwest::Response) -> std::result::Result<Self, reqwest::Error> {
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await?;
        Ok(Self { status, headers, body })
    }
}
