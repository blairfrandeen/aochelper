use std::fs;
use std::io;
use std::path::PathBuf;

use glob::glob;
use rusqlite::{Connection, OpenFlags};

const COOKIE_GLOB: &str = "/home/*/snap/firefox/common/.mozilla/firefox/*.default/cookies.sqlite";

/// Find the firefox cookies.sqlite file.
/// This only works on linux with Firefox installed via Snap
/// Only the default profile is currently supported
fn find_firefox_cookie(cookie_glob: &str) -> Result<PathBuf, String> {
    // glob pattern is hard-coded, so single run should be enough to prove
    // that this can't fail
    let mut gb = glob(cookie_glob).expect("Failed to read glob pattern");
    match gb.next() {
        Some(path) => {
            Ok(path
                .expect("Directory matched glob pattern but could not be read; check permissions"))
        }
        None => Err(String::from("Failed to find firefox cookie database")),
    }
}

#[derive(Debug)]
enum ReadDbError {
    Io(io::Error),
    Sql(rusqlite::Error),
}

fn read_ff_host_cookie(db_path: PathBuf, hostname: &str) -> Result<String, ReadDbError> {
    // We can't read the database if Firefox is running, so we make a temporary
    // copy that allows us to open it
    let tmp_db_path = PathBuf::from("/tmp/cookies-tmp.sqlite");
    fs::copy(db_path, &tmp_db_path).map_err(ReadDbError::Io)?;

    let key: String;
    {
        // inner scope such that DB connection will be closed before temporary file is
        // deleted
        let conn = Connection::open_with_flags(
            &tmp_db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(ReadDbError::Sql)?;
        let mut query = conn
            .prepare(
                "SELECT name, value FROM moz_cookies
            WHERE host=?1",
            )
            .map_err(ReadDbError::Sql)?;
        let mut res = query.query([hostname]).map_err(ReadDbError::Sql)?;
        match res.next().map_err(ReadDbError::Sql)? {
            Some(row) => key = row.get(1).map_err(ReadDbError::Sql)?,
            None => panic!("No key found! Also need better error handling :("),
        };
    }
    fs::remove_file(tmp_db_path).map_err(ReadDbError::Io)?;
    Ok(key)
}

fn main() {
    let cookie_db_path = find_firefox_cookie(COOKIE_GLOB);
    println!("{cookie_db_path:?}");
    let cookie = read_ff_host_cookie(cookie_db_path.unwrap(), ".adventofcode.com");
    println!("{cookie:?}");
}
