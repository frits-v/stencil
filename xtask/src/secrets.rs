//! The secrets the review reads from its environment. They stay in this process's
//! environment; `child` starts every child without them.

use std::env;

use crate::anthropic_token::{REQUEST_TOKEN_VARIABLE, REQUEST_URL_VARIABLE};
use crate::app_token::REVIEW;

/// Every variable that holds a secret a review run may see: the App key, the Langfuse secret
/// key, the Actions OIDC request pair, which together mint tokens for the job, the Actions
/// runtime token and a job token.
pub const SECRET_VARIABLES: [&str; 6] = [
    REVIEW.key_variable,
    "LANGFUSE_SECRET_KEY",
    REQUEST_TOKEN_VARIABLE,
    REQUEST_URL_VARIABLE,
    "ACTIONS_RUNTIME_TOKEN",
    "GITHUB_TOKEN",
];

/// The value of a listed secret variable; `None` when it is unset or not listed.
pub fn read(name: &str) -> Option<String> {
    if !SECRET_VARIABLES.contains(&name) {
        return None;
    }
    env::var(name).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_secret_variable_the_review_reads_is_listed() {
        for name in [
            "FRITS_CODE_REVIEW_APP_PRIVATE_KEY",
            "LANGFUSE_SECRET_KEY",
            "ACTIONS_ID_TOKEN_REQUEST_TOKEN",
            "ACTIONS_ID_TOKEN_REQUEST_URL",
        ] {
            assert!(SECRET_VARIABLES.contains(&name), "{name}");
        }
    }

    #[test]
    fn a_variable_outside_the_list_is_never_read() {
        assert!(env::var_os("PATH").is_some());
        assert_eq!(read("PATH"), None);
    }
}
