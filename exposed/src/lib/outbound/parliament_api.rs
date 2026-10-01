//! Sequential Members API requests, typed decoding and bounded retries.
use crate::domain::{
    models::member_ingestion::{HouseMembership, MemberHistory, MemberProfile},
    repositories::parliament_api::{ParliamentApi, ParliamentApiError},
};
use chrono::{DateTime, NaiveDate, NaiveDateTime};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
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

    async fn get(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<Vec<u8>, ParliamentApiError> {
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
                            Ok(bytes) => return Ok(bytes.to_vec()),
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
    ) -> Result<Vec<MemberProfile>, ParliamentApiError> {
        let mut profiles = Vec::new();
        let mut offset = 0;
        loop {
            let mut params = filters.clone();
            params.extend([("skip", offset.to_string()), ("take", "20".to_owned())]);
            let bytes = self.get("/api/Members/Search", &params).await?;
            let context = format!("/api/Members/Search {params:?}");
            let page: SearchPage = decode(&bytes, &context)?;
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
            for value in page.items {
                let item: Item<Profile> = decode_item(value, &context)?;
                profiles.push(MemberProfile {
                    parliament_member_id: item.value.id,
                    name: item.value.name_display_as,
                    party_id: item.value.latest_party.as_ref().and_then(|p| p.id),
                    party_name: item.value.latest_party.and_then(|p| p.name),
                    latest_house: item.value.latest_house_membership.house,
                    latest_membership_from: item.value.latest_house_membership.membership_from,
                });
            }
            if offset >= page.total_results {
                return Ok(profiles);
            }
        }
    }
}

impl ParliamentApi for MembersApi {
    async fn current_commons(&self) -> Result<Vec<MemberProfile>, ParliamentApiError> {
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
    ) -> Result<Vec<MemberProfile>, ParliamentApiError> {
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
        let bytes = self.get("/api/Members/History", &params).await?;
        let context = format!("/api/Members/History {member_ids:?}");
        let values: Vec<Value> = decode(&bytes, &context)?;
        values
            .into_iter()
            .map(|value| {
                let item: Item<History> = decode_item(value, &context)?;
                Ok(MemberHistory {
                    parliament_member_id: item.value.id,
                    house_memberships: item
                        .value
                        .house_membership_history
                        .into_iter()
                        .map(|period| HouseMembership {
                            house: period.house,
                            start_date: period.start_date.0,
                            end_date: period.end_date.map(|d| d.0),
                        })
                        .collect(),
                })
            })
            .collect()
    }
}

fn failure(message: impl Display) -> ParliamentApiError {
    ParliamentApiError(message.to_string())
}
fn decode<T: DeserializeOwned>(bytes: &[u8], context: &str) -> Result<T, ParliamentApiError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let result = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|error| failure(format!("{context}: {error}")))?;
    deserializer
        .end()
        .map_err(|error| failure(format!("{context}: {error}")))?;
    Ok(result)
}
fn decode_item<T: DeserializeOwned>(value: Value, context: &str) -> Result<T, ParliamentApiError> {
    let id = value
        .pointer("/value/id")
        .map_or_else(|| "unknown".into(), Value::to_string);
    serde_path_to_error::deserialize(value)
        .map_err(|error| failure(format!("{context}, member {id}: {error}")))
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
    items: Vec<Value>,
    total_results: usize,
    skip: usize,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    id: i32,
    name_display_as: String,
    latest_party: Option<Party>,
    latest_house_membership: LatestMembership,
}
#[derive(Deserialize)]
struct Party {
    id: Option<i32>,
    name: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LatestMembership {
    house: i16,
    membership_from: Option<String>,
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
