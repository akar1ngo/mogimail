//! # MogiMail
//!
//! MogiMail is an embedded SMTP server for testing.
//!
//! It enables testing of email code without using mocks.
//!
//! ## Quick Start
//!
//! ```rust
//! use mogimail::SmtpServer;
//! use std::sync::mpsc;
//! use std::thread;
//! use std::time::Duration;
//!
//! // Create and start server
//! let (tx, rx) = mpsc::channel();
//! let server = SmtpServer::new("test.local");
//!
//! thread::spawn(move || {
//!     server.start("127.0.0.1:2525", tx).unwrap();
//! });
//!
//! // Application sends email to localhost:2525
//! // ...
//!
//! // Check the contents of the sent email
//! if let Ok(email) = rx.recv_timeout(Duration::from_millis(100)) {
//!   println!("Received email from: {}", email.from);
//! }
//! ```
//!
//! ## Supported SMTP commands
//!
//! - `HELO` - Identify the sender
//! - `MAIL FROM` - Specify the sender's address
//! - `RCPT TO` - Specify the destination (multiple destinations are supported)
//! - `DATA` - Send the email body
//! - `RSET` - Reset the current transaction
//! - `NOOP` - Do nothing
//! - `QUIT` - Close connection
//!
//! ## Additional Features
//!
//! Enabling the `ehlo` feature also allows you to use the `EHLO` command.
//!
//! ## Notes
//!
//! - Only the "minimal implementation" defined in RFC 821 is implemented.
//! - Runs in-memory only. Email persistence is not supported.
//! - SMTP authentication is not supported.
//! - SSL/TLS connection is not supported.
//! - Mail relay is not supported.
//!
//! ## Size Limits
//!
//! The server enforces RFC 821 size limits:
//! - User names: 64 characters max
//! - Domain names: 64 characters max
//! - Paths: 256 characters max
//! - Command lines: 512 characters max
//! - Text lines: 1000 characters max
//! - Recipients: 100 max per message
//!
//! ## Email Handling
//!
//! Emails are sent directly to the channel provided when starting the server.
//!
//! Use `recv_timeout()` on the receiver to wait for emails with a timeout.
//! This avoids the need for `thread::sleep()` and provides deterministic
//! behavior when testing email functionality.

mod smtp;

pub use smtp::{Email, SmtpError, SmtpLimits, SmtpResponse, SmtpServer, SmtpSession, SmtpState};
