//! Error types for the SMTP server

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SmtpError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid command")]
    InvalidCommand,

    #[error("Invalid state for command")]
    InvalidState(String),

    #[error("Invalid syntax")]
    InvalidSyntax(String),

    #[error("Line too long (max {max} characters)")]
    LineTooLong { max: usize },

    #[error("Path too long (max {max} characters)")]
    PathTooLong { max: usize },

    #[error("Too many recipients (max {max})")]
    TooManyRecipients { max: usize },

    #[error("Too much mail data (max {max} bytes)")]
    TooMuchData { max: usize },

    #[error("Domain name too long (max {max} characters)")]
    DomainTooLong { max: usize },

    #[error("User name too long (max {max} characters)")]
    UserTooLong { max: usize },

    #[error("Non-UTF-8 data encountered")]
    NonUtf8Data,

    #[error("Connection closed unexpectedly")]
    ConnectionClosed,

    #[error("Protocol violation")]
    ProtocolViolation,
}

/// SMTP size limits as defined in RFC 821
pub struct SmtpLimits;

impl SmtpLimits {
    /// Maximum length of a user name
    pub const USER_MAX_LENGTH: usize = 64;

    /// Maximum length of a domain name
    pub const DOMAIN_MAX_LENGTH: usize = 64;

    /// Maximum length of a path (reverse-path or forward-path)
    pub const PATH_MAX_LENGTH: usize = 256;

    /// Maximum length of a command line including CRLF
    pub const COMMAND_LINE_MAX_LENGTH: usize = 512;

    /// Maximum length of a reply line including CRLF
    pub const REPLY_LINE_MAX_LENGTH: usize = 512;

    /// Maximum length of a text line including CRLF
    pub const TEXT_LINE_MAX_LENGTH: usize = 1000;

    /// Maximum number of recipients per message
    pub const MAX_RECIPIENTS: usize = 100;

    /// Maximum total size of email data (reasonable limit for in-memory storage)
    pub const MAX_DATA_SIZE: usize = 10 * 1024 * 1024; // 10MB
}

/// Maps SMTP errors to appropriate response codes
impl SmtpError {
    pub fn to_response_code(&self) -> &'static str {
        match self {
            SmtpError::Io(_) => "421",
            SmtpError::InvalidCommand => "500",
            SmtpError::InvalidState(_) => "503",
            SmtpError::InvalidSyntax(_) => "501",
            SmtpError::LineTooLong { .. } => "500",
            SmtpError::PathTooLong { .. } => "501",
            SmtpError::TooManyRecipients { .. } => "552",
            SmtpError::TooMuchData { .. } => "552",
            SmtpError::DomainTooLong { .. } => "501",
            SmtpError::UserTooLong { .. } => "501",
            SmtpError::NonUtf8Data => "500",
            SmtpError::ConnectionClosed => "421",
            SmtpError::ProtocolViolation => "500",
        }
    }

    pub fn to_response_message(&self) -> String {
        match self {
            SmtpError::Io(_) => "Service not available".to_string(),
            SmtpError::InvalidCommand => "Syntax error, command unrecognized".to_string(),
            SmtpError::InvalidState(msg) => format!("Bad sequence of commands: {msg}"),
            SmtpError::InvalidSyntax(msg) => format!("Syntax error: {msg}"),
            SmtpError::LineTooLong { max } => format!("Line too long (max {max} characters)"),
            SmtpError::PathTooLong { max } => format!("Path too long (max {max} characters)"),
            SmtpError::TooManyRecipients { max } => format!("Too many recipients (max {max})"),
            SmtpError::TooMuchData { max } => format!("Too much mail data (max {max} bytes)"),
            SmtpError::DomainTooLong { max } => {
                format!("Domain name too long (max {max} characters)")
            }
            SmtpError::UserTooLong { max } => {
                format!("User name too long (max {max} characters)")
            }
            SmtpError::NonUtf8Data => "Invalid character encoding".to_string(),
            SmtpError::ConnectionClosed => "Connection closed".to_string(),
            SmtpError::ProtocolViolation => "Protocol violation".to_string(),
        }
    }
}
