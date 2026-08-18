use regex::Regex;
use scraper::{ElementRef, Html, Node, Selector};

use crate::domain::{Book, Paper};
use crate::errors::{AppError, Result};

pub struct AnnaScraper;

impl AnnaScraper {
    pub fn parse_books(html: &str, base_url: &str) -> Result<Vec<Book>> {
        let document = Html::parse_document(html);

        // Selector for main item links
        let link_selector = Selector::parse("a[href^='/md5/']").map_err(|e| {
            AppError::HtmlParse(format!("Invalid link selector: {e:?}"))
        })?;

        let mut books = Vec::new();
        let mut seen_hashes = std::collections::HashSet::new();

        for link_elem in document.select(&link_selector) {

            let href = match link_elem.value().attr("href") {
                Some(h) if h.starts_with("/md5/") => h,
                _ => continue,
            };

            let hash = href.trim_start_matches("/md5/").to_string();
            if hash.is_empty() || seen_hashes.contains(&hash) {
                continue;
            }

            // Find parent container
            let parent = match link_elem.ancestors().find_map(ElementRef::wrap) {
                Some(p) => p,
                None => continue,
            };

            // Search for container with metadata inside the same card
            let container = parent
                .ancestors()
                .take(3)
                .find_map(ElementRef::wrap)
                .unwrap_or(parent);

            // Extract title
            let title = extract_title(&container, &link_elem);
            if title.is_empty() {
                continue;
            }

            seen_hashes.insert(hash.clone());

            // Extract authors & publisher
            let (authors, publisher) = extract_authors_and_publisher(&container);

            // Extract metadata (language, format, size, year)
            let (language, format, size, year) = extract_meta_line(&container);

            // Extract cover image
            let cover_url = extract_cover_image(&container, base_url);

            let book_url = format!("https://{}/md5/{}", base_url.trim_end_matches('/'), hash);

            books.push(Book {
                hash,
                title,
                authors,
                publisher,
                language,
                format,
                size,
                year,
                url: book_url,
                cover_url,
                description: None,
            });
        }

        Ok(books)
    }

    pub fn parse_articles(html: &str, base_url: &str) -> Result<Vec<Paper>> {
        let document = Html::parse_document(html);
        let link_selector = Selector::parse("a[href^='/md5/']").map_err(|e| {
            AppError::HtmlParse(format!("Invalid link selector: {e:?}"))
        })?;

        let mut papers = Vec::new();
        let mut seen_hashes = std::collections::HashSet::new();

        for link_elem in document.select(&link_selector) {
            let href = match link_elem.value().attr("href") {
                Some(h) if h.starts_with("/md5/") => h,
                _ => continue,
            };

            let hash = href.trim_start_matches("/md5/").to_string();
            if hash.is_empty() || seen_hashes.contains(&hash) {
                continue;
            }

            let parent = match link_elem.ancestors().find_map(ElementRef::wrap) {
                Some(p) => p,
                None => continue,
            };

            let container = parent
                .ancestors()
                .take(3)
                .find_map(ElementRef::wrap)
                .unwrap_or(parent);

            let title = extract_title(&container, &link_elem);
            if title.is_empty() {
                continue;
            }

            seen_hashes.insert(hash.clone());

            let (authors, journal) = extract_authors_and_publisher(&container);
            let (_lang, _fmt, size, _yr) = extract_meta_line(&container);

            let page_url = format!("https://{}/md5/{}", base_url.trim_end_matches('/'), hash);

            papers.push(Paper {
                doi: String::new(),
                title: Some(title),
                authors,
                journal,
                size,
                hash: Some(hash),
                download_url: None,
                page_url,
                scihub_url: None,
            });
        }

        Ok(papers)
    }

    pub fn parse_scidb_search_for_md5(html: &str) -> Option<String> {
        let document = Html::parse_document(html);
        let link_selector = Selector::parse("a[href^='/md5/']").ok()?;

        for link in document.select(&link_selector) {
            if let Some(href) = link.value().attr("href") {
                let hash = href.trim_start_matches("/md5/").trim();
                if !hash.is_empty() {
                    return Some(hash.to_string());
                }
            }
        }
        None
    }

    pub fn parse_scidb_detail_page(html: &str, doi: &str, base_url: &str) -> Paper {
        let document = Html::parse_document(html);

        // Title from <title> tag
        let title_selector = Selector::parse("title").ok();
        let title = title_selector
            .and_then(|s| document.select(&s).next())
            .map(|e| {
                let full = e.text().collect::<String>();
                if let Some(idx) = full.find(" - Anna") {
                    full[..idx].trim().to_string()
                } else {
                    full.trim().to_string()
                }
            });

        // Metadata description: Format often "Authors\n\nPublisher\n\nJournal, #issue, vol, pages, year"
        let desc_selector = Selector::parse("meta[name='description']").ok();
        let mut journal = None;
        if let Some(sel) = desc_selector {
            if let Some(desc_elem) = document.select(&sel).next() {
                if let Some(content) = desc_elem.value().attr("content") {
                    let parts: Vec<&str> = content.split("\n\n").collect();
                    if parts.len() >= 3 {
                        journal = Some(parts[2].trim().to_string());
                    } else if parts.len() >= 2 {
                        journal = Some(parts[1].trim().to_string());
                    }
                }
            }
        }

        // Authors from search link
        let mut authors = None;
        if let Ok(search_link_sel) = Selector::parse("a[href^='/search']") {
            if let Ok(icon_sel) = Selector::parse("span.icon-\\[mdi--user-edit\\]") {
                for a in document.select(&search_link_sel) {
                    if a.select(&icon_sel).next().is_some() {
                        let text = a.text().collect::<String>().trim().to_string();
                        if !text.is_empty() {
                            authors = Some(text);
                            break;
                        }
                    }
                }
            }
        }

        // Size from gray text
        let mut size = None;
        if let Ok(gray_sel) = Selector::parse("div.text-gray-500, div.text-gray-800") {
            for div in document.select(&gray_sel) {
                let text = div.text().collect::<String>();
                if text.contains("MB") || text.contains("KB") || text.contains("GB") {
                    size = Some(text.trim().to_string());
                    break;
                }
            }
        }

        let scidb_download = format!("https://{}/scidb?doi={}", base_url.trim_end_matches('/'), doi);
        let page_url = format!("https://{}/scidb/{}", base_url.trim_end_matches('/'), doi);

        Paper {
            doi: doi.to_string(),
            title,
            authors,
            journal,
            size,
            hash: None,
            download_url: Some(scidb_download),
            page_url,
            scihub_url: Some(format!("https://sci-hub.se/{doi}")),
        }
    }

    pub fn parse_scihub_page(html: &str, doi: &str, host: &str) -> Option<Paper> {
        // Extract title: meta name="citation_title"
        let title_re = Regex::new(r#"(?i)<meta\s+name=["']citation_title["']\s+content=["']([^"']+)["']"#).ok()?;
        let title = title_re.captures(html).map(|c| c[1].trim().to_string());

        // Extract authors: meta name="citation_author"
        let author_re = Regex::new(r#"(?i)<meta\s+name=["']citation_author["']\s+content=["']([^"']+)["']"#).ok()?;
        let mut authors = Vec::new();
        for cap in author_re.captures_iter(html) {
            let a = cap[1].trim();
            if !a.is_empty() && a != "et., al." && a != "et al." {
                authors.push(a.to_string());
            }
        }
        let authors_str = if !authors.is_empty() {
            Some(authors.join(", "))
        } else {
            None
        };

        // Extract journal: meta name="citation_journal_title"
        let journal_re = Regex::new(r#"(?i)<meta\s+name=["']citation_journal_title["']\s+content=["']([^"']+)["']"#).ok()?;
        let journal = journal_re.captures(html).map(|c| c[1].trim().to_string());

        // Extract PDF download URL
        let pdf_re = Regex::new(r#"(?i)<meta\s+name=["']citation_pdf_url["']\s+content=["']([^"']+)["']"#).ok()?;
        let mut pdf_url = pdf_re.captures(html).map(|c| c[1].trim().to_string());

        if pdf_url.is_none() {
            let embed_re = Regex::new(r#"(?i)<(?:embed|iframe)[^>]+src=["']([^"']+\.pdf[^"']*)["']"#).ok()?;
            pdf_url = embed_re.captures(html).map(|c| c[1].trim().to_string());
        }

        let resolved_pdf_url = pdf_url.map(|u| {
            if u.starts_with("//") {
                format!("https:{u}")
            } else if u.starts_with('/') {
                format!("https://{}{u}", host.trim_end_matches('/'))
            } else if !u.starts_with("http") {
                format!("https://{}/{}", host.trim_end_matches('/'), u)
            } else {
                u
            }
        });

        if title.is_some() || resolved_pdf_url.is_some() {
            Some(Paper {
                doi: doi.to_string(),
                title,
                authors: authors_str,
                journal,
                size: None,
                hash: None,
                download_url: resolved_pdf_url,
                page_url: format!("https://{}/{}", host.trim_end_matches('/'), doi),
                scihub_url: Some(format!("https://{}/{}", host.trim_end_matches('/'), doi)),
            })
        } else {
            None
        }
    }
}

fn extract_title(container: &ElementRef, _link: &ElementRef) -> String {
    // Check for title link
    let selectors = ["a.js-vim-focus", "div.max-w-full a[href^='/md5/']", "h3", "a[href^='/md5/']"];
    for sel_str in selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            for elem in container.select(&sel) {
                let text = elem.text().collect::<String>().trim().to_string();
                if !text.is_empty() {
                    return text;
                }
            }
        }
    }
    String::new()
}

fn extract_authors_and_publisher(container: &ElementRef) -> (Option<String>, Option<String>) {
    let mut authors = None;
    let mut publisher = None;

    if let Ok(search_sel) = Selector::parse("a[href^='/search']") {
        let user_icon_sel = Selector::parse("span.icon-\\[mdi--user-edit\\]").ok();
        let company_icon_sel = Selector::parse("span.icon-\\[mdi--company\\]").ok();

        for link in container.select(&search_sel) {
            if let Some(ref icon_sel) = user_icon_sel {
                if link.select(icon_sel).next().is_some() {
                    let text = link.text().collect::<String>().trim().to_string();
                    if !text.is_empty() && authors.is_none() {
                        authors = Some(text);
                    }
                }
            }

            if let Some(ref icon_sel) = company_icon_sel {
                if link.select(icon_sel).next().is_some() {
                    let text = link.text().collect::<String>().trim().to_string();
                    if !text.is_empty() && publisher.is_none() {
                        publisher = Some(text);
                    }
                }
            }
        }
    }

    (authors, publisher)
}

fn extract_meta_line(container: &ElementRef) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let selectors = [
        "div.text-gray-800",
        "div.text-gray-500",
        "div.line-clamp-\\[2\\]",
    ];

    for sel_str in selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            for elem in container.select(&sel) {
                let text = extract_text_without_scripts(elem);
                if text.contains('·') || text.contains("MB") || text.contains("KB") || text.contains("EPUB") || text.contains("PDF") {
                    let (lang, fmt, size, yr) = parse_meta_parts(&text);
                    if lang.is_some() || fmt.is_some() || size.is_some() {
                        return (lang, fmt, size, yr);
                    }
                }
            }
        }
    }

    (None, None, None, None)
}

pub fn parse_meta_parts(meta: &str) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let parts: Vec<&str> = meta.split('·').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return (None, None, None, None);
    }

    let mut language = None;
    let mut format = None;
    let mut size = None;
    let mut year = None;

    let format_re = Regex::new(r"(?i)\b(EPUB|PDF|MOBI|AZW3|AZW|DJVU|CBZ|CBR|FB2|DOCX?|TXT|RTF)\b").unwrap();
    let size_re = Regex::new(r"(?i)\d+\.?\d*\s*(MB|KB|GB|TB|B)\b").unwrap();
    let year_re = Regex::new(r"\b(19\d\d|20\d\d)\b").unwrap();

    for (idx, part) in parts.iter().enumerate() {
        let p = part.trim();

        // Extract language from first part if it contains language code [en] or checkmark
        if idx == 0 && (p.contains('[') && p.contains(']')) {
            let clean = p.trim_start_matches("✅").trim();
            language = Some(clean.to_string());
            continue;
        }

        if format.is_none() {
            if let Some(mat) = format_re.captures(p) {
                if let Some(m) = mat.get(1) {
                    format = Some(m.as_str().to_uppercase());
                }
            }
        }

        if size.is_none() {
            if let Some(mat) = size_re.find(p) {
                size = Some(mat.as_str().to_string());
            }
        }

        if year.is_none() {
            if let Some(mat) = year_re.find(p) {
                year = Some(mat.as_str().to_string());
            }
        }
    }

    (language, format, size, year)
}

fn extract_cover_image(container: &ElementRef, base_url: &str) -> Option<String> {
    if let Ok(img_sel) = Selector::parse("img") {
        for img in container.select(&img_sel) {
            if let Some(src) = img.value().attr("src") {
                if src.contains("/covers/") || src.starts_with("http") || src.starts_with('/') {
                    if src.starts_with("http") {
                        return Some(src.to_string());
                    } else {
                        return Some(format!("https://{}{}", base_url.trim_end_matches('/'), src));
                    }
                }
            }
        }
    }
    None
}

fn extract_text_without_scripts(element: ElementRef) -> String {
    let mut text = String::new();
    for node in element.descendants() {
        if let Node::Text(t) = node.value() {
            let in_script = node.ancestors().any(|ancestor| {
                ancestor
                    .value()
                    .as_element()
                    .map(|el| el.name() == "script" || el.name() == "style")
                    .unwrap_or(false)
            });

            if !in_script {
                text.push_str(t);
            }
        }
    }
    text
}
