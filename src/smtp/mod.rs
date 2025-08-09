//! SMTP server implementation

pub mod commands;
pub mod email;
pub mod error;
pub mod response;
pub mod server;
pub mod session;

pub use email::Email;
pub use error::{SmtpError, SmtpLimits};
pub use response::SmtpResponse;
pub use server::SmtpServer;
pub use session::{SmtpSession, SmtpState};
