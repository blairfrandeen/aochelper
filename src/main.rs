use glob::glob;
use std::fs;
use std::path::PathBuf;

const COOKIE_GLOB: &str = "/home/*/snap/firefox/common/.mozilla/firefox/*.default/cookies.sqlite";

fn find_firefox_cookie(cookie_glob: &str) -> Result<PathBuf, String> {
    let mut gb = glob(cookie_glob).expect("Failed to read glob pattern");
    match gb.next() {
        Some(path) => Ok(PathBuf::from(path.expect("glob error"))),
        None => Err(String::from("Failed to find firefox cookie database")),
    }
}

fn main() {
    let cookie_path = find_firefox_cookie(COOKIE_GLOB);
    println!("{cookie_path:?}");
}
