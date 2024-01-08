use std::fs;
use std::path::PathBuf;

use glob::glob;
use rusqlite::{Connection, OpenFlags, Result};

const COOKIE_GLOB: &str = "/home/*/snap/firefox/common/.mozilla/firefox/*.default/cookies.sqlite";

fn find_firefox_cookie(cookie_glob: &str) -> Result<PathBuf, String> {
    let mut gb = glob(cookie_glob).expect("Failed to read glob pattern");
    match gb.next() {
        Some(path) => Ok(PathBuf::from(path.expect("glob error"))),
        None => Err(String::from("Failed to find firefox cookie database")),
    }
}

fn read_cookie(db_path: PathBuf) -> Result<String> {
    let tmp_db_path = PathBuf::from("/tmp/cookies-tmp.sqlite");
    // TODO: Error handing for fs operations
    fs::copy(db_path, &tmp_db_path);
    let conn = Connection::open_with_flags(
        &tmp_db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let mut query =
        conn.prepare("SELECT name, value FROM moz_cookies WHERE host='.adventofcode.com'")?;
    let mut res = query.query([])?;
    // TODO: Error handing for fs operations
    fs::remove_file(tmp_db_path);
    let mut key = String::new();
    match res.next()? {
        Some(row) => key = row.get(1)?,
        None => panic!("No key found! Also need better error handling :("),
    };
    Ok(key)
}

fn main() {
    let cookie_db_path = find_firefox_cookie(COOKIE_GLOB);
    println!("{cookie_db_path:?}");
    let cookie = read_cookie(cookie_db_path.unwrap());
    println!("{cookie:?}");
}
