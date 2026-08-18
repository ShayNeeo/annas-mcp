use annas_mcp::downloader::PartnerDownloadResolver;

#[test]
fn test_ipfs_gateway_urls() {
    let urls = PartnerDownloadResolver::get_ipfs_gateway_urls("bafykbzacedtestcid123", Some("Rust_Book.epub"));
    assert_eq!(urls.len(), 5);
    assert!(urls[0].contains("cloudflare-ipfs.com/ipfs/bafykbzacedtestcid123?filename=Rust_Book.epub"));
    assert!(urls[1].contains("ipfs.io/ipfs/bafykbzacedtestcid123?filename=Rust_Book.epub"));
}

#[test]
fn test_parse_library_lol_html() {
    let html = r#"
    <div id="info">
        <h2><a href="https://cloudflare-ipfs.com/ipfs/bafykbzacedtest?filename=book.epub">Cloudflare IPFS</a></h2>
        <h2><a href="https://ipfs.io/ipfs/bafykbzacedtest?filename=book.epub">IPFS.io</a></h2>
        <h2><a href="https://download.library.lol/main/120000/abcdef/book.epub">GET</a></h2>
    </div>
    "#;

    let links = PartnerDownloadResolver::parse_library_lol_html(html);
    assert_eq!(links.len(), 3);
    assert!(links.contains(&"https://download.library.lol/main/120000/abcdef/book.epub".to_string()));
    assert!(links.contains(&"https://cloudflare-ipfs.com/ipfs/bafykbzacedtest?filename=book.epub".to_string()));
}

#[test]
fn test_parse_slow_download_link() {
    let html = r#"
    <div>
        <p>Your file is ready!</p>
        <a class="js-download-link" href="/slow_download_file/abcdef123456/0/0">Download now</a>
    </div>
    "#;

    let link = PartnerDownloadResolver::parse_slow_download_link(html, "annas-archive.pk");
    assert_eq!(
        link,
        Some("https://annas-archive.pk/slow_download_file/abcdef123456/0/0".to_string())
    );
}

#[test]
fn test_extract_countdown_seconds() {
    let html1 = "<div>Please wait 15 seconds before downloading...</div>";
    assert_eq!(PartnerDownloadResolver::extract_countdown_seconds(html1), Some(15));

    let html2 = "<span>Countdown: 30s</span>";
    assert_eq!(PartnerDownloadResolver::extract_countdown_seconds(html2), Some(30));
}
