use crate::testcase_spec::*;
use crate::{assert_body_eq, assert_status, assert_header};

pub async fn run_test_case(
    base_url: &str
) -> Result<()> {
    let response = reqwest::get(format!("{}/plain_text", base_url))
        .await?;
    
    let checker = ResponseChecker::new(response).await?;
    assert_status!(checker, reqwest::StatusCode::OK);
    assert_header!(checker, "content-type", "text/plain");
    assert_body_eq!(checker, "Hello, World!", "Response body does not match expected plain text");

    Ok(())
}