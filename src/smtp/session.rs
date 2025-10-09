//! SMTP session state management

use crate::smtp::email::Email;
use crate::smtp::error::{SmtpError, SmtpLimits};

/// Represents the current state of an SMTP session
#[derive(Debug, Clone, PartialEq)]
pub enum SmtpState {
    /// Initial state - waiting for HELO
    Initial,
    /// HELO received - ready for MAIL command
    GreetingReceived,
    /// MAIL FROM received - ready for RCPT commands
    MailReceived,
    /// At least one RCPT TO received - ready for DATA or more RCPT commands
    RecipientsReceived,
    /// DATA command received - collecting email data
    DataMode,
}

/// Manages the state and data for a single SMTP session
#[derive(Debug)]
pub struct SmtpSession {
    /// Current state of the session
    pub state: SmtpState,
    /// Sender address from MAIL FROM command
    pub from: Option<String>,
    /// List of recipients from RCPT TO commands
    pub to: Vec<String>,
    /// Email data lines collected during DATA mode
    pub data: Vec<String>,
    /// Whether we're currently in data collection mode
    pub in_data_mode: bool,
    /// Total size of data collected so far
    pub data_size: usize,
    /// Client domain from HELO command
    pub client_domain: Option<String>,
}

impl SmtpSession {
    /// Create a new SMTP session
    pub fn new() -> Self {
        Self {
            state: SmtpState::Initial,
            from: None,
            to: Vec::new(),
            data: Vec::new(),
            in_data_mode: false,
            data_size: 0,
            client_domain: None,
        }
    }

    /// Reset the session to post-HELO state (clears transaction data)
    pub fn reset(&mut self) {
        self.state = SmtpState::GreetingReceived;
        self.from = None;
        self.to.clear();
        self.data.clear();
        self.in_data_mode = false;
        self.data_size = 0;
        // Keep client_domain as it's set by HELO
    }

    /// Complete reset including HELO state
    pub fn full_reset(&mut self) {
        self.state = SmtpState::Initial;
        self.from = None;
        self.to.clear();
        self.data.clear();
        self.in_data_mode = false;
        self.data_size = 0;
        self.client_domain = None;
    }

    /// Set the sender address
    pub fn set_sender(&mut self, sender: String) -> Result<(), SmtpError> {
        if sender.len() > SmtpLimits::PATH_MAX_LENGTH {
            return Err(SmtpError::PathTooLong {
                max: SmtpLimits::PATH_MAX_LENGTH,
            });
        }

        self.from = Some(sender);
        self.to.clear();
        self.data.clear();
        self.data_size = 0;
        self.state = SmtpState::MailReceived;
        Ok(())
    }

    /// Add a recipient address
    pub fn add_recipient(&mut self, recipient: String) -> Result<(), SmtpError> {
        if recipient.len() > SmtpLimits::PATH_MAX_LENGTH {
            return Err(SmtpError::PathTooLong {
                max: SmtpLimits::PATH_MAX_LENGTH,
            });
        }

        if self.to.len() >= SmtpLimits::MAX_RECIPIENTS {
            return Err(SmtpError::TooManyRecipients {
                max: SmtpLimits::MAX_RECIPIENTS,
            });
        }

        self.to.push(recipient);
        self.state = SmtpState::RecipientsReceived;
        Ok(())
    }

    /// Start data collection mode
    pub fn start_data_mode(&mut self) -> Result<(), SmtpError> {
        if self.state != SmtpState::RecipientsReceived {
            return Err(SmtpError::InvalidState(
                "DATA command requires RCPT first".to_string(),
            ));
        }

        self.in_data_mode = true;
        self.data.clear();
        self.data_size = 0;
        self.state = SmtpState::DataMode;
        Ok(())
    }

    /// Add a line of data during data collection
    pub fn add_data_line(&mut self, line: String) -> Result<(), SmtpError> {
        let line_size = line.len() + 2; // +2 for CRLF

        if line_size > SmtpLimits::TEXT_LINE_MAX_LENGTH {
            return Err(SmtpError::LineTooLong {
                max: SmtpLimits::TEXT_LINE_MAX_LENGTH,
            });
        }

        if self.data_size + line_size > SmtpLimits::MAX_DATA_SIZE {
            return Err(SmtpError::TooMuchData {
                max: SmtpLimits::MAX_DATA_SIZE,
            });
        }

        self.data.push(line);
        self.data_size += line_size;
        Ok(())
    }

    /// Finish data collection and create an email
    pub fn finish_data_collection(&mut self) -> Result<Email, SmtpError> {
        if !self.in_data_mode {
            return Err(SmtpError::InvalidState(
                "Not in data collection mode".to_string(),
            ));
        }

        let from = self
            .from
            .as_ref()
            .ok_or_else(|| SmtpError::InvalidState("No sender specified".to_string()))?;

        if self.to.is_empty() {
            return Err(SmtpError::InvalidState(
                "No recipients specified".to_string(),
            ));
        }

        let email = Email::new(from.clone(), self.to.clone(), self.data.join("\n"));

        self.in_data_mode = false;
        self.state = SmtpState::GreetingReceived;
        Ok(email)
    }

    /// Set the client domain from HELO command
    pub fn set_client_domain(&mut self, domain: String) -> Result<(), SmtpError> {
        if domain.len() > SmtpLimits::DOMAIN_MAX_LENGTH {
            return Err(SmtpError::DomainTooLong {
                max: SmtpLimits::DOMAIN_MAX_LENGTH,
            });
        }

        self.client_domain = Some(domain);
        self.state = SmtpState::GreetingReceived;
        self.reset(); // Clear any existing transaction
        Ok(())
    }

    /// Check if the session is ready for a specific command
    pub fn can_execute_command(&self, command: &str) -> bool {
        match command.to_uppercase().as_str() {
            #[cfg(feature = "ehlo")]
            "EHLO" => true, // EHLO can be sent at any time
            "HELO" => true, // HELO can be sent at any time
            "MAIL" => self.state == SmtpState::GreetingReceived,
            "RCPT" => {
                self.state == SmtpState::MailReceived || self.state == SmtpState::RecipientsReceived
            }
            "DATA" => self.state == SmtpState::RecipientsReceived,
            "RSET" => self.state != SmtpState::Initial,
            "NOOP" => true, // NOOP can be sent at any time
            "QUIT" => true, // QUIT can be sent at any time
            _ => false,
        }
    }

    /// Get the current recipient count
    pub fn recipient_count(&self) -> usize {
        self.to.len()
    }

    /// Get the current data size
    pub fn current_data_size(&self) -> usize {
        self.data_size
    }

    /// Check if we have a complete transaction ready
    pub fn has_complete_transaction(&self) -> bool {
        self.from.is_some() && !self.to.is_empty() && self.state == SmtpState::RecipientsReceived
    }
}

impl Default for SmtpSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session() {
        let session = SmtpSession::new();
        assert_eq!(session.state, SmtpState::Initial);
        assert!(session.from.is_none());
        assert!(session.to.is_empty());
        assert!(session.data.is_empty());
        assert!(!session.in_data_mode);
        assert_eq!(session.data_size, 0);
        assert!(session.client_domain.is_none());
    }

    #[test]
    fn test_set_client_domain() {
        let mut session = SmtpSession::new();
        session
            .set_client_domain("client.local".to_string())
            .unwrap();

        assert_eq!(session.state, SmtpState::GreetingReceived);
        assert_eq!(session.client_domain, Some("client.local".to_string()));
    }

    #[test]
    fn test_domain_too_long() {
        let mut session = SmtpSession::new();
        let long_domain = "a".repeat(SmtpLimits::DOMAIN_MAX_LENGTH + 1);

        let result = session.set_client_domain(long_domain);
        assert!(matches!(result, Err(SmtpError::DomainTooLong { .. })));
    }

    #[test]
    fn test_set_sender() {
        let mut session = SmtpSession::new();
        session
            .set_client_domain("client.local".to_string())
            .unwrap();

        session
            .set_sender("sender@example.com".to_string())
            .unwrap();
        assert_eq!(session.from, Some("sender@example.com".to_string()));
        assert_eq!(session.state, SmtpState::MailReceived);
    }

    #[test]
    fn test_sender_path_too_long() {
        let mut session = SmtpSession::new();
        let long_path = "a".repeat(SmtpLimits::PATH_MAX_LENGTH + 1);

        let result = session.set_sender(long_path);
        assert!(matches!(result, Err(SmtpError::PathTooLong { .. })));
    }

    #[test]
    fn test_add_recipient() {
        let mut session = SmtpSession::new();
        session
            .set_client_domain("client.local".to_string())
            .unwrap();
        session
            .set_sender("sender@example.com".to_string())
            .unwrap();

        session
            .add_recipient("recipient@example.com".to_string())
            .unwrap();
        assert_eq!(session.to, vec!["recipient@example.com".to_string()]);
        assert_eq!(session.state, SmtpState::RecipientsReceived);
    }

    #[test]
    fn test_too_many_recipients() {
        let mut session = SmtpSession::new();
        session
            .set_client_domain("client.local".to_string())
            .unwrap();
        session
            .set_sender("sender@example.com".to_string())
            .unwrap();

        // Add maximum allowed recipients
        for i in 0..SmtpLimits::MAX_RECIPIENTS {
            session
                .add_recipient(format!("user{i}@example.com"))
                .unwrap();
        }

        // Try to add one more
        let result = session.add_recipient("extra@example.com".to_string());
        assert!(matches!(result, Err(SmtpError::TooManyRecipients { .. })));
    }

    #[test]
    fn test_data_collection() {
        let mut session = SmtpSession::new();
        session
            .set_client_domain("client.local".to_string())
            .unwrap();
        session
            .set_sender("sender@example.com".to_string())
            .unwrap();
        session
            .add_recipient("recipient@example.com".to_string())
            .unwrap();

        session.start_data_mode().unwrap();
        assert!(session.in_data_mode);
        assert_eq!(session.state, SmtpState::DataMode);

        session.add_data_line("Subject: Test".to_string()).unwrap();
        session.add_data_line("".to_string()).unwrap();
        session.add_data_line("Test body".to_string()).unwrap();

        let email = session.finish_data_collection().unwrap();
        assert_eq!(email.from, "sender@example.com");
        assert_eq!(email.to, vec!["recipient@example.com"]);
        assert_eq!(email.data, "Subject: Test\n\nTest body");
        assert!(!session.in_data_mode);
    }

    #[test]
    fn test_line_too_long() {
        let mut session = SmtpSession::new();
        session
            .set_client_domain("client.local".to_string())
            .unwrap();
        session
            .set_sender("sender@example.com".to_string())
            .unwrap();
        session
            .add_recipient("recipient@example.com".to_string())
            .unwrap();
        session.start_data_mode().unwrap();

        let long_line = "a".repeat(SmtpLimits::TEXT_LINE_MAX_LENGTH + 1);
        let result = session.add_data_line(long_line);
        assert!(matches!(result, Err(SmtpError::LineTooLong { .. })));
    }

    #[test]
    fn test_can_execute_command() {
        let mut session = SmtpSession::new();

        // Initial state
        assert!(session.can_execute_command("HELO"));
        assert!(session.can_execute_command("NOOP"));
        assert!(session.can_execute_command("QUIT"));
        assert!(!session.can_execute_command("MAIL"));
        assert!(!session.can_execute_command("RCPT"));
        assert!(!session.can_execute_command("DATA"));
        assert!(!session.can_execute_command("RSET"));

        // After HELO
        session
            .set_client_domain("client.local".to_string())
            .unwrap();
        assert!(session.can_execute_command("MAIL"));
        assert!(session.can_execute_command("RSET"));
        assert!(!session.can_execute_command("RCPT"));
        assert!(!session.can_execute_command("DATA"));

        // After MAIL
        session
            .set_sender("sender@example.com".to_string())
            .unwrap();
        assert!(session.can_execute_command("RCPT"));
        assert!(!session.can_execute_command("DATA"));

        // After RCPT
        session
            .add_recipient("recipient@example.com".to_string())
            .unwrap();
        assert!(session.can_execute_command("DATA"));
        assert!(session.can_execute_command("RCPT")); // Can add more recipients
    }

    #[test]
    fn test_reset() {
        let mut session = SmtpSession::new();
        session
            .set_client_domain("client.local".to_string())
            .unwrap();
        session
            .set_sender("sender@example.com".to_string())
            .unwrap();
        session
            .add_recipient("recipient@example.com".to_string())
            .unwrap();

        session.reset();

        assert_eq!(session.state, SmtpState::GreetingReceived);
        assert!(session.from.is_none());
        assert!(session.to.is_empty());
        assert!(session.data.is_empty());
        assert!(!session.in_data_mode);
        assert_eq!(session.data_size, 0);
        // Should keep client domain
        assert_eq!(session.client_domain, Some("client.local".to_string()));
    }

    #[test]
    fn test_full_reset() {
        let mut session = SmtpSession::new();
        session
            .set_client_domain("client.local".to_string())
            .unwrap();
        session
            .set_sender("sender@example.com".to_string())
            .unwrap();

        session.full_reset();

        assert_eq!(session.state, SmtpState::Initial);
        assert!(session.from.is_none());
        assert!(session.to.is_empty());
        assert!(session.data.is_empty());
        assert!(!session.in_data_mode);
        assert_eq!(session.data_size, 0);
        assert!(session.client_domain.is_none());
    }

    #[test]
    fn test_has_complete_transaction() {
        let mut session = SmtpSession::new();
        assert!(!session.has_complete_transaction());

        session
            .set_client_domain("client.local".to_string())
            .unwrap();
        assert!(!session.has_complete_transaction());

        session
            .set_sender("sender@example.com".to_string())
            .unwrap();
        assert!(!session.has_complete_transaction());

        session
            .add_recipient("recipient@example.com".to_string())
            .unwrap();
        assert!(session.has_complete_transaction());
    }
}
