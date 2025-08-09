//! Implementation of SMTP commands

use crate::smtp::error::{SmtpError, SmtpLimits};
use crate::smtp::response::SmtpResponse;
use crate::smtp::session::SmtpSession;

/// Handles SMTP commands and returns appropriate responses
#[derive(Debug)]
pub struct SmtpCommandHandler<'a> {
    hostname: &'a str,
}

impl<'a> SmtpCommandHandler<'a> {
    /// Create a new command handler
    pub fn new(hostname: &'a str) -> Self {
        Self { hostname }
    }

    /// Process a command line and return a response
    pub fn process_command(
        &self,
        command_line: &str,
        session: &mut SmtpSession,
    ) -> Result<SmtpResponse, SmtpError> {
        // Check command line length
        if command_line.len() > SmtpLimits::COMMAND_LINE_MAX_LENGTH {
            return Err(SmtpError::LineTooLong {
                max: SmtpLimits::COMMAND_LINE_MAX_LENGTH,
            });
        }

        let parts: Vec<&str> = command_line.split_whitespace().collect();
        if parts.is_empty() {
            return Err(SmtpError::InvalidCommand);
        }

        let cmd = parts[0].to_uppercase();

        match cmd.as_str() {
            "HELO" => self.handle_helo(parts, session),
            "MAIL" => self.handle_mail(parts, session),
            "RCPT" => self.handle_rcpt(parts, session),
            "DATA" => self.handle_data(parts, session),
            "RSET" => self.handle_rset(session),
            "NOOP" => self.handle_noop(),
            "QUIT" => self.handle_quit(),
            _ => Err(SmtpError::InvalidCommand),
        }
    }

    /// Handle HELO command
    fn handle_helo(
        &self,
        parts: Vec<&str>,
        session: &mut SmtpSession,
    ) -> Result<SmtpResponse, SmtpError> {
        if parts.len() < 2 {
            return Err(SmtpError::InvalidSyntax(
                "HELO requires domain argument".to_string(),
            ));
        }

        let client_domain = parts[1].to_string();
        session.set_client_domain(client_domain.clone())?;

        Ok(SmtpResponse::helo(self.hostname, &client_domain))
    }

    /// Handle MAIL command
    fn handle_mail(
        &self,
        parts: Vec<&str>,
        session: &mut SmtpSession,
    ) -> Result<SmtpResponse, SmtpError> {
        if !session.can_execute_command("MAIL") {
            return Err(SmtpError::InvalidState(
                "MAIL command requires HELO first".to_string(),
            ));
        }

        if parts.len() < 2 {
            return Err(SmtpError::InvalidSyntax(
                "MAIL requires FROM argument".to_string(),
            ));
        }

        let from_part = parts[1..].join(" ");
        if !from_part.to_uppercase().starts_with("FROM:") {
            return Err(SmtpError::InvalidSyntax(
                "MAIL command must be 'MAIL FROM:<address>'".to_string(),
            ));
        }

        let from_addr = from_part[5..].trim();
        if !from_addr.starts_with('<') || !from_addr.ends_with('>') {
            return Err(SmtpError::InvalidSyntax(
                "FROM address must be enclosed in angle brackets".to_string(),
            ));
        }

        let addr = from_addr[1..from_addr.len() - 1].to_string();
        if addr.is_empty() {
            return Err(SmtpError::InvalidSyntax(
                "FROM address cannot be empty".to_string(),
            ));
        }

        // Validate email address components
        self.validate_email_address(&addr)?;

        session.set_sender(addr)?;

        Ok(SmtpResponse::ok())
    }

    /// Handle RCPT command
    fn handle_rcpt(
        &self,
        parts: Vec<&str>,
        session: &mut SmtpSession,
    ) -> Result<SmtpResponse, SmtpError> {
        if !session.can_execute_command("RCPT") {
            return Err(SmtpError::InvalidState(
                "RCPT command requires MAIL first".to_string(),
            ));
        }

        if parts.len() < 2 {
            return Err(SmtpError::InvalidSyntax(
                "RCPT requires TO argument".to_string(),
            ));
        }

        let to_part = parts[1..].join(" ");
        if !to_part.to_uppercase().starts_with("TO:") {
            return Err(SmtpError::InvalidSyntax(
                "RCPT command must be 'RCPT TO:<address>'".to_string(),
            ));
        }

        let to_addr = to_part[3..].trim();
        if !to_addr.starts_with('<') || !to_addr.ends_with('>') {
            return Err(SmtpError::InvalidSyntax(
                "TO address must be enclosed in angle brackets".to_string(),
            ));
        }

        let addr = to_addr[1..to_addr.len() - 1].to_string();
        if addr.is_empty() {
            return Err(SmtpError::InvalidSyntax(
                "TO address cannot be empty".to_string(),
            ));
        }

        // Validate email address components
        self.validate_email_address(&addr)?;

        session.add_recipient(addr)?;

        Ok(SmtpResponse::ok())
    }

    /// Handle DATA command
    fn handle_data(
        &self,
        parts: Vec<&str>,
        session: &mut SmtpSession,
    ) -> Result<SmtpResponse, SmtpError> {
        if !session.can_execute_command("DATA") {
            return Err(SmtpError::InvalidState(
                "DATA command requires RCPT first".to_string(),
            ));
        }

        if parts.len() > 1 {
            return Err(SmtpError::InvalidSyntax(
                "DATA command takes no arguments".to_string(),
            ));
        }

        session.start_data_mode()?;

        Ok(SmtpResponse::data_start())
    }

    /// Handle RSET command
    fn handle_rset(&self, session: &mut SmtpSession) -> Result<SmtpResponse, SmtpError> {
        if !session.can_execute_command("RSET") {
            return Err(SmtpError::InvalidState(
                "RSET command requires HELO first".to_string(),
            ));
        }

        session.reset();
        Ok(SmtpResponse::ok())
    }

    /// Handle NOOP command
    fn handle_noop(&self) -> Result<SmtpResponse, SmtpError> {
        Ok(SmtpResponse::ok())
    }

    /// Handle QUIT command
    fn handle_quit(&self) -> Result<SmtpResponse, SmtpError> {
        Ok(SmtpResponse::quit())
    }

    /// Validate email address format and size limits
    fn validate_email_address(&self, addr: &str) -> Result<(), SmtpError> {
        // Check for @ symbol
        if let Some(at_pos) = addr.find('@') {
            let user_part = &addr[..at_pos];
            let domain_part = &addr[at_pos + 1..];

            // Check user part length
            if user_part.len() > SmtpLimits::USER_MAX_LENGTH {
                return Err(SmtpError::UserTooLong {
                    max: SmtpLimits::USER_MAX_LENGTH,
                });
            }

            // Check domain part length
            if domain_part.len() > SmtpLimits::DOMAIN_MAX_LENGTH {
                return Err(SmtpError::DomainTooLong {
                    max: SmtpLimits::DOMAIN_MAX_LENGTH,
                });
            }

            // Basic validation - must have user and domain parts
            if user_part.is_empty() || domain_part.is_empty() {
                return Err(SmtpError::InvalidSyntax(
                    "Invalid email address format".to_string(),
                ));
            }
        } else {
            return Err(SmtpError::InvalidSyntax(
                "Email address must contain @ symbol".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_handler<'a>() -> SmtpCommandHandler<'a> {
        SmtpCommandHandler::new("test.local")
    }

    #[test]
    fn test_helo_command() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        let response = handler
            .process_command("HELO client.local", &mut session)
            .unwrap();

        assert_eq!(response.code, "250");
        assert_eq!(response.message, "test.local Hello client.local");
        assert_eq!(session.client_domain, Some("client.local".to_string()));
    }

    #[test]
    fn test_helo_missing_domain() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        let result = handler.process_command("HELO", &mut session);
        assert!(result.is_err());
    }

    #[test]
    fn test_mail_command() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        // First HELO
        handler
            .process_command("HELO client.local", &mut session)
            .unwrap();

        // Then MAIL
        let response = handler
            .process_command("MAIL FROM:<sender@example.com>", &mut session)
            .unwrap();

        assert_eq!(response.code, "250");
        assert_eq!(session.from, Some("sender@example.com".to_string()));
    }

    #[test]
    fn test_mail_without_helo() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        let result = handler.process_command("MAIL FROM:<sender@example.com>", &mut session);
        assert!(result.is_err());
    }

    #[test]
    fn test_mail_invalid_syntax() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        handler
            .process_command("HELO client.local", &mut session)
            .unwrap();

        let result = handler.process_command("MAIL sender@example.com", &mut session);
        assert!(result.is_err());
    }

    #[test]
    fn test_rcpt_command() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        // Setup session
        handler
            .process_command("HELO client.local", &mut session)
            .unwrap();
        handler
            .process_command("MAIL FROM:<sender@example.com>", &mut session)
            .unwrap();

        // RCPT command
        let response = handler
            .process_command("RCPT TO:<recipient@example.com>", &mut session)
            .unwrap();

        assert_eq!(response.code, "250");
        assert_eq!(session.to, vec!["recipient@example.com".to_string()]);
    }

    #[test]
    fn test_rcpt_without_mail() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        handler
            .process_command("HELO client.local", &mut session)
            .unwrap();

        let result = handler.process_command("RCPT TO:<recipient@example.com>", &mut session);
        assert!(result.is_err());
    }

    #[test]
    fn test_data_command() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        // Setup session
        handler
            .process_command("HELO client.local", &mut session)
            .unwrap();
        handler
            .process_command("MAIL FROM:<sender@example.com>", &mut session)
            .unwrap();
        handler
            .process_command("RCPT TO:<recipient@example.com>", &mut session)
            .unwrap();

        // DATA command
        let response = handler.process_command("DATA", &mut session).unwrap();

        assert_eq!(response.code, "354");
        assert!(session.in_data_mode);
    }

    #[test]
    fn test_data_without_rcpt() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        handler
            .process_command("HELO client.local", &mut session)
            .unwrap();
        handler
            .process_command("MAIL FROM:<sender@example.com>", &mut session)
            .unwrap();

        let result = handler.process_command("DATA", &mut session);
        assert!(result.is_err());
    }

    #[test]
    fn test_rset_command() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        // Setup session with transaction
        handler
            .process_command("HELO client.local", &mut session)
            .unwrap();
        handler
            .process_command("MAIL FROM:<sender@example.com>", &mut session)
            .unwrap();
        handler
            .process_command("RCPT TO:<recipient@example.com>", &mut session)
            .unwrap();

        // RSET should clear transaction
        let response = handler.process_command("RSET", &mut session).unwrap();

        assert_eq!(response.code, "250");
        assert!(session.from.is_none());
        assert!(session.to.is_empty());
    }

    #[test]
    fn test_noop_command() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        let response = handler.process_command("NOOP", &mut session).unwrap();
        assert_eq!(response.code, "250");
    }

    #[test]
    fn test_quit_command() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        let response = handler.process_command("QUIT", &mut session).unwrap();
        assert_eq!(response.code, "221");
    }

    #[test]
    fn test_invalid_command() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        let result = handler.process_command("INVALID", &mut session);
        assert!(result.is_err());
    }

    #[test]
    fn test_command_line_too_long() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        let long_command = "HELO ".to_string() + &"a".repeat(SmtpLimits::COMMAND_LINE_MAX_LENGTH);
        let result = handler.process_command(&long_command, &mut session);
        assert!(matches!(result, Err(SmtpError::LineTooLong { .. })));
    }

    #[test]
    fn test_validate_email_address() {
        let handler = create_handler();

        // Valid addresses
        assert!(handler.validate_email_address("user@example.com").is_ok());
        assert!(handler.validate_email_address("test@test.local").is_ok());

        // Invalid addresses
        assert!(handler.validate_email_address("invalid").is_err());
        assert!(handler.validate_email_address("@example.com").is_err());
        assert!(handler.validate_email_address("user@").is_err());

        // Too long user part
        let long_user = "a".repeat(SmtpLimits::USER_MAX_LENGTH + 1) + "@example.com";
        assert!(matches!(
            handler.validate_email_address(&long_user),
            Err(SmtpError::UserTooLong { .. })
        ));

        // Too long domain part
        let long_domain = "user@".to_string() + &"a".repeat(SmtpLimits::DOMAIN_MAX_LENGTH + 1);
        assert!(matches!(
            handler.validate_email_address(&long_domain),
            Err(SmtpError::DomainTooLong { .. })
        ));
    }

    #[test]
    fn test_empty_email_addresses() {
        let handler = create_handler();
        let mut session = SmtpSession::new();

        handler
            .process_command("HELO client.local", &mut session)
            .unwrap();

        // Empty FROM address
        let result = handler.process_command("MAIL FROM:<>", &mut session);
        assert!(result.is_err());

        // Empty TO address
        session
            .set_sender("sender@example.com".to_string())
            .unwrap();
        let result = handler.process_command("RCPT TO:<>", &mut session);
        assert!(result.is_err());
    }
}
