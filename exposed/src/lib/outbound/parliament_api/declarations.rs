//! Deserialize and project the Interests API response.

use crate::domain::models::parliament_member::MemberId;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;

use super::{ApiError, ParliamentApiClient, decode_response};
use crate::domain::models::declaration_ingestion::CapturedDeclaration;
use crate::outbound::declaration_source::decode_declaration;

const INTERESTS_URL: &str = "https://interests-api.parliament.uk/api/v2/Interests";

impl ParliamentApiClient {
    pub(super) async fn capture_declarations(
        &self,
        member: MemberId,
    ) -> Result<Vec<CapturedDeclaration>, ApiError> {
        let mut declarations = Vec::new();
        let mut offset = 0;
        loop {
            let url = format!(
                "{INTERESTS_URL}?MemberId={}&Type=Commons&ExcludeExpired=false&ExpandChildInterests=false&Skip={offset}&Take={}",
                member.value(),
                self.batch_size,
            );
            let result = self.client.get(&url).send().await?;
            let status = result.status();
            let bytes = result.bytes().await?;
            let response: InterestPage = decode_response(&bytes, &url, status)?;
            if response.items.is_empty() {
                break;
            }
            let fetched_at = Utc::now();
            offset += response.items.len();
            for source in response.items {
                declarations.push(decode_declaration(source, fetched_at)?);
            }
            if offset >= response.total_results {
                break;
            }
        }
        Ok(declarations)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InterestPage {
    total_results: usize,
    // Retain every source field alongside the typed projection below.
    items: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use super::super::ParliamentResponse;
    use super::*;
    use crate::inbound::cli::{IntoResponse, Response};
    use reqwest::StatusCode;
    use serde::de::DeserializeOwned;

    fn assert_diagnostic<T: DeserializeOwned>(url: &str, body: &str, cause: &str) {
        let Err(error) = decode_response::<T>(body.as_bytes(), url, StatusCode::BAD_GATEWAY) else {
            panic!("invalid response was accepted");
        };
        let error: crate::domain::repositories::parliament_api::ParliamentApiError = error.into();
        let diagnostic = error.to_string();
        assert!(diagnostic.contains(cause), "{diagnostic}");
        assert!(diagnostic.contains("HTTP 502 Bad Gateway"), "{diagnostic}");
        assert!(
            diagnostic.contains(body),
            "response body missing from diagnostic: {diagnostic}"
        );
        assert!(
            diagnostic.contains(url),
            "request URL missing from diagnostic: {diagnostic}"
        );
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = Result::<Response, _>::Err(error)
            .into_response()
            .write_to(&mut stdout, &mut stderr)
            .unwrap();
        assert_eq!(exit, std::process::ExitCode::FAILURE);
        assert!(stdout.is_empty());
        let output: Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(output["error"]["message"].as_str().unwrap(), diagnostic);
    }

    #[test]
    fn members_missing_total_results_retains_full_response() {
        let body = format!(
            r#"{{"items":[],"message":"{}end of response"}}"#,
            "upstream detail ".repeat(1024)
        );
        assert_diagnostic::<ParliamentResponse>(
            "https://members-api.parliament.uk/api/Members/Search?skip=20&take=10",
            &body,
            "missing field `totalResults`",
        );
    }

    #[test]
    fn interests_missing_total_results_retains_full_response() {
        let body = format!(
            r#"{{"items":[],"message":"{}end of response"}}"#,
            "upstream detail ".repeat(1024)
        );
        assert_diagnostic::<InterestPage>(
            "https://interests-api.parliament.uk/api/v2/Interests?MemberId=42&Skip=20&Take=10",
            &body,
            "missing field `totalResults`",
        );
    }

    #[test]
    fn members_malformed_json_retains_response() {
        assert_diagnostic::<ParliamentResponse>(
            "https://members-api.parliament.uk/api/Members/Search?skip=0&take=10",
            "<html>\nupstream unavailable\n</html>",
            "expected value at line 1 column 1",
        );
    }

    #[test]
    fn interests_malformed_json_retains_response() {
        assert_diagnostic::<InterestPage>(
            "https://interests-api.parliament.uk/api/v2/Interests?MemberId=42&Skip=0&Take=10",
            "<html>\nupstream unavailable\n</html>",
            "expected value at line 1 column 1",
        );
    }

    #[test]
    fn valid_pages_still_decode() {
        let body = br#"{"items":[],"totalResults":0}"#;
        let members: ParliamentResponse = decode_response(body, "members", StatusCode::OK).unwrap();
        assert!(members.items.is_empty());
        assert_eq!(members.total_results, 0);
        let interests: InterestPage = decode_response(body, "interests", StatusCode::OK).unwrap();
        assert!(interests.items.is_empty());
        assert_eq!(interests.total_results, 0);
    }
}
