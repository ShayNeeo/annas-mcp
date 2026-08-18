use std::collections::HashMap;
use std::time::Duration;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::config::{DEFAULT_FALLBACK_MIRROR, DEFAULT_SLUM_STATUS_URL, DEFAULT_USER_AGENT};
use crate::errors::{AppError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub status: i32,
    #[serde(default)]
    pub ping: i64,
    #[serde(default)]
    pub time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub monitor_id: i32,
    pub base_url: String,
    pub source_url: String,
    #[serde(default)]
    pub heartbeats: Vec<Heartbeat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateScore {
    pub success_count: usize,
    pub sample_count: usize,
    pub last_status: i32,
    pub average_ping: i64,
    pub success_rate: f64,
}

impl Candidate {
    pub fn score(&self) -> CandidateScore {
        let mut success_count = 0;
        let mut ping_sum: i64 = 0;
        let mut last_status = -1;

        for (idx, hb) in self.heartbeats.iter().enumerate() {
            if idx == self.heartbeats.len().saturating_sub(1) {
                last_status = hb.status;
            }
            if hb.status == 1 {
                success_count += 1;
                ping_sum += hb.ping;
            }
        }

        let sample_count = self.heartbeats.len();
        let average_ping = if success_count > 0 {
            ping_sum / success_count as i64
        } else {
            0
        };

        let success_rate = if sample_count > 0 {
            success_count as f64 / sample_count as f64
        } else {
            0.0
        };

        CandidateScore {
            success_count,
            sample_count,
            last_status,
            average_ping,
            success_rate,
        }
    }
}

pub struct MirrorResolver {
    client: Client,
    status_page_url: String,
}

#[derive(Deserialize)]
struct HeartbeatEnvelope {
    #[serde(rename = "heartbeatList")]
    heartbeat_list: HashMap<String, Vec<Heartbeat>>,
}

impl MirrorResolver {
    pub fn new(client: Option<Client>, status_page_url: Option<String>) -> Self {
        let client = client.unwrap_or_else(|| {
            Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent(DEFAULT_USER_AGENT)
                .build()
                .unwrap_or_default()
        });

        Self {
            client,
            status_page_url: status_page_url.unwrap_or_else(|| DEFAULT_SLUM_STATUS_URL.to_string()),
        }
    }

    pub fn normalize_base_url(raw: &str) -> String {
        let mut value = raw.trim().to_string();
        if value.ends_with('/') {
            value.pop();
        }
        if let Some(stripped) = value.strip_prefix("https://") {
            value = stripped.to_string();
        } else if let Some(stripped) = value.strip_prefix("http://") {
            value = stripped.to_string();
        }
        value
    }

    pub async fn resolve(&self, fallback: Option<&str>) -> Result<String> {
        let fallback_url = fallback
            .map(Self::normalize_base_url)
            .unwrap_or_else(|| DEFAULT_FALLBACK_MIRROR.to_string());

        let candidates = match self.fetch_and_rank_candidates().await {
            Ok(c) if !c.is_empty() => c,
            Ok(_) => {
                warn!("No mirror candidates found from SLUM, using fallback: {}", fallback_url);
                return Ok(fallback_url);
            }
            Err(e) => {
                warn!("Failed to query SLUM mirror status ({}): using fallback {}", e, fallback_url);
                return Ok(fallback_url);
            }
        };

        for candidate in candidates {
            if self.probe(&candidate.base_url).await {
                info!("Selected healthy Anna's Archive mirror: https://{}", candidate.base_url);
                return Ok(candidate.base_url);
            } else {
                warn!("Mirror probe failed for https://{}, trying next candidate", candidate.base_url);
            }
        }

        warn!("All discovered mirror probes failed, falling back to {}", fallback_url);
        Ok(fallback_url)
    }

    pub async fn fetch_and_rank_candidates(&self) -> Result<Vec<Candidate>> {
        let resp = self
            .client
            .get(&self.status_page_url)
            .send()
            .await
            .map_err(|e| AppError::MirrorResolution(format!("Failed to fetch SLUM status page: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::MirrorResolution(format!(
                "SLUM status page returned HTTP {}",
                resp.status()
            )));
        }

        let html = resp
            .text()
            .await
            .map_err(|e| AppError::MirrorResolution(format!("Failed to read SLUM HTML: {e}")))?;

        let candidates = Self::parse_status_page_html(&html)?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let slug = Self::extract_slug(&html);
        let heartbeat_url = format!(
            "{}/api/status-page/heartbeat/{}",
            self.status_page_url.trim_end_matches('/'),
            slug
        );

        let heartbeats = match self.client.get(&heartbeat_url).send().await {
            Ok(r) if r.status().is_success() => {
                let envelope: Option<HeartbeatEnvelope> = r.json().await.ok();
                envelope.map(|e| e.heartbeat_list).unwrap_or_default()
            }
            _ => HashMap::new(),
        };

        let mut enriched = Vec::with_capacity(candidates.len());
        for mut c in candidates {
            if c.monitor_id > 0 {
                if let Some(hbs) = heartbeats.get(&c.monitor_id.to_string()) {
                    c.heartbeats = hbs.clone();
                }
            }
            enriched.push(c);
        }

        Ok(Self::rank_candidates(enriched))
    }

    pub fn parse_status_page_html(html: &str) -> Result<Vec<Candidate>> {
        let normalized = html.replace(r"\'", "'");

        if let Some(anna_idx) = normalized.find("Anna's Archive") {
            let sub = &normalized[anna_idx..];
            if let Some(start_pos) = sub.find("'monitorList':[").or_else(|| sub.find(r#""monitorList":["#)) {
                let list_start = anna_idx + start_pos + 14;
                if let Some(end_pos) = Self::find_matching_bracket(&normalized, list_start) {
                    let section = &normalized[list_start..=end_pos];
                    let candidates = Self::parse_candidates_from_section(section);
                    if !candidates.is_empty() {
                        return Ok(candidates);
                    }
                }
            }
        }

        // Fallback regex matching domain URLs
        let re = Regex::new(r"https://annas-archive\.[a-z0-9-]+/?").unwrap();
        let mut seen = HashMap::new();
        let mut candidates = Vec::new();

        for mat in re.find_iter(&normalized) {
            let base = Self::normalize_base_url(mat.as_str());
            if !seen.contains_key(&base) {
                seen.insert(base.clone(), ());
                candidates.push(Candidate {
                    monitor_id: 0,
                    base_url: base.clone(),
                    source_url: format!("https://{base}/"),
                    heartbeats: Vec::new(),
                });
            }
        }

        Ok(candidates)
    }

    fn parse_candidates_from_section(section: &str) -> Vec<Candidate> {
        let obj_re = Regex::new(
            r#"(?s)\{[^{}]*['"]id['"]\s*:\s*([0-9]+)[^{}]*['"]url['"]\s*:\s*['"]([^'"]+)['"][^{}]*\}"#,
        )
        .unwrap();

        let mut seen = HashMap::new();
        let mut candidates = Vec::new();

        for cap in obj_re.captures_iter(section) {
            if cap.len() >= 3 {
                let id: i32 = cap[1].parse().unwrap_or(0);
                let raw_url = &cap[2];
                let base = Self::normalize_base_url(raw_url);

                if base.starts_with("annas-archive.") && !seen.contains_key(&base) {
                    seen.insert(base.clone(), ());
                    candidates.push(Candidate {
                        monitor_id: id,
                        base_url: base.clone(),
                        source_url: raw_url.to_string(),
                        heartbeats: Vec::new(),
                    });
                }
            }
        }

        candidates
    }

    fn extract_slug(html: &str) -> String {
        let slug_re = Regex::new(r#"['"]slug['"]\s*:\s*['"]([^'"]+)['"]"#).unwrap();
        slug_re
            .captures(html)
            .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
            .unwrap_or_else(|| "slum".to_string())
    }

    fn find_matching_bracket(input: &str, start: usize) -> Option<usize> {
        let mut depth = 0;
        for (i, c) in input[start..].char_indices() {
            if c == '[' {
                depth += 1;
            } else if c == ']' {
                depth -= 1;
                if depth == 0 {
                    return Some(start + i);
                }
            }
        }
        None
    }

    pub fn rank_candidates(candidates: Vec<Candidate>) -> Vec<Candidate> {
        let mut ranked: Vec<Candidate> = candidates
            .into_iter()
            .filter(|c| {
                let score = c.score();
                score.last_status == 1 || score.sample_count == 0
            })
            .collect();

        ranked.sort_by(|a, b| {
            let sa = a.score();
            let sb = b.score();

            if sa.sample_count == 0 || sb.sample_count == 0 {
                return sa.sample_count.cmp(&sb.sample_count).reverse();
            }

            match sb.success_rate.partial_cmp(&sa.success_rate) {
                Some(std::cmp::Ordering::Equal) | None => {
                    match sa.average_ping.cmp(&sb.average_ping) {
                        std::cmp::Ordering::Equal => a.base_url.cmp(&b.base_url),
                        other => other,
                    }
                }
                Some(other) => other,
            }
        });

        ranked
    }

    pub async fn probe(&self, base_url: &str) -> bool {
        let test_url = format!("https://{}/search?q=test&content=book_any", Self::normalize_base_url(base_url));
        match self
            .client
            .get(&test_url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}
