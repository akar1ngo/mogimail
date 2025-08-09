//! Email data structures and functionality

use std::time::SystemTime;

/// Represents an email message received by the SMTP server
#[derive(Debug, Clone)]
pub struct Email {
    /// The sender's email address
    pub from: String,

    /// List of recipient email addresses
    pub to: Vec<String>,

    /// The email content including headers and body
    pub data: String,

    /// When the email was received by the server
    pub timestamp: SystemTime,
}

impl Email {
    /// Create a new email
    pub fn new(from: String, to: Vec<String>, data: String) -> Self {
        Self {
            from,
            to,
            data,
            timestamp: SystemTime::now(),
        }
    }

    /// Check if this email was sent to a specific recipient
    pub fn has_recipient(&self, recipient: &str) -> bool {
        self.to.iter().any(|addr| addr == recipient)
    }

    /// Check if this email was sent from a specific sender
    pub fn is_from_sender(&self, sender: &str) -> bool {
        self.from == sender
    }

    /// Get the size of the email data in bytes
    pub fn data_size(&self) -> usize {
        self.data.len()
    }

    /// Get the subject line from the email headers (if present)
    pub fn get_subject(&self) -> Option<&str> {
        for line in self.data.lines() {
            if line.is_empty() {
                // End of headers
                break;
            }
            if let Some(subject) = line.strip_prefix("Subject: ") {
                return Some(subject);
            }
            if let Some(subject) = line.strip_prefix("subject: ") {
                return Some(subject);
            }
        }
        None
    }

    /// Get the message body (content after the first empty line)
    pub fn get_body(&self) -> Option<&str> {
        let mut in_body = false;
        let mut body_start = 0;

        for (i, line) in self.data.lines().enumerate() {
            if !in_body && line.is_empty() {
                in_body = true;
                // Calculate byte offset for the body start
                body_start = self.data.lines().take(i + 1).map(|l| l.len() + 1).sum();
                break;
            }
        }

        if in_body && body_start < self.data.len() {
            Some(&self.data[body_start..])
        } else {
            None
        }
    }

    /// Check if the email contains a specific text in headers or body
    pub fn contains_text(&self, text: &str) -> bool {
        self.data.contains(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_creation() {
        let email = Email::new(
            "sender@example.com".to_string(),
            vec!["recipient@example.com".to_string()],
            "Subject: Test\n\nHello World".to_string(),
        );

        assert_eq!(email.from, "sender@example.com");
        assert_eq!(email.to, vec!["recipient@example.com"]);
        assert_eq!(email.data, "Subject: Test\n\nHello World");
        assert!(email.timestamp <= SystemTime::now());
    }

    #[test]
    fn test_has_recipient() {
        let email = Email::new(
            "sender@example.com".to_string(),
            vec![
                "user1@example.com".to_string(),
                "user2@example.com".to_string(),
            ],
            "Test email".to_string(),
        );

        assert!(email.has_recipient("user1@example.com"));
        assert!(email.has_recipient("user2@example.com"));
        assert!(!email.has_recipient("user3@example.com"));
    }

    #[test]
    fn test_is_from_sender() {
        let email = Email::new(
            "sender@example.com".to_string(),
            vec!["recipient@example.com".to_string()],
            "Test email".to_string(),
        );

        assert!(email.is_from_sender("sender@example.com"));
        assert!(!email.is_from_sender("other@example.com"));
    }

    #[test]
    fn test_get_subject() {
        let email = Email::new(
            "sender@example.com".to_string(),
            vec!["recipient@example.com".to_string()],
            "Subject: Test Email\nFrom: sender@example.com\n\nHello World".to_string(),
        );

        assert_eq!(email.get_subject(), Some("Test Email"));

        let email_no_subject = Email::new(
            "sender@example.com".to_string(),
            vec!["recipient@example.com".to_string()],
            "From: sender@example.com\n\nHello World".to_string(),
        );

        assert_eq!(email_no_subject.get_subject(), None);
    }

    #[test]
    fn test_get_body() {
        let email = Email::new(
            "sender@example.com".to_string(),
            vec!["recipient@example.com".to_string()],
            "Subject: Test\nFrom: sender@example.com\n\nHello World\nSecond line".to_string(),
        );

        assert_eq!(email.get_body(), Some("Hello World\nSecond line"));

        let email_no_body = Email::new(
            "sender@example.com".to_string(),
            vec!["recipient@example.com".to_string()],
            "Subject: Test\nFrom: sender@example.com".to_string(),
        );

        assert_eq!(email_no_body.get_body(), None);
    }

    #[test]
    fn test_contains_text() {
        let email = Email::new(
            "sender@example.com".to_string(),
            vec!["recipient@example.com".to_string()],
            "Subject: Important Message\n\nThis is a test email".to_string(),
        );

        assert!(email.contains_text("Important"));
        assert!(email.contains_text("test email"));
        assert!(!email.contains_text("not found"));
    }

    #[test]
    fn test_data_size() {
        let email = Email::new(
            "sender@example.com".to_string(),
            vec!["recipient@example.com".to_string()],
            "Hello".to_string(),
        );

        assert_eq!(email.data_size(), 5);
    }
}
