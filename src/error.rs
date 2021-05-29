use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ConfigError(#[from] crate::config::Error),

    #[error(transparent)]
    ClickMuteError(#[from] crate::click_mute::Error),

    #[error("unsupported path")]
    UnsupportedPath(String), // message
}
