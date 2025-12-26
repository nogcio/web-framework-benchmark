#[macro_export]
macro_rules! assert_status {
    ($checker:expr, $expected:expr) => {
        if $checker.status != $expected {
            return Err($crate::testcase_spec::error::Error {
                code: $checker.status,
                response_body: Some($checker.body.clone()),
                assertion: format!("Expected status code {}, got {}", $expected, $checker.status),
                transport_error: None,
            });
        }
    };
}

#[macro_export]
macro_rules! assert_body_eq {
    ($checker:expr, $expected:expr) => {
        if $checker.body != $expected {
            return Err($crate::testcase_spec::error::Error {
                code: $checker.status,
                response_body: Some($checker.body.clone()),
                assertion: format!("Expected body {:?}, got {:?}", $expected, $checker.body),
                transport_error: None,
            });
        }
    };
    ($checker:expr, $expected:expr, $msg:expr) => {
        if $checker.body != $expected {
            return Err($crate::testcase_spec::error::Error {
                code: $checker.status,
                response_body: Some($checker.body.clone()),
                assertion: $msg.to_string(),
                transport_error: None,
            });
        }
    };
}

#[macro_export]
macro_rules! assert_header {
    ($checker:expr, $key:expr, $expected:expr) => {
        let key = $key;
        let expected = $expected;
        match $checker.headers.get(key) {
            Some(value) => {
                if value != expected {
                    return Err($crate::testcase_spec::error::Error {
                        code: $checker.status,
                        response_body: Some($checker.body.clone()),
                        assertion: format!("Expected header {} to be {:?}, got {:?}", key, expected, value),
                        transport_error: None,
                    });
                }
            }
            None => {
                return Err($crate::testcase_spec::error::Error {
                    code: $checker.status,
                    response_body: Some($checker.body.clone()),
                    assertion: format!("Expected header {} to be present", key),
                    transport_error: None,
                });
            }
        }
    };
}
