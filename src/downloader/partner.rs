use std::time::Duration;
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{info, warn};

use crate::errors::Result;

pub struct PartnerDownloadResolver {
    client: Client,
}

impl PartnerDownloadResolver {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Generate public IPFS gateway URLs for a given CID
    pub fn get_ipfs_gateway_urls(cid: &str, filename: Option<&str>) -> Vec<String> {
        let clean_cid = cid.trim();
        let fname_param = filename
            .map(|f| format!("?filename={}", urlencoding::encode(f)))
            .unwrap_or_default();

        vec![
            format!("https://cloudflare-ipfs.com/ipfs/{clean_cid}{fname_param}"),
            format!("https://ipfs.io/ipfs/{clean_cid}{fname_param}"),
            format!("https://dweb.link/ipfs/{clean_cid}{fname_param}"),
            format!("https://gateway.pinata.cloud/ipfs/{clean_cid}{fname_param}"),
            format!("https://w3s.link/ipfs/{clean_cid}{fname_param}"),
        ]
    }

    /// Try resolving download links from Library.lol (Libgen main mirror)
    pub async fn resolve_library_lol(&self, md5: &str) -> Result<Vec<String>> {
        let url = format!("https://library.lol/main/{}", md5.trim().to_lowercase());
        info!("Resolving download links via Library.lol: {}", url);

        let resp = match self
            .client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                warn!("Library.lol returned HTTP {}", r.status());
                return Ok(Vec::new());
            }
            Err(e) => {
                warn!("Failed to query Library.lol: {e}");
                return Ok(Vec::new());
            }
        };

        let html = resp.text().await.unwrap_or_default();
        let urls = Self::parse_library_lol_html(&html);
        Ok(urls)
    }

    pub fn parse_library_lol_html(html: &str) -> Vec<String> {
        let mut links = Vec::new();
        let document = Html::parse_document(html);

        // Look for <h2><a href="...">GET</a></h2> or IPFS/download links in #info
        if let Ok(sel) = Selector::parse("#info a, #download a, h2 a") {
            for a in document.select(&sel) {
                if let Some(href) = a.value().attr("href") {
                    let href_trimmed = href.trim();
                    if href_trimmed.starts_with("http://") || href_trimmed.starts_with("https://") {
                        if !links.contains(&href_trimmed.to_string()) {
                            links.push(href_trimmed.to_string());
                        }
                    }
                }
            }
        }

        // Regex fallback for download URLs
        let re = Regex::new(r#"href=["'](https?://(?:download\.library\.lol|cloudflare-ipfs\.com|ipfs\.io)/[^"']+)["']"#).unwrap();
        for cap in re.captures_iter(html) {
            let u = cap[1].to_string();
            if !links.contains(&u) {
                links.push(u);
            }
        }

        links
    }

    /// Try resolving download link from Libgen.li / Libgen.rocks
    pub async fn resolve_libgen_li(&self, md5: &str) -> Result<Vec<String>> {
        let url = format!("https://libgen.li/ads.php?md5={}", md5.trim().to_lowercase());
        info!("Resolving download links via Libgen.li: {}", url);

        let resp = match self
            .client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => r,
            _ => return Ok(Vec::new()),
        };

        let html = resp.text().await.unwrap_or_default();
        let mut links = Vec::new();

        let re = Regex::new(r#"href=["'](get\.php\?md5=[^"']+)["']"#).unwrap();
        for cap in re.captures_iter(&html) {
            let full_url = format!("https://libgen.li/{}", &cap[1]);
            if !links.contains(&full_url) {
                links.push(full_url);
            }
        }

        Ok(links)
    }

    /// Try resolving Anna's Archive Slow Download queue
    pub async fn resolve_slow_download(&self, mirror: &str, md5: &str) -> Result<Option<String>> {
        let clean_md5 = md5.trim().to_lowercase();
        // Try slow partner mirror slots (slot 0, slot 1)
        for slot in ["0/0", "0/1", "0/2"] {
            let slow_url = format!("https://{}/slow_download/{}/{}", mirror, clean_md5, slot);
            info!("Attempting Anna's Archive slow partner queue: {}", slow_url);

            let resp = match self
                .client
                .get(&slow_url)
                .timeout(Duration::from_secs(10))
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => r,
                _ => continue,
            };

            let html = resp.text().await.unwrap_or_default();

            // Extract direct download link if immediately ready
            if let Some(direct_link) = Self::parse_slow_download_link(&html, mirror) {
                return Ok(Some(direct_link));
            }

            // Check for countdown timer (e.g. "wait 15 seconds")
            if let Some(wait_secs) = Self::extract_countdown_seconds(&html) {
                if wait_secs <= 35 {
                    info!("Waiting {} seconds in Anna's Archive slow download queue...", wait_secs);
                    tokio::time::sleep(Duration::from_secs(wait_secs + 1)).await;

                    // Re-request to fetch final link
                    if let Ok(retry_resp) = self.client.get(&slow_url).send().await {
                        if retry_resp.status().is_success() {
                            let retry_html = retry_resp.text().await.unwrap_or_default();
                            if let Some(link) = Self::parse_slow_download_link(&retry_html, mirror) {
                                return Ok(Some(link));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    pub fn parse_slow_download_link(html: &str, mirror: &str) -> Option<String> {
        let document = Html::parse_document(html);

        if let Ok(sel) = Selector::parse("a.js-download-link, a[href*='/slow_download_file/'], a:contains('Download now')") {
            for a in document.select(&sel) {
                if let Some(href) = a.value().attr("href") {
                    let h = href.trim();
                    if h.starts_with("http://") || h.starts_with("https://") {
                        return Some(h.to_string());
                    } else if h.starts_with('/') {
                        return Some(format!("https://{}{h}", mirror.trim_end_matches('/')));
                    }
                }
            }
        }

        let re = Regex::new(r#"href=["'](/slow_download_file/[^"']+)["']"#).ok()?;
        re.captures(html).map(|c| format!("https://{}{}", mirror.trim_end_matches('/'), &c[1]))
    }

    pub fn extract_countdown_seconds(html: &str) -> Option<u64> {
        let re = Regex::new(r#"(?i)(?:wait|countdown|seconds?|timer)[^\d]{0,20}(\d{1,3})\s*(?:s|sec|seconds)?"#).ok()?;
        if let Some(cap) = re.captures(html) {
            cap[1].parse::<u64>().ok()
        } else {
            None
        }
    }
}
