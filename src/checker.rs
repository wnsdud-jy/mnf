use std::{collections::HashSet, time::Duration};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use tokio::time::sleep;

const BATCH_ENDPOINT: &str = "https://api.mojang.com/profiles/minecraft";
const PROFILE_LOOKUP_ENDPOINT: &str = "https://api.mojang.com/users/profiles/minecraft";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RETRIES: u8 = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchCheckOutcome {
    pub taken_names: Vec<String>,
    pub likely_available_names: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct MojangProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub legacy: bool,
}

#[async_trait]
pub trait NameChecker: Send + Sync {
    async fn check_batch(&self, batch: &[String]) -> Result<BatchCheckOutcome>;
}

#[derive(Clone, Debug)]
pub struct MojangChecker {
    client: Client,
}

impl MojangChecker {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("mnf/0.1")
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self { client })
    }

    async fn check_batch_request(&self, batch: &[String]) -> Result<BatchCheckOutcome> {
        for attempt in 0..=MAX_RETRIES {
            let response = self.client.post(BATCH_ENDPOINT).json(batch).send().await;

            match response {
                Ok(response) if response.status().is_success() => {
                    let profiles = response
                        .json::<Vec<MojangProfile>>()
                        .await
                        .context("failed to parse Mojang batch response")?;
                    return Ok(classify_batch(batch, &profiles));
                }
                Ok(response) if response.status() == StatusCode::FORBIDDEN => {
                    if attempt == MAX_RETRIES {
                        return self.check_names_individually(batch).await;
                    }
                    sleep(backoff_for(attempt, None)).await;
                }
                Ok(response)
                    if response.status() == StatusCode::TOO_MANY_REQUESTS
                        || response.status().is_server_error() =>
                {
                    if attempt == MAX_RETRIES {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        bail!("Mojang batch lookup failed with {status}: {body}");
                    }
                    sleep(backoff_for(attempt, retry_after(&response))).await;
                }
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    bail!("Mojang batch lookup failed with {status}: {body}");
                }
                Err(error) => {
                    if attempt == MAX_RETRIES {
                        return Err(error).context("Mojang batch lookup request failed");
                    }
                    sleep(backoff_for(attempt, None)).await;
                }
            }
        }

        unreachable!("retry loop always returns or errors");
    }

    async fn check_names_individually(&self, batch: &[String]) -> Result<BatchCheckOutcome> {
        let mut taken_names = Vec::new();
        let mut likely_available_names = Vec::new();

        for name in batch {
            if self.is_name_taken(name).await? {
                taken_names.push(name.clone());
            } else {
                likely_available_names.push(name.clone());
            }
        }

        Ok(BatchCheckOutcome {
            taken_names,
            likely_available_names,
        })
    }

    async fn is_name_taken(&self, name: &str) -> Result<bool> {
        let url = format!("{PROFILE_LOOKUP_ENDPOINT}/{name}");

        for attempt in 0..=MAX_RETRIES {
            let response = self.client.get(&url).send().await;

            match response {
                Ok(response) if response.status() == StatusCode::OK => return Ok(true),
                Ok(response) if response.status() == StatusCode::NO_CONTENT => return Ok(false),
                Ok(response)
                    if response.status() == StatusCode::TOO_MANY_REQUESTS
                        || response.status().is_server_error()
                        || response.status() == StatusCode::FORBIDDEN =>
                {
                    if attempt == MAX_RETRIES {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        bail!("name lookup failed for {name} with {status}: {body}");
                    }
                    sleep(backoff_for(attempt, retry_after(&response))).await;
                }
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    bail!("name lookup failed for {name} with {status}: {body}");
                }
                Err(error) => {
                    if attempt == MAX_RETRIES {
                        return Err(error)
                            .with_context(|| format!("name lookup request failed for {name}"));
                    }
                    sleep(backoff_for(attempt, None)).await;
                }
            }
        }

        unreachable!("retry loop always returns or errors");
    }
}

#[async_trait]
impl NameChecker for MojangChecker {
    async fn check_batch(&self, batch: &[String]) -> Result<BatchCheckOutcome> {
        if batch.is_empty() {
            return Ok(BatchCheckOutcome {
                taken_names: Vec::new(),
                likely_available_names: Vec::new(),
            });
        }

        if batch.len() > 10 {
            bail!("batch size must be 10 or less");
        }

        self.check_batch_request(batch).await
    }
}

fn backoff_for(attempt: u8, retry_after: Option<Duration>) -> Duration {
    retry_after.unwrap_or_else(|| Duration::from_secs(1_u64 << attempt.min(5)))
}

fn retry_after(response: &reqwest::Response) -> Option<Duration> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
}

pub fn classify_batch(batch: &[String], profiles: &[MojangProfile]) -> BatchCheckOutcome {
    let taken_lookup: HashSet<String> = profiles
        .iter()
        .map(|profile| profile.name.to_lowercase())
        .collect();
    let mut taken_names = Vec::new();
    let mut likely_available_names = Vec::new();

    for name in batch {
        if taken_lookup.contains(&name.to_lowercase()) {
            taken_names.push(name.clone());
        } else {
            likely_available_names.push(name.clone());
        }
    }

    BatchCheckOutcome {
        taken_names,
        likely_available_names,
    }
}

#[cfg(test)]
mod tests {
    use super::{BatchCheckOutcome, MojangProfile, classify_batch};

    #[test]
    fn classifies_likely_available_names() {
        let batch = vec!["eaaa".to_string(), "eaab".to_string(), "eaac".to_string()];
        let profiles = vec![MojangProfile {
            id: "123".to_string(),
            name: "eaab".to_string(),
            legacy: false,
        }];

        let outcome = classify_batch(&batch, &profiles);
        assert_eq!(
            outcome,
            BatchCheckOutcome {
                taken_names: vec!["eaab".to_string()],
                likely_available_names: vec!["eaaa".to_string(), "eaac".to_string()],
            }
        );
    }
}
