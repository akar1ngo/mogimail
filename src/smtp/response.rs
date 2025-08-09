//! SMTP response handling

/// Represents an SMTP response that can be sent to a client
#[derive(Debug, Clone)]
pub struct SmtpResponse {
    /// The SMTP response code (e.g., "250", "354", "500")
    pub code: String,
    /// The human-readable message
    pub message: String,
}

impl SmtpResponse {
    /// Create a new SMTP response
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }

    /// Create a success response (250 OK)
    pub fn ok() -> Self {
        Self::new("250", "OK")
    }

    /// Create a greeting response (220)
    pub fn greeting() -> Self {
        Self::new("220", "Welcome to MogiMail")
    }

    /// Create a HELO response (250)
    pub fn helo(hostname: &str, client_domain: &str) -> Self {
        Self::new("250", &format!("{hostname} Hello {client_domain}"))
    }

    /// Create a DATA intermediate response (354)
    pub fn data_start() -> Self {
        Self::new("354", "End data with <CR><LF>.<CR><LF>")
    }

    /// Create a QUIT response (221)
    pub fn quit() -> Self {
        Self::new("221", "Bye")
    }

    /// Create an error response from an error
    pub fn error(code: &str, message: &str) -> Self {
        Self::new(code, message)
    }

    /// Format the response for sending over the wire
    pub fn format(&self) -> String {
        format!("{} {}\r\n", self.code, self.message)
    }

    /// Check if this is a success response (2xx)
    pub fn is_success(&self) -> bool {
        self.code.starts_with('2')
    }

    /// Check if this is an error response (4xx or 5xx)
    pub fn is_error(&self) -> bool {
        self.code.starts_with('4') || self.code.starts_with('5')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_creation() {
        let response = SmtpResponse::new("250", "OK");
        assert_eq!(response.code, "250");
        assert_eq!(response.message, "OK");
    }

    #[test]
    fn test_ok_response() {
        let response = SmtpResponse::ok();
        assert_eq!(response.code, "250");
        assert_eq!(response.message, "OK");
    }

    #[test]
    fn test_greeting_response() {
        let response = SmtpResponse::greeting();
        assert_eq!(response.code, "220");
        assert_eq!(response.message, "Welcome to MogiMail");
    }

    #[test]
    fn test_helo_response() {
        let response = SmtpResponse::helo("server.local", "client.local");
        assert_eq!(response.code, "250");
        assert_eq!(response.message, "server.local Hello client.local");
    }

    #[test]
    fn test_data_start_response() {
        let response = SmtpResponse::data_start();
        assert_eq!(response.code, "354");
        assert_eq!(response.message, "End data with <CR><LF>.<CR><LF>");
    }

    #[test]
    fn test_quit_response() {
        let response = SmtpResponse::quit();
        assert_eq!(response.code, "221");
        assert_eq!(response.message, "Bye");
    }

    #[test]
    fn test_error_response() {
        let response = SmtpResponse::error("500", "Syntax error");
        assert_eq!(response.code, "500");
        assert_eq!(response.message, "Syntax error");
    }

    #[test]
    fn test_format() {
        let response = SmtpResponse::new("250", "OK");
        assert_eq!(response.format(), "250 OK\r\n");
    }

    #[test]
    fn test_is_success() {
        let success_response = SmtpResponse::new("250", "OK");
        assert!(success_response.is_success());

        let error_response = SmtpResponse::new("500", "Error");
        assert!(!error_response.is_success());
    }

    #[test]
    fn test_is_error() {
        let error_response = SmtpResponse::new("500", "Error");
        assert!(error_response.is_error());

        let client_error_response = SmtpResponse::new("421", "Service not available");
        assert!(client_error_response.is_error());

        let success_response = SmtpResponse::new("250", "OK");
        assert!(!success_response.is_error());
    }
}
