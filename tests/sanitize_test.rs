use annas_mcp::downloader::sanitize::{make_safe_filepath, sanitize_filename};
use std::path::Path;

#[test]
fn test_sanitize_filename() {
    assert_eq!(sanitize_filename("Valid_Filename"), "Valid_Filename");
    assert_eq!(
        sanitize_filename("Book: Subtitle / Part 1 *special?*"),
        "Book_ Subtitle _ Part 1 _special"
    );
    assert_eq!(sanitize_filename("../../../etc/passwd"), "etc_passwd");
    assert_eq!(sanitize_filename("..."), "untitled");
    assert_eq!(sanitize_filename(""), "untitled");
}

#[test]
fn test_make_safe_filepath() {
    let folder = Path::new("/downloads");
    let path = make_safe_filepath(folder, "My Great Book: Volume 1", "epub");
    assert_eq!(path, Path::new("/downloads/My Great Book_ Volume 1.epub"));

    let path_with_dot = make_safe_filepath(folder, "Another Book", ".pdf");
    assert_eq!(path_with_dot, Path::new("/downloads/Another Book.pdf"));
}
