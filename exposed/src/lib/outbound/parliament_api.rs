//! Sequential Members API requests, typed decoding and bounded retries.
use crate::domain::{
    models::parliament_member::ParliamentMember,
    repositories::parliament_api::{
        HouseMembership, MemberHistory, ParliamentApi, ParliamentApiError,
    },
};
use chrono::{DateTime, NaiveDate, NaiveDateTime};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    fmt::Display,
    time::{Duration, SystemTime},
};

/// Production HTTP source for the Commons cohort.
#[derive(Clone)]
pub struct MembersApi {
    client: Client,
    base_url: Url,
}
impl MembersApi {
    /// Build an HTTP client without issuing requests.
    ///
    /// # Errors
    /// Rejects invalid URLs or HTTP client configuration.
    pub fn new(base_url: &str) -> Result<Self, ParliamentApiError> {
        let base_url = Url::parse(base_url).map_err(|_| failure("invalid Members API URL"))?;
        if !matches!(base_url.scheme(), "http" | "https")
            || base_url.host_str().is_none()
            || !base_url.username().is_empty()
            || base_url.password().is_some()
        {
            return Err(failure(
                "Members API URL must be an HTTP(S) URL without credentials",
            ));
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(failure)?;
        Ok(Self { client, base_url })
    }

    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T, ParliamentApiError> {
        let mut url = self.base_url.join(path).map_err(failure)?;
        url.query_pairs_mut()
            .extend_pairs(params.iter().map(|(key, value)| (*key, value.as_str())));
        // Request context contains source filters/IDs, never connection credentials.
        let context = format!("{path}?{}", url.query().unwrap_or_default());
        for attempt in 0..4 {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let mut delay = Duration::from_secs(1 << attempt);
            let problem = match self.client.get(url.clone()).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == StatusCode::OK {
                        match response.bytes().await {
                            Ok(bytes) => {
                                let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
                                let result = serde_path_to_error::deserialize(&mut deserializer)
                                    .map_err(|error| {
                                        let member = decoding_member_context(&bytes, error.path());
                                        failure(format!("{context}{member}: {error}"))
                                    })?;
                                deserializer
                                    .end()
                                    .map_err(|error| failure(format!("{context}: {error}")))?;
                                return Ok(result);
                            }
                            Err(error) => format!("response transport failure: {error}"),
                        }
                    } else {
                        if status != StatusCode::TOO_MANY_REQUESTS && !status.is_server_error() {
                            return Err(failure(format!("{context}: HTTP {status}")));
                        }
                        if let Some(header) = response
                            .headers()
                            .get(reqwest::header::RETRY_AFTER)
                            .and_then(|h| h.to_str().ok())
                        {
                            delay = retry_delay(header).unwrap_or(delay);
                        }
                        format!("HTTP {status}")
                    }
                }
                Err(error) => format!("transport failure: {error}"),
            };
            if attempt == 3 {
                return Err(failure(format!("{context}: {problem} after four attempts")));
            }
            eprintln!("{context}: {problem}; retry {}/4", attempt + 2);
            tokio::time::sleep(delay.min(Duration::from_secs(30))).await;
        }
        unreachable!("four attempts always return success or failure")
    }

    async fn search(
        &self,
        filters: Vec<(&str, String)>,
    ) -> Result<Vec<ParliamentMember>, ParliamentApiError> {
        let mut profiles = Vec::new();
        let mut offset = 0;
        loop {
            let mut params = filters.clone();
            params.extend([("skip", offset.to_string()), ("take", "20".to_owned())]);
            let page: SearchPage = self.get("/api/Members/Search", &params).await?;
            let context = format!("/api/Members/Search {params:?}");
            if page.skip != offset {
                return Err(failure(format!(
                    "{context}: response skip {} does not match request {offset}",
                    page.skip
                )));
            }
            if page.items.is_empty() && offset < page.total_results {
                return Err(failure(format!(
                    "{context}: premature empty page before total {}",
                    page.total_results
                )));
            }
            offset = offset
                .checked_add(page.items.len())
                .ok_or_else(|| failure("search pagination overflow"))?;
            for item in page.items {
                profiles.push(
                    ParliamentMember::new(
                        item.value.name_display_as,
                        item.value.id,
                        item.value.latest_party.id,
                        item.value.latest_party.name,
                        item.value.latest_house_membership.house,
                        item.value.latest_house_membership.membership_from,
                    )
                    .map_err(|error| failure(format!("{context}: {error}")))?,
                );
            }
            if offset >= page.total_results {
                return Ok(profiles);
            }
        }
    }
}

impl ParliamentApi for MembersApi {
    async fn current_commons(&self) -> Result<Vec<ParliamentMember>, ParliamentApiError> {
        self.search(vec![
            ("House", "1".into()),
            ("IsCurrentMember", "true".into()),
        ])
        .await
    }
    async fn commons_candidates(
        &self,
        term_start: NaiveDate,
        observation_date: NaiveDate,
    ) -> Result<Vec<ParliamentMember>, ParliamentApiError> {
        self.search(vec![
            (
                "MembershipInDateRange.WasMemberOnOrAfter",
                term_start.to_string(),
            ),
            (
                "MembershipInDateRange.WasMemberOnOrBefore",
                observation_date.to_string(),
            ),
            ("MembershipInDateRange.WasMemberOfHouse", "1".into()),
        ])
        .await
    }
    async fn member_histories(
        &self,
        member_ids: &[i32],
    ) -> Result<Vec<MemberHistory>, ParliamentApiError> {
        if member_ids.is_empty() || member_ids.len() > 100 {
            return Err(failure("history requests require 1–100 member IDs"));
        }
        let params: Vec<_> = member_ids
            .iter()
            .map(|id| ("ids", id.to_string()))
            .collect();
        let items: Vec<Item<History>> = self.get("/api/Members/History", &params).await?;
        items
            .into_iter()
            .map(|item| {
                let id = item.value.id;
                let periods = item
                    .value
                    .house_membership_history
                    .into_iter()
                    .map(|period| {
                        HouseMembership::new(
                            period.house,
                            period.start_date.0,
                            period.end_date.map(|date| date.0),
                        )
                        .map_err(|error| {
                            failure(format!("/api/Members/History, member {id}: {error}"))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                MemberHistory::new(id, periods)
                    .map_err(|error| failure(format!("/api/Members/History: {error}")))
            })
            .collect()
    }
}

fn failure(message: impl Display) -> ParliamentApiError {
    ParliamentApiError(message.to_string())
}

// Decode raw JSON only on failure to recover the affected member's identity.
// The normal response path deserializes directly into typed API values.
fn decoding_member_context(bytes: &[u8], path: &serde_path_to_error::Path) -> String {
    let Some(index) = path.iter().find_map(|segment| match segment {
        serde_path_to_error::Segment::Seq { index } => Some(*index),
        _ => None,
    }) else {
        return String::new();
    };
    let Ok(response) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return String::new();
    };
    // Search wraps items in an object; History returns the item array directly.
    response.get("items").unwrap_or(&response)[index]["value"]["id"]
        .as_i64()
        .map_or_else(String::new, |id| format!(", member {id}"))
}

fn retry_delay(header: &str) -> Option<Duration> {
    if let Ok(seconds) = header.parse::<f64>() {
        return seconds
            .is_finite()
            .then(|| Duration::from_secs_f64(seconds.clamp(0.0, 30.0)));
    }
    httpdate::parse_http_date(header).ok().map(|date| {
        date.duration_since(SystemTime::now())
            .unwrap_or_default()
            .min(Duration::from_secs(30))
    })
}

#[derive(Deserialize)]
struct Item<T> {
    value: T,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchPage {
    items: Vec<Item<Profile>>,
    total_results: usize,
    skip: usize,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    id: i32,
    name_display_as: String,
    latest_party: Party,
    latest_house_membership: LatestMembership,
}
#[derive(Deserialize)]
struct Party {
    id: i32,
    name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LatestMembership {
    house: i16,
    membership_from: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct History {
    id: i32,
    house_membership_history: Vec<Membership>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Membership {
    house: i16,
    #[serde(rename = "membershipStartDate")]
    start_date: SourceDate,
    #[serde(rename = "membershipEndDate")]
    end_date: Option<SourceDate>,
}

struct SourceDate(NaiveDate);
impl<'de> Deserialize<'de> for SourceDate {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        let date = NaiveDate::parse_from_str(&value, "%Y-%m-%d")
            .or_else(|_| DateTime::parse_from_rfc3339(&value).map(|d| d.date_naive()))
            .or_else(|_| {
                NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S%.f").map(|d| d.date())
            })
            .or_else(|_| {
                NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S%.f").map(|d| d.date())
            })
            .map_err(serde::de::Error::custom)?;
        Ok(Self(date))
    }
}
