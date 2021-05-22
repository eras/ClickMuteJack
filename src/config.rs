use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::io;
use std::io::Write;
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Delays {
    pub mute_offset: f64,
    pub mute_duration: f64,
    pub fade: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Config {
    pub delays: Delays,
}

#[derive(Error, Debug)]
pub struct ParseError {
    pub filename: String,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to parse {}: {}", self.filename, self.message)
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ParseError(ParseError),

    #[error(transparent)]
    TomlDeError(#[from] toml::de::Error),

    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),

    #[error(transparent)]
    IOError(#[from] io::Error),

    #[error(transparent)]
    AtomicIOError(#[from] atomicwrites::Error<io::Error>),
}

pub static FILENAME: &str = "click_mute.ini";

impl Config {
    pub fn new() -> Config {
        let delays = Delays {
            mute_offset: -0.04,
            mute_duration: 0.08,
            fade: 0.01,
        };
        Config { delays }
    }

    // If no file is found, returns default config instead of error
    pub fn load(filename: &str) -> Result<Config, Error> {
        let contents = match fs::read_to_string(filename) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Config::new()),
            Err(error) => return Err(Error::IOError(error)),
        };
        let config = match toml::from_str(&contents) {
            Ok(contents) => contents,
            Err(error) if error.line_col().is_some() => {
                return Err(Error::ParseError(ParseError {
                    filename: String::from(filename),
                    message: format!("{}", error),
                }));
            }
            Err(error) => return Err(Error::TomlDeError(error)),
        };
        println!("Loaded config from {}", filename);
        Ok(config)
    }

    pub fn save(self, filename: &str) -> Result<(), Error> {
        let contents = toml::to_string(&self)?;
        let writer = atomicwrites::AtomicFile::new(filename, atomicwrites::AllowOverwrite);
        writer.write(|f| f.write_all(contents.as_bytes()))?;
        Ok(())
    }
}
