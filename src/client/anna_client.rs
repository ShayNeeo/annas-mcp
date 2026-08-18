use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::client::elasticsearch::parse_elasticsearch_record;
use crate::client::scraper::AnnaScraper;
use crate::config::AppConfig;
use crate::domain::{Book, ItemDetails, Paper};
use crate::downloader::{DownloadedFileInfo, FileDownloader, PartnerDownloadResolver};
use crate::errors::{AppError, Result};
use crate::mirror::MirrorResolver;

#[derive(Deserialize)]
struct FastDownloadApiResponse {
    download_url: Option<String>,
    error: Option<String>,
}

use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub struct AnnaClient {
    client: Client,
    config: Arc<AppConfig>,
    resolver: Arc<MirrorResolver>,
    active_mirror: Arc<RwLock<Option<String>>>,
    downloader: Arc<FileDownloader>,
    partner_resolver: Arc<PartnerDownloadResolver>,
    authenticated: Arc<AtomicBool>,
}

impl AnnaClient {
    pub fn new(config: AppConfig) -> Result<Self> {
        let timeout = Duration::from_secs(config.timeout_secs);
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(&config.user_agent)
            .cookie_store(true)
            .build()?;

        let resolver = Arc::new(MirrorResolver::new(
            Some(client.clone()),
            Some(config.wikipedia_url.clone()),
            Some(config.slum_status_url.clone()),
        ));

        let downloader = Arc::new(FileDownloader::new(client.clone()));
        let partner_resolver = Arc::new(PartnerDownloadResolver::new(client.clone()));

        Ok(Self {
            client,
            config: Arc::new(config),
            resolver,
            active_mirror: Arc::new(RwLock::new(None)),
            downloader,
            partner_resolver,
            authenticated: Arc::new(AtomicBool::new(false)),
        })
    }

    pub async fn ensure_authenticated(&self) -> Result<()> {
        if let Some(ref key) = self.config.secret_key {
            if !self.authenticated.load(Ordering::SeqCst) {
                let mirror = self.get_active_mirror().await?;
                let login_url = format!("https://{}/account/", mirror);
                let resp = self
                    .client
                    .post(&login_url)
                    .form(&[("key", key.as_str())])
                    .send()
                    .await;

                if let Ok(r) = resp {
                    if r.status().is_success() || r.status().is_redirection() {
                        self.authenticated.store(true, Ordering::SeqCst);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn resolver(&self) -> &MirrorResolver {
        &self.resolver
    }

    pub async fn get_active_mirror(&self) -> Result<String> {
        // If explicit base_url configured, use it
        if let Some(ref explicit) = self.config.base_url {
            return Ok(MirrorResolver::normalize_base_url(explicit));
        }

        {
            let guard = self.active_mirror.read().await;
            if let Some(ref m) = *guard {
                return Ok(m.clone());
            }
        }

        let mut guard = self.active_mirror.write().await;
        if let Some(ref m) = *guard {
            return Ok(m.clone());
        }

        let resolved = self.resolver.resolve(None).await?;
        *guard = Some(resolved.clone());
        Ok(resolved)
    }

    pub async fn invalidate_active_mirror(&self) {
        let mut guard = self.active_mirror.write().await;
        *guard = None;
    }

    pub async fn search_books(&self, query: &str, limit: Option<usize>) -> Result<Vec<Book>> {
        let _ = self.ensure_authenticated().await;
        let mirror = self.get_active_mirror().await?;
        let encoded_q = urlencoding::encode(query);
        let url = format!(
            "https://{}/search?q={}&content=book_any",
            mirror, encoded_q
        );

        info!("Searching books: {}", url);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Network(e))?;

        if !resp.status().is_success() {
            self.invalidate_active_mirror().await;
            return Err(AppError::Api(format!(
                "Search request failed with HTTP {}",
                resp.status()
            )));
        }

        let html = resp.text().await?;
        let mut books = AnnaScraper::parse_books(&html, &mirror)?;

        if let Some(max) = limit {
            if books.len() > max {
                books.truncate(max);
            }
        }

        Ok(books)
    }

    pub async fn search_articles(&self, query: &str, limit: Option<usize>) -> Result<Vec<Paper>> {
        let _ = self.ensure_authenticated().await;
        let trimmed = query.trim();
        // If it starts with 10., treat as DOI lookup
        if trimmed.starts_with("10.") {
            let paper = self.lookup_doi(trimmed).await?;
            return Ok(vec![paper]);
        }

        let mirror = self.get_active_mirror().await?;
        let encoded_q = urlencoding::encode(query);
        let url = format!("https://{}/search?q={}&content=journal", mirror, encoded_q);

        info!("Searching articles: {}", url);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Network(e))?;

        if !resp.status().is_success() {
            self.invalidate_active_mirror().await;
            return Err(AppError::Api(format!(
                "Article search request failed with HTTP {}",
                resp.status()
            )));
        }

        let html = resp.text().await?;
        let mut papers = AnnaScraper::parse_articles(&html, &mirror)?;

        if let Some(max) = limit {
            if papers.len() > max {
                papers.truncate(max);
            }
        }

        Ok(papers)
    }

    pub async fn lookup_doi(&self, doi: &str) -> Result<Paper> {
        let _ = self.ensure_authenticated().await;
        let clean_doi = doi.trim();

        // 1. Try Sci-Hub mirrors for direct PDF streams and publication metadata
        let scihub_hosts = ["sci-hub.ru", "sci-hub.st", "sci-hub.se"];
        for host in scihub_hosts {
            let scihub_url = format!("https://{}/{}", host, clean_doi);
            info!("Looking up DOI via Sci-Hub: {}", scihub_url);

            match self.client.get(&scihub_url).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        if let Ok(html) = resp.text().await {
                            info!("Sci-Hub {} returned HTTP {}, body length: {}", host, status, html.len());
                            if let Some(paper) = AnnaScraper::parse_scihub_page(&html, clean_doi, host) {
                                info!("Parsed paper: title={:?}, download_url={:?}", paper.title, paper.download_url);
                                if paper.download_url.is_some() {
                                    info!("Successfully resolved direct Sci-Hub paper PDF from https://{}", host);
                                    return Ok(paper);
                                }
                            } else {
                                warn!("Failed to parse Sci-Hub HTML from https://{}", host);
                            }
                        }
                    } else {
                        warn!("Sci-Hub {} returned HTTP {}", host, status);
                    }
                }
                Err(e) => {
                    warn!("Sci-Hub request to {} failed: {}", host, e);
                }
            }
        }

        // 2. Fall back to Anna's Archive active mirror SciDB
        let mirror = self.get_active_mirror().await?;
        let scidb_search_url = format!("https://{}/scidb/{}", mirror, clean_doi);

        info!("Looking up DOI via Anna's Archive SciDB: {}", scidb_search_url);

        let resp = self
            .client
            .get(&scidb_search_url)
            .send()
            .await
            .map_err(AppError::Network)?;

        if !resp.status().is_success() {
            return Err(AppError::NotFound(format!("No paper found for DOI: {doi}")));
        }

        let html = resp.text().await?;
        let found_hash = AnnaScraper::parse_scidb_search_for_md5(&html);

        let mut paper = if let Some(ref hash) = found_hash {
            let detail_url = format!("https://{}/md5/{}", mirror, hash);
            if let Ok(detail_resp) = self.client.get(&detail_url).send().await {
                if detail_resp.status().is_success() {
                    let detail_html = detail_resp.text().await.unwrap_or_default();
                    let mut p = AnnaScraper::parse_scidb_detail_page(&detail_html, clean_doi, &mirror);
                    p.hash = Some(hash.clone());
                    p
                } else {
                    AnnaScraper::parse_scidb_detail_page(&html, clean_doi, &mirror)
                }
            } else {
                AnnaScraper::parse_scidb_detail_page(&html, clean_doi, &mirror)
            }
        } else {
            AnnaScraper::parse_scidb_detail_page(&html, clean_doi, &mirror)
        };

        if paper.hash.is_none() {
            paper.hash = found_hash;
        }

        Ok(paper)
    }

    pub async fn get_fast_download_url(&self, md5: &str) -> Result<String> {
        let key = self
            .config
            .secret_key
            .as_ref()
            .ok_or_else(|| AppError::Config("ANNAS_SECRET_KEY is required for book downloads".to_string()))?;

        let mirror = self.get_active_mirror().await?;
        let api_url = format!(
            "https://{}/dyn/api/fast_download.json?md5={}&key={}",
            mirror, md5, key
        );

        info!("Requesting fast download URL for MD5: {}", md5);

        let resp = self
            .client
            .get(&api_url)
            .send()
            .await
            .map_err(AppError::Network)?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Api(format!(
                "Fast download API returned HTTP {status}: {body}"
            )));
        }

        let api_data: FastDownloadApiResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Api(format!("Failed to parse fast download response: {e}")))?;

        if let Some(err) = api_data.error {
            return Err(AppError::Api(format!("Fast download error: {err}")));
        }

        let download_url = api_data
            .download_url
            .ok_or_else(|| AppError::Api("API returned empty download URL".to_string()))?;

        Ok(download_url)
    }

    pub async fn get_item_details(&self, md5: &str) -> Result<ItemDetails> {
        let mirror = self.get_active_mirror().await?;
        let url = format!(
            "https://{}/db/aarecord_elasticsearch/md5:{}.json",
            mirror, md5
        );

        info!("Fetching Elasticsearch item details for MD5: {}", md5);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(AppError::Network)?;

        if !resp.status().is_success() {
            return Err(AppError::Api(format!(
                "Details API returned HTTP {}",
                resp.status()
            )));
        }

        let body = resp.text().await?;
        parse_elasticsearch_record(&body, md5)
    }

    pub async fn download_book(
        &self,
        md5: &str,
        title: &str,
        format: &str,
        dest_dir: Option<&Path>,
    ) -> Result<DownloadedFileInfo> {
        let clean_md5 = md5.trim();
        let target_folder = dest_dir.unwrap_or(&self.config.download_path);

        // Tier 1: Fast Download API (if ANNAS_SECRET_KEY is configured)
        if self.config.secret_key.is_some() {
            match self.get_fast_download_url(clean_md5).await {
                Ok(fast_url) => {
                    info!("Downloading book via Fast Download API");
                    match self
                        .downloader
                        .download_to_folder(&fast_url, target_folder, title, format)
                        .await
                    {
                        Ok(info) => return Ok(info),
                        Err(e) => warn!("Fast download API stream failed: {}, falling back to free mirrors", e),
                    }
                }
                Err(e) => warn!("Fast download URL resolution failed: {}, falling back to free mirrors", e),
            }
        }

        info!("Starting automated multi-tier free partner download for MD5: {}", clean_md5);

        // Tier 2: IPFS Gateway Direct Resolver
        if let Ok(details) = self.get_item_details(clean_md5).await {
            if let Some(ref cids) = details.ipfs_cids {
                let filename = format!("{}.{}", title, format);
                for ipfs_info in cids {
                    let ipfs_urls = PartnerDownloadResolver::get_ipfs_gateway_urls(&ipfs_info.cid, Some(&filename));
                    for ipfs_url in ipfs_urls {
                        info!("Attempting free IPFS download via: {}", ipfs_url);
                        if let Ok(info) = self
                            .downloader
                            .download_to_folder(&ipfs_url, target_folder, title, format)
                            .await
                        {
                            info!("Successfully downloaded book from free IPFS gateway!");
                            return Ok(info);
                        }
                    }
                }
            }
        }

        // Tier 3: External Mirror Resolvers (Library.lol & Libgen)
        if let Ok(libgen_urls) = self.partner_resolver.resolve_library_lol(clean_md5).await {
            for dl_url in libgen_urls {
                info!("Attempting free download via Library.lol partner mirror: {}", dl_url);
                if let Ok(info) = self
                    .downloader
                    .download_to_folder(&dl_url, target_folder, title, format)
                    .await
                {
                    info!("Successfully downloaded book from Library.lol partner mirror!");
                    return Ok(info);
                }
            }
        }

        if let Ok(libgen_li_urls) = self.partner_resolver.resolve_libgen_li(clean_md5).await {
            for dl_url in libgen_li_urls {
                info!("Attempting free download via Libgen.li partner mirror: {}", dl_url);
                if let Ok(info) = self
                    .downloader
                    .download_to_folder(&dl_url, target_folder, title, format)
                    .await
                {
                    info!("Successfully downloaded book from Libgen.li partner mirror!");
                    return Ok(info);
                }
            }
        }

        // Tier 4: Anna's Archive Slow Download Queue
        let mirror = self.get_active_mirror().await.unwrap_or_else(|_| "annas-archive.pk".to_string());
        if let Ok(Some(slow_url)) = self.partner_resolver.resolve_slow_download(&mirror, clean_md5).await {
            info!("Attempting download via Anna's Archive Slow Partner link: {}", slow_url);
            if let Ok(info) = self
                .downloader
                .download_to_folder(&slow_url, target_folder, title, format)
                .await
            {
                info!("Successfully downloaded book from Anna's Archive Slow Partner queue!");
                return Ok(info);
            }
        }

        // Tier 5: All automated free download channels failed (Anti-bot / Captcha)
        let web_url = format!("https://{}/md5/{}", mirror, clean_md5);
        Err(AppError::Download(format!(
            "Could not automatically download book via Fast API, IPFS, Libgen, or Slow Partner.\n\
             Please open this link in your browser to download for free via partner mirrors:\n{}",
            web_url
        )))
    }

    pub async fn download_paper(
        &self,
        doi: &str,
        dest_dir: Option<&Path>,
    ) -> Result<DownloadedFileInfo> {
        let paper = self.lookup_doi(doi).await?;
        let target_folder = dest_dir.unwrap_or(&self.config.download_path);

        // Try fast download if key & hash available
        if let (Some(ref hash), Some(_)) = (&paper.hash, &self.config.secret_key) {
            if let Ok(dl_url) = self.get_fast_download_url(hash).await {
                info!("Downloading paper via Fast Download API");
                let title = paper.title.as_deref().unwrap_or(doi);
                if let Ok(res) = self
                    .downloader
                    .download_to_folder(&dl_url, target_folder, title, "pdf")
                    .await
                {
                    return Ok(res);
                }
                warn!("Fast download attempt failed, falling back to SciDB direct download");
            }
        }

        // Fallback to direct SciDB download
        let dl_url = paper
            .download_url
            .ok_or_else(|| AppError::Download("No download URL available for paper".to_string()))?;

        info!("Downloading paper via SciDB direct endpoint: {}", dl_url);
        let title = paper.title.as_deref().unwrap_or(doi);

        self.downloader
            .download_to_folder(&dl_url, target_folder, title, "pdf")
            .await
    }
}
