use annas_mcp::client::scraper::{parse_meta_parts, AnnaScraper};

#[test]
fn test_parse_meta_parts() {
    let meta = "✅ English [en] · EPUB · 4.2MB · 2023 · Other info";
    let (lang, fmt, size, yr) = parse_meta_parts(meta);

    assert_eq!(lang, Some("English [en]".to_string()));
    assert_eq!(fmt, Some("EPUB".to_string()));
    assert_eq!(size, Some("4.2MB".to_string()));
    assert_eq!(yr, Some("2023".to_string()));
}

#[test]
fn test_parse_meta_parts_pdf() {
    let meta = "PDF · 54.2MB · 1987";
    let (lang, fmt, size, yr) = parse_meta_parts(meta);

    assert_eq!(lang, None);
    assert_eq!(fmt, Some("PDF".to_string()));
    assert_eq!(size, Some("54.2MB".to_string()));
    assert_eq!(yr, Some("1987".to_string()));
}

#[test]
fn test_parse_html_book_card() {
    let html = r#"
    <div class="flex pt-3 pb-3 border-b">
        <a href="/md5/abcdef1234567890abcdef1234567890" class="custom-a block mr-2 sm:mr-4 hover:opacity-80">
            <img src="/covers/abcdef.jpg" alt="Cover" />
        </a>
        <div class="max-w-full">
            <a href="/md5/abcdef1234567890abcdef1234567890" class="js-vim-focus">
                The Rust Programming Language
            </a>
            <div class="text-xs text-gray-500">
                <a href="/search?q=Steve+Klabnik">
                    <span class="icon-[mdi--user-edit]"></span> Steve Klabnik, Carol Nichols
                </a>
                <a href="/search?q=No+Starch+Press">
                    <span class="icon-[mdi--company]"></span> No Starch Press
                </a>
            </div>
            <div class="text-gray-800 text-sm">
                ✅ English [en] · EPUB · 12.5MB · 2023
            </div>
        </div>
    </div>
    "#;

    let books = AnnaScraper::parse_books(html, "annas-archive.li").expect("Failed to parse books");
    assert_eq!(books.len(), 1);

    let b = &books[0];
    assert_eq!(b.hash, "abcdef1234567890abcdef1234567890");
    assert_eq!(b.title, "The Rust Programming Language");
    assert_eq!(b.authors, Some("Steve Klabnik, Carol Nichols".to_string()));
    assert_eq!(b.publisher, Some("No Starch Press".to_string()));
    assert_eq!(b.language, Some("English [en]".to_string()));
    assert_eq!(b.format, Some("EPUB".to_string()));
    assert_eq!(b.size, Some("12.5MB".to_string()));
    assert_eq!(b.year, Some("2023".to_string()));
    assert_eq!(b.url, "https://annas-archive.li/md5/abcdef1234567890abcdef1234567890");
}

#[test]
fn test_parse_scihub_page() {
    let html = r#"
    <!DOCTYPE html>
    <html>
        <head>
            <title>Sci-Hub. Nanometre-scale thermometry in a living cell / Nature, 2013</title>
            <meta name="citation_title" content="Nanometre-scale thermometry in a living cell">
            <meta name="citation_author" content="Kucsko, G.">
            <meta name="citation_author" content="Maurer, P. C.">
            <meta name="citation_publication_date" content="2013">
            <meta name="citation_journal_title" content="Nature">
            <meta name="citation_doi" content="10.1038/nature12373">
            <meta name="citation_pdf_url" content="/storage/2024/2161/f1fa2076e55135dec9460db8704912d7/kucsko2013.pdf">
        </head>
        <body></body>
    </html>
    "#;

    let paper = AnnaScraper::parse_scihub_page(html, "10.1038/nature12373", "sci-hub.ru")
        .expect("Failed to parse Sci-Hub paper");
    assert_eq!(paper.doi, "10.1038/nature12373");
    assert_eq!(paper.title, Some("Nanometre-scale thermometry in a living cell".to_string()));
    assert_eq!(paper.authors, Some("Kucsko, G., Maurer, P. C.".to_string()));
    assert_eq!(paper.journal, Some("Nature".to_string()));
    assert_eq!(
        paper.download_url,
        Some("https://sci-hub.ru/storage/2024/2161/f1fa2076e55135dec9460db8704912d7/kucsko2013.pdf".to_string())
    );
}
