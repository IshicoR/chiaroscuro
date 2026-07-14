use std::{io, net::AddrParseError};

#[derive(Debug, thiserror::Error)]
pub(crate) enum UdpError {
    #[error("failed to load settings: {0}")]
    Config(#[from] config::ConfigError),

    #[error("invalid bind_addr `{value}`: {source}")]
    InvalidBindAddr {
        value: String,
        #[source]
        source: AddrParseError,
    },

    #[error("udp socket error: {0}")]
    Io(#[from] io::Error),

    #[cfg(target_os = "windows")]
    #[error("failed to set process priority class: {0}")]
    SetPriorityClass(#[source] io::Error),
}
