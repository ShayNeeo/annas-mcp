use serde_json::Value;

use crate::domain::metadata::{DownloadSource, Identifiers, IpfsInfo, ItemDetails};
use crate::errors::{AppError, Result};

pub fn parse_elasticsearch_record(raw_json: &str, md5: &str) -> Result<ItemDetails> {
    let trimmed = raw_json.trim();
    let json_str = if trimmed.starts_with('"') && trimmed.ends_with('"') {
        serde_json::from_str::<String>(trimmed)
            .map_err(|e| AppError::Api(format!("Failed to unquote outer JSON string: {e}")))?
    } else {
        trimmed.to_string()
    };

    let data: Value = serde_json::from_str(&json_str)
        .map_err(|e| AppError::Api(format!("Failed to parse record JSON: {e}")))?;

    if let Some(err_msg) = data.get("error").and_then(|v| v.as_str()) {
        return Err(AppError::Api(err_msg.to_string()));
    }

    let file_data = data.get("file_unified_data").unwrap_or(&data);

    let title = file_data
        .get("title_best")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Title")
        .to_string();

    let author = file_data
        .get("author_best")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let format = file_data
        .get("extension_best")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_uppercase());

    let size_bytes = file_data.get("filesize_best").and_then(|v| v.as_u64());
    let size = size_bytes.map(format_filesize);

    let language = file_data
        .get("language_codes")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .map(String::from);

    let publisher = file_data
        .get("publisher_best")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let year = file_data
        .get("year_best")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let description = file_data
        .get("stripped_description_best")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let cover_url = file_data
        .get("cover_url_best")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let content_type = file_data
        .get("content_type_best")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let original_filename = file_data
        .get("original_filename_best")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let pages = file_data
        .get("pages_best")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let edition = file_data
        .get("edition_varia_best")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let series = file_data
        .get("series_best")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let identifiers = parse_identifiers(file_data.get("identifiers_unified"));
    let categories = parse_categories(file_data.get("classifications_unified"));
    let ipfs_cids = parse_ipfs_infos(file_data.get("ipfs_infos"));

    let additional = data.get("additional");
    let download_sources = parse_download_sources(additional);
    let torrent_paths = parse_torrent_paths(additional);

    Ok(ItemDetails {
        md5: md5.to_string(),
        title,
        author,
        format,
        size,
        size_bytes,
        language,
        publisher,
        year,
        description,
        cover_url,
        content_type,
        original_filename,
        pages,
        edition,
        series,
        identifiers,
        categories,
        ipfs_cids,
        download_sources,
        torrent_paths,
    })
}

fn parse_identifiers(val: Option<&Value>) -> Option<Identifiers> {
    let obj = val?.as_object()?;

    let get_str_vec = |key: &str| -> Option<Vec<String>> {
        obj.get(key).and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
        })
    };

    let get_first_str = |key: &str| -> Option<String> {
        obj.get(key)
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .map(String::from)
    };

    Some(Identifiers {
        isbn10: get_str_vec("isbn10"),
        isbn13: get_str_vec("isbn13"),
        doi: get_str_vec("doi"),
        asin: get_str_vec("asin"),
        sha1: get_first_str("sha1"),
        sha256: get_first_str("sha256"),
        open_library: get_str_vec("ol"),
        google_books: get_str_vec("googlebookid"),
        goodreads: get_str_vec("goodreads"),
    })
}

fn parse_categories(val: Option<&Value>) -> Option<Vec<String>> {
    let obj = val?.as_object()?;
    let mut result = Vec::new();
    for (k, v) in obj {
        if k == "collection" || k.starts_with('_') {
            continue;
        }
        if let Some(arr) = v.as_array() {
            for item in arr {
                if let Some(s) = item.as_str() {
                    let st = s.trim().to_string();
                    if !st.is_empty() && !result.contains(&st) {
                        result.push(st);
                    }
                }
            }
        }
    }
    if result.is_empty() { None } else { Some(result) }
}

fn parse_ipfs_infos(val: Option<&Value>) -> Option<Vec<IpfsInfo>> {
    let arr = val?.as_array()?;
    let list: Vec<IpfsInfo> = arr
        .iter()
        .filter_map(|v| {
            let obj = v.as_object()?;
            let cid = obj.get("ipfs_cid")?.as_str()?.to_string();
            let from = obj
                .get("from")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string();
            Some(IpfsInfo { cid, from })
        })
        .collect();

    if list.is_empty() { None } else { Some(list) }
}

fn parse_download_sources(val: Option<&Value>) -> Option<Vec<DownloadSource>> {
    let obj = val?.as_object()?;
    let mut sources = Vec::new();

    if let Some(urls) = obj.get("download_urls").and_then(|v| v.as_array()) {
        for u in urls {
            if let Some(s) = u.as_str() {
                sources.push(DownloadSource {
                    name: "direct".to_string(),
                    url: s.to_string(),
                });
            }
        }
    }

    if let Some(urls) = obj.get("ipfs_urls").and_then(|v| v.as_array()) {
        for u in urls {
            if let Some(s) = u.as_str() {
                sources.push(DownloadSource {
                    name: "ipfs".to_string(),
                    url: s.to_string(),
                });
            }
        }
    }

    if sources.is_empty() { None } else { Some(sources) }
}

fn parse_torrent_paths(val: Option<&Value>) -> Option<Vec<String>> {
    let arr = val?.as_object()?.get("torrent_paths")?.as_array()?;
    let list: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
    if list.is_empty() { None } else { Some(list) }
}

fn format_filesize(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}MB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes}B")
    }
}
