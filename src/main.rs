use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use env_logger;
use glob::glob;
use log;
use reqwest::blocking::Client;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

const COOKIE_GLOB: &str = "/home/*/snap/firefox/common/.mozilla/firefox/*.default/cookies.sqlite";
const CONFIG_FILE: &str = "aochelper.toml";
// TODO: Use date functions to determine max year
const MAX_YEAR: u16 = 2023;

/// Find the firefox cookies.sqlite file.
/// This only works on linux with Firefox installed via Snap
/// Only the default profile is currently supported
fn find_firefox_cookie(cookie_glob: &str) -> Result<PathBuf> {
    // glob pattern is hard-coded, so single run should be enough to prove
    // that this can't fail
    let mut gb = glob(cookie_glob).expect("Failed to read glob pattern");
    match gb.next() {
        Some(path) => Ok(path.expect("Error with file path")),
        None => Err(anyhow::anyhow!(
            "Could not find Firefox cookies. No matches for {cookie_glob}."
        )),
    }
}

fn read_ff_host_cookie(db_path: &PathBuf, hostname: &str) -> Result<String> {
    // We can't read the database if Firefox is running, so we make a temporary
    // copy that allows us to open it
    let tmp_db_path = PathBuf::from("/tmp/cookies-tmp.sqlite");
    fs::copy(db_path, &tmp_db_path)
        .with_context(|| format!("Failed to copy from {:?} to {:?}", &db_path, &tmp_db_path))?;

    let key: String;
    {
        // inner scope such that DB connection will be closed before temporary file is
        // deleted
        let conn = Connection::open_with_flags(
            &tmp_db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("Failed to open database connection to {:?}.", &tmp_db_path))?;
        let mut query = conn
            .prepare(
                "SELECT name, value FROM moz_cookies
            WHERE host=?1",
            )
            .with_context(|| format!("Error with SQLite database connection {:?}.", &conn))?;
        let mut res = query
            .query([hostname])
            .expect("Error with sqlite query execution");
        match res.next()? {
            Some(row) => key = row.get(1)?,
            None => return Err(anyhow::anyhow!(
                    "No cookie found for '{hostname}'. You may need to log in via the web browswer first."
                    )),
        };
    }
    match fs::remove_file(&tmp_db_path) {
        Ok(_) => {}
        Err(err) => println!("Warning: Unable to remove {:?}: {:?}", &tmp_db_path, err),
    }
    Ok(key)
}

fn get_puzzle_input(puzzle_url: String, cookie: &str) -> Result<String> {
    log::debug!("Querying puzzle input from {puzzle_url}");
    let client = Client::new();
    let mut res = client
        .get(&puzzle_url)
        .header("cookie", format!("session={cookie}"))
        .send()?;
    let mut body = String::new();
    res.read_to_string(&mut body)?;

    match res.status() {
        reqwest::StatusCode::OK => Ok(body),
        reqwest::StatusCode::NOT_FOUND => Err(anyhow::anyhow!(
            "Puzzle input for {} not found.",
            &puzzle_url
        )),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR => Err(anyhow::anyhow!("Invalid session key supplied. You may need to log into adventofcode.com with your browser again.")),
        _  => Err(anyhow::anyhow!(
            "Error getting puzzle input: {}\n{body}",
            res.status()
        )),
    }
}

fn build_puzzle_url(year: u16, day: u8) -> Result<String> {
    if year < 2015 || year > MAX_YEAR {
        Err(anyhow::anyhow!("Invalid year: {year}"))
    } else if day > 25 || day < 1 {
        Err(anyhow::anyhow!("Invalid day: {day}"))
    } else {
        Ok(format!("https://adventofcode.com/{year}/day/{day}/input"))
    }
}

/// Tool to download Advent of Code puzzle inputs
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Set a configuration variable. An optional, local per-folder configuration is
    /// stored in 'aochelper.toml'.
    ///
    /// The following variables that can be set with this command:
    ///
    ///     year:           Year to download puzzle inputs from
    ///
    ///     session_key:    Session cookie, which can be pulled from your browser's
    ///                     cookie database, or by inspecting a GET request while logged
    ///                     into adventofcode.com
    ///
    ///     output_path:    Folder where puzzle inputs will be downloaded to.
    Set { key: String, value: String },

    /// Get puzzle input for a given day.
    Get {
        day: u8,

        /// Puzzle year if not supplied in aochelper.toml
        #[clap(short, long, value_name = "YEAR")]
        year: Option<u16>,

        /// Directory to which to write inputs
        #[clap(short, long, value_name = "OUTPUT")]
        output: Option<PathBuf>,

        /// Session key, typically read from browser cookie
        #[clap(short, long, value_name = "SESSION_KEY")]
        session_key: Option<String>,
    },
}

#[derive(Deserialize, Serialize, Debug)]
struct Config {
    year: Option<u16>,
    session_key: Option<String>,
    output_path: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            year: None,
            session_key: None,
            output_path: None,
        }
    }
}

fn read_config(config_path: PathBuf) -> Result<Config> {
    if config_path.exists() {
        let mut config_file = fs::File::open(&config_path)?;
        let mut config_buf = String::new();
        config_file.read_to_string(&mut config_buf)?;
        let config: Config = toml::from_str(&config_buf)?;
        log::debug!("Read configuration file from {:?}", config_file);
        Ok(config)
    } else {
        Ok(Config::default())
    }
}

fn set_config_option(key: &str, value: &str) -> Result<()> {
    let mut config = read_config(PathBuf::from(CONFIG_FILE))?;

    match key {
        "year" => config.year = Some(value.parse::<u16>()?),
        "session_key" => config.session_key = Some(value.to_string()),
        "output_path" => config.output_path = Some(PathBuf::from(value)),
        _ => return Err(anyhow::anyhow!("Invalid key specified!")),
    }

    let config_toml = toml::to_string(&config)?;
    let mut config_file = fs::File::create(PathBuf::from(CONFIG_FILE))?;
    config_file.write_all(config_toml.as_bytes())?;
    log::debug!("Updated local config file: {:?}", config_file);
    log::debug!("Set {} = {}", key, value);

    Ok(())
}

fn get_cmd(
    day: &u8,
    year: &Option<u16>,
    output: &Option<PathBuf>,
    session_key: &Option<String>,
) -> Result<()> {
    let config = read_config(PathBuf::from(CONFIG_FILE))?;
    let cmd_year = match year {
        Some(yr) => *yr,
        None => match &config.year {
            Some(yr) => {
                log::debug!("Found year = {} from local config", yr);
                *yr
            }
            None => {
                return Err(anyhow::anyhow!(
                    "No year specified. You can re-run this command with the \
                     --year=<year> flag, or run `aochelper set year <year>` to permanently set it."
                ))
            }
        },
    };

    let cmd_session_key = match session_key {
        Some(key) => key.clone(),
        None => match config.session_key {
            Some(key) => {
                log::debug!("Found session key from local config");
                key
            }
            None => {
                log::debug!("No session key found in local config, attempting to read from browser cookie store");
                let cookie_db_path = find_firefox_cookie(COOKIE_GLOB)?;
                log::debug!("Found Firefox cookies at {cookie_db_path:?}");
                let key = read_ff_host_cookie(&cookie_db_path, ".adventofcode.com").with_context(
                    || format!("Failed to read firefox cookies from {:?}", &cookie_db_path),
                )?;
                log::debug!("Found cookie for advent of code from Firefox.");
                key
            }
        },
    };
    let puzzle_url = build_puzzle_url(cmd_year, *day)?;
    let response = get_puzzle_input(puzzle_url, &cmd_session_key)?;

    let mut input_path = match output {
        Some(dir) => dir.clone(),
        None => match config.output_path {
            Some(dir) => dir,
            None => PathBuf::from("inputs"),
        },
    };
    fs::create_dir_all(&input_path)?;
    input_path.push(format!("{}.{:02}", cmd_year, day));
    log::info!("Successfully wrote to {}", &input_path.display());
    let mut puzzle_file = fs::File::create(input_path)?;
    puzzle_file.write_all(response.as_bytes())?;

    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Cli::parse();
    match &args.command {
        Commands::Set { key, value } => {
            set_config_option(key, value)?;
        }
        Commands::Get {
            day,
            year,
            output,
            session_key,
        } => {
            get_cmd(day, year, output, session_key)?;
        }
    };

    Ok(())
}
