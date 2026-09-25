use std::{future::Future, path::PathBuf, process::Stdio, time::Duration};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::Mutex,
};

use super::declarations::{interpret, source_date};
use crate::imports::core::{
    ImportError, Result,
    declarations::{Draft, Evidence},
    members::{HouseMembership, MemberHistory, MemberProfile},
    ports::{DeclarationSource, MemberSource, Page},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case")]
pub(crate) enum Request {
    CurrentMembers {
        offset: usize,
    },
    MemberCandidates {
        term_start: NaiveDate,
        as_of: NaiveDate,
        offset: usize,
    },
    MemberHistories {
        ids: Vec<i32>,
    },
    Declarations {
        member: i32,
        offset: usize,
    },
    Parent {
        member: i32,
        parent: i32,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Response {
    pub payload: Value,
    pub fetched_at: DateTime<Utc>,
    pub context: String,
}

pub(crate) trait Transport: Send + Sync {
    fn fetch(&self, request: Request) -> impl Future<Output = Result<Response>> + Send;
}

pub(crate) struct Parliament<T>(pub T);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope {
    items: Vec<Value>,
    total_results: usize,
    #[serde(rename = "skip")]
    _skip: usize,
}

fn envelope(
    response: &Response,
    offset: usize,
    members: bool,
) -> Result<(Vec<Value>, Option<usize>)> {
    let page: Envelope = serde_json::from_value(response.payload.clone())
        .map_err(|e| ImportError::Invalid(format!("Invalid Parliament page: {e}")))?;
    if members && page.items.is_empty() && offset < page.total_results {
        return Err(ImportError::Invalid(
            "Search ended before all advertised members were returned".into(),
        ));
    }
    let next = offset
        .checked_add(page.items.len())
        .ok_or_else(|| ImportError::Invalid("Invalid page size".into()))?;
    let more = !page.items.is_empty() && next < page.total_results;
    Ok((page.items, more.then_some(next)))
}

#[derive(Deserialize)]
struct Party {
    id: Option<i32>,
    name: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Membership {
    house: i16,
    membership_from: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    id: i32,
    name_display_as: String,
    latest_party: Option<Party>,
    latest_house_membership: Membership,
}

fn profile(raw: Value) -> Result<MemberProfile> {
    let p: Profile = serde_json::from_value(raw["value"].clone())
        .map_err(|e| ImportError::Invalid(format!("Invalid member: {e}")))?;
    let (party_id, party_name) = p.latest_party.map_or((None, None), |p| (p.id, p.name));
    let member = MemberProfile {
        id: p.id,
        name: p.name_display_as,
        party_id,
        party_name,
        house: p.latest_house_membership.house,
        membership_from: p.latest_house_membership.membership_from,
    };
    member.validate()?;
    Ok(member)
}

impl<T: Transport> MemberSource for Parliament<T> {
    async fn profiles(
        &self,
        term: NaiveDate,
        as_of: NaiveDate,
        current: bool,
        offset: usize,
    ) -> Result<Page<MemberProfile>> {
        let request = if current {
            Request::CurrentMembers { offset }
        } else {
            Request::MemberCandidates {
                term_start: term,
                as_of,
                offset,
            }
        };
        let response = self.0.fetch(request).await?;
        let (items, next) = envelope(&response, offset, true)?;
        Ok(Page {
            entries: items.into_iter().map(profile).collect::<Result<_>>()?,
            next,
        })
    }
    async fn histories(&self, ids: &[i32]) -> Result<Vec<MemberHistory>> {
        if ids.is_empty() || ids.len() > 100 {
            return Err(ImportError::Invalid(
                "History requests need between 1 and 100 IDs".into(),
            ));
        }
        let response = self
            .0
            .fetch(Request::MemberHistories { ids: ids.to_vec() })
            .await?;
        let items = response
            .payload
            .as_array()
            .ok_or_else(|| ImportError::Invalid("Invalid history envelope".into()))?;
        items
            .iter()
            .map(|item| {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct History {
                    id: i32,
                    house_membership_history: Vec<Value>,
                }
                let history: History = serde_json::from_value(item["value"].clone())
                    .map_err(|e| ImportError::Invalid(format!("Invalid history: {e}")))?;
                let memberships = history
                    .house_membership_history
                    .iter()
                    .map(|m| {
                        let house = m["house"]
                            .as_i64()
                            .and_then(|h| i16::try_from(h).ok())
                            .filter(|h| [1, 2].contains(h))
                            .ok_or_else(|| ImportError::Invalid("Invalid history House".into()))?;
                        let start = source_date(&m["membershipStartDate"], "membershipStartDate")
                            .map_err(|e| ImportError::Invalid(e.to_string()))?;
                        let end = if m["membershipEndDate"].is_null() {
                            None
                        } else {
                            Some(
                                source_date(&m["membershipEndDate"], "membershipEndDate")
                                    .map_err(|e| ImportError::Invalid(e.to_string()))?,
                            )
                        };
                        Ok(HouseMembership { house, start, end })
                    })
                    .collect::<Result<Vec<_>>>()?;
                if history.id <= 0 || memberships.is_empty() {
                    return Err(ImportError::Invalid("Invalid member history".into()));
                }
                Ok(MemberHistory {
                    id: history.id,
                    memberships,
                })
            })
            .collect()
    }
}

fn evidence(items: Vec<Value>, response: &Response) -> Vec<Evidence> {
    items
        .into_iter()
        .map(|payload| Evidence {
            source_id: payload
                .get("id")
                .and_then(Value::as_i64)
                .and_then(|n| i32::try_from(n).ok()),
            payload,
            fetched_at: response.fetched_at,
            context: response.context.clone(),
        })
        .collect()
}

impl<T: Transport> DeclarationSource for Parliament<T> {
    async fn declarations(&self, member: i32, offset: usize) -> Result<Page<Evidence>> {
        let response = self
            .0
            .fetch(Request::Declarations { member, offset })
            .await?;
        let (items, next) = envelope(&response, offset, false)?;
        Ok(Page {
            entries: evidence(items, &response),
            next,
        })
    }
    async fn parents(&self, member: i32, parent: i32) -> Result<Vec<Evidence>> {
        let response = self.0.fetch(Request::Parent { member, parent }).await?;
        let (items, _) = envelope(&response, 0, false)?;
        Ok(evidence(items, &response))
    }
    fn interpret(&self, evidence: &Evidence) -> Result<Draft> {
        interpret(&evidence.payload)
    }
}

struct Worker {
    _child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
pub(crate) struct CliTransport {
    python: PathBuf,
    directory: PathBuf,
    worker: Mutex<Option<Worker>>,
}

impl CliTransport {
    pub fn new(python: PathBuf, directory: PathBuf) -> Self {
        Self {
            python,
            directory,
            worker: Mutex::new(None),
        }
    }
    fn start(&self) -> Result<Worker> {
        let python = self
            .python
            .canonicalize()
            .map_err(|_| ImportError::Source("Python executable is unavailable".into()))?;
        let mut command = Command::new(python);
        command
            .args(["-m", "exposed", "source"])
            .current_dir(&self.directory)
            .env_clear()
            .env("PYTHONPATH", "src")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        for key in [
            "PATH",
            "SSL_CERT_FILE",
            "HTTPS_PROXY",
            "HTTP_PROXY",
            "NO_PROXY",
        ] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        let mut child = command
            .spawn()
            .map_err(|_| ImportError::Source("Could not start Python API worker".into()))?;
        let input = child
            .stdin
            .take()
            .ok_or_else(|| ImportError::Source("Worker stdin unavailable".into()))?;
        let output = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| ImportError::Source("Worker stdout unavailable".into()))?,
        );
        Ok(Worker {
            _child: child,
            input,
            output,
        })
    }
}

impl Transport for CliTransport {
    async fn fetch(&self, request: Request) -> Result<Response> {
        tokio::time::timeout(Duration::from_secs(240), async {
            let mut slot = self.worker.lock().await;
            if slot.is_none() {
                *slot = Some(self.start()?);
            }
            let worker = slot
                .as_mut()
                .ok_or_else(|| ImportError::Source("Worker did not start".into()))?;
            let mut data =
                serde_json::to_vec(&request).map_err(|e| ImportError::Source(e.to_string()))?;
            data.push(b'\n');
            worker
                .input
                .write_all(&data)
                .await
                .map_err(|_| ImportError::Source("Worker input closed".into()))?;
            worker
                .input
                .flush()
                .await
                .map_err(|_| ImportError::Source("Worker input closed".into()))?;
            let mut line = String::new();
            worker
                .output
                .read_line(&mut line)
                .await
                .map_err(|_| ImportError::Source("Worker output closed".into()))?;
            let value: Value = serde_json::from_str(&line)
                .map_err(|_| ImportError::Source("Invalid worker response".into()))?;
            if let Some(error) = value.get("error").and_then(Value::as_str) {
                return Err(ImportError::Source(error.to_owned()));
            }
            serde_json::from_value(value)
                .map_err(|e| ImportError::Source(format!("Invalid evidence envelope: {e}")))
        })
        .await
        .map_err(|_| ImportError::Source("Python API worker timed out".into()))?
    }
}
