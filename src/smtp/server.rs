//! SMTP server implementation

use crate::smtp::commands::SmtpCommandHandler;
use crate::smtp::email::Email;
use crate::smtp::error::{SmtpError, SmtpLimits};
use crate::smtp::response::SmtpResponse;
use crate::smtp::session::SmtpSession;

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;

/// Main SMTP server that handles connections and sends emails to a channel
#[derive(Debug, Clone)]
pub struct SmtpServer {
    /// Server hostname
    hostname: String,
}

impl SmtpServer {
    /// Create a new SMTP server
    pub fn new(hostname: &str) -> Self {
        Self {
            hostname: hostname.to_owned(),
        }
    }

    /// Start the server on the specified address (blocking)
    /// Emails will be sent to the provided channel as they are received
    pub fn start(&self, addr: &str, email_sender: mpsc::Sender<Email>) -> Result<(), SmtpError> {
        let listener = TcpListener::bind(addr)?;
        println!("SMTP server listening on {addr}");

        let command_handler = SmtpCommandHandler::new(&self.hostname);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_client(stream, &command_handler, &email_sender) {
                        eprintln!("Error handling client: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {e}");
                }
            }
        }

        Ok(())
    }

    /// Start the server with an existing listener (blocking)
    /// Emails will be sent to the provided channel as they are received
    pub fn start_with_listener(
        &self,
        listener: TcpListener,
        email_sender: mpsc::Sender<Email>,
    ) -> Result<(), SmtpError> {
        println!(
            "SMTP server listening on {}",
            listener.local_addr().map_err(SmtpError::Io)?
        );

        let command_handler = SmtpCommandHandler::new(&self.hostname);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_client(stream, &command_handler, &email_sender) {
                        eprintln!("Error handling client: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {e}");
                }
            }
        }

        Ok(())
    }

    /// Handle a client connection
    fn handle_client(
        &self,
        mut stream: TcpStream,
        command_handler: &SmtpCommandHandler,
        email_sender: &mpsc::Sender<Email>,
    ) -> Result<(), SmtpError> {
        let mut session = SmtpSession::new();
        let mut reader = BufReader::new(stream.try_clone()?);

        // Send greeting
        self.send_response(&mut stream, &SmtpResponse::greeting())?;

        let mut line_buffer = Vec::new();
        loop {
            line_buffer.clear();

            // Read line with UTF-8 safety
            match reader.read_until(b'\n', &mut line_buffer) {
                Ok(0) => break, // Connection closed
                Ok(_) => {
                    // Handle potential UTF-8 issues gracefully
                    let line = match String::from_utf8(line_buffer.clone()) {
                        Ok(s) => s,
                        Err(_) => {
                            // Replace invalid UTF-8 sequences with replacement character
                            String::from_utf8_lossy(&line_buffer).into_owned()
                        }
                    };

                    let command = line.trim();
                    if command.is_empty() {
                        continue;
                    }

                    // Handle data mode specially
                    if session.in_data_mode {
                        match self.handle_data_line(command, &mut session) {
                            Ok(Some(response)) => {
                                self.send_response(&mut stream, &response)?;
                                if response.code == "250" {
                                    // Email stored successfully
                                    if let Ok(email) = session.finish_data_collection() {
                                        // Errors when there are no listeners.
                                        // We ignore these errors for now.
                                        let _ = email_sender.send(email);
                                    }
                                    session.reset();
                                } else {
                                    // Reset on error
                                    session.reset();
                                }
                            }
                            Ok(None) => {
                                // Continue collecting data
                            }
                            Err(e) => {
                                let response = SmtpResponse::error(
                                    e.to_response_code(),
                                    &e.to_response_message(),
                                );
                                self.send_response(&mut stream, &response)?;
                                session.reset();
                            }
                        }
                    } else {
                        // Normal command processing
                        match command_handler.process_command(command, &mut session) {
                            Ok(response) => {
                                self.send_response(&mut stream, &response)?;
                                if response.code == "221" {
                                    break; // QUIT command
                                }
                            }
                            Err(e) => {
                                let response = SmtpResponse::error(
                                    e.to_response_code(),
                                    &e.to_response_message(),
                                );
                                self.send_response(&mut stream, &response)?;

                                // Don't automatically reset on all 5xx errors
                                // Let the command handler manage session state
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from client: {e}");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a line of data during DATA mode
    fn handle_data_line(
        &self,
        line: &str,
        session: &mut SmtpSession,
    ) -> Result<Option<SmtpResponse>, SmtpError> {
        if line == "." {
            // End of data
            Ok(Some(SmtpResponse::ok()))
        } else {
            // Add data line
            session.add_data_line(line.to_string())?;
            Ok(None)
        }
    }

    /// Send a response to the client
    fn send_response(
        &self,
        stream: &mut TcpStream,
        response: &SmtpResponse,
    ) -> Result<(), SmtpError> {
        // Ensure response doesn't exceed maximum line length
        let formatted = response.format();
        if formatted.len() > SmtpLimits::REPLY_LINE_MAX_LENGTH {
            // Truncate message if too long
            let truncated_response =
                SmtpResponse::new(&response.code, "Response too long (truncated)");
            stream.write_all(truncated_response.format().as_bytes())?;
        } else {
            stream.write_all(formatted.as_bytes())?;
        }
        stream.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    fn start_test_server() -> (String, mpsc::Receiver<Email>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server = SmtpServer::new("test.local");
        let (tx, rx) = mpsc::channel();

        // Start server in background thread
        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let command_handler = SmtpCommandHandler::new("test.local");
                        if let Err(e) = server.handle_client(stream, &command_handler, &tx) {
                            eprintln!("Error handling client: {e}");
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        (addr, rx)
    }

    fn send_command(stream: &mut TcpStream, command: &str) -> Result<String, std::io::Error> {
        writeln!(stream, "{command}")?;
        stream.flush()?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;
        Ok(response.trim().to_string())
    }

    #[test]
    fn test_server_creation() {
        let server = SmtpServer::new("test.local");
        assert_eq!(server.hostname, "test.local");
    }

    #[test]
    fn test_complete_smtp_session() {
        let (addr, rx) = start_test_server();

        // Connect to server
        let mut stream = TcpStream::connect(&addr).unwrap();

        // Read greeting
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut greeting = String::new();
        reader.read_line(&mut greeting).unwrap();
        assert!(greeting.starts_with("220"));

        // Send HELO
        let response = send_command(&mut stream, "HELO client.local").unwrap();
        assert!(response.starts_with("250"));

        // Send MAIL FROM
        let response = send_command(&mut stream, "MAIL FROM:<test@example.com>").unwrap();
        assert!(response.starts_with("250"));

        // Send RCPT TO
        let response = send_command(&mut stream, "RCPT TO:<recipient@example.com>").unwrap();
        assert!(response.starts_with("250"));

        // Send DATA
        let response = send_command(&mut stream, "DATA").unwrap();
        assert!(response.starts_with("354"));

        // Send email content
        writeln!(stream, "Subject: Test Email").unwrap();
        writeln!(stream).unwrap();
        writeln!(stream, "This is a test email.").unwrap();
        writeln!(stream, ".").unwrap();
        stream.flush().unwrap();

        // Read final response
        let mut final_response = String::new();
        reader.read_line(&mut final_response).unwrap();
        assert!(final_response.starts_with("250"));

        // Send QUIT
        let response = send_command(&mut stream, "QUIT").unwrap();
        assert!(response.starts_with("221"));

        // Wait for email to be processed
        let email = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(email.from, "test@example.com");
        assert_eq!(email.to, vec!["recipient@example.com"]);
        assert!(email.data.contains("Subject: Test Email"));
        assert!(email.data.contains("This is a test email."));
    }

    #[test]
    fn test_error_handling() {
        let (addr, _rx) = start_test_server();

        // Connect to server
        let mut stream = TcpStream::connect(&addr).unwrap();

        // Read greeting
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut greeting = String::new();
        reader.read_line(&mut greeting).unwrap();
        assert!(greeting.starts_with("220"));

        // Send invalid command
        let response = send_command(&mut stream, "INVALID").unwrap();
        assert!(response.starts_with("500"));

        // Try MAIL without HELO
        let response = send_command(&mut stream, "MAIL FROM:<test@example.com>").unwrap();
        assert!(response.starts_with("503") || response.starts_with("500"));

        // Send QUIT
        let response = send_command(&mut stream, "QUIT").unwrap();
        assert!(response.starts_with("221"));
    }

    #[test]
    fn test_multiple_recipients() {
        let (addr, rx) = start_test_server();

        // Connect to server
        let mut stream = TcpStream::connect(&addr).unwrap();

        // Read greeting
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut greeting = String::new();
        reader.read_line(&mut greeting).unwrap();

        // Complete SMTP session with multiple recipients
        send_command(&mut stream, "HELO client.local").unwrap();
        send_command(&mut stream, "MAIL FROM:<sender@example.com>").unwrap();
        send_command(&mut stream, "RCPT TO:<recipient1@example.com>").unwrap();
        send_command(&mut stream, "RCPT TO:<recipient2@example.com>").unwrap();
        send_command(&mut stream, "DATA").unwrap();

        writeln!(stream, "Subject: Multiple Recipients").unwrap();
        writeln!(stream).unwrap();
        writeln!(stream, "Test message for multiple recipients").unwrap();
        writeln!(stream, ".").unwrap();
        stream.flush().unwrap();

        // Read response
        let mut response = String::new();
        reader.read_line(&mut response).unwrap();
        assert!(response.starts_with("250"));

        send_command(&mut stream, "QUIT").unwrap();

        // Wait for email to be processed
        let email = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(email.to.len(), 2);
        assert!(email.to.contains(&"recipient1@example.com".to_string()));
        assert!(email.to.contains(&"recipient2@example.com".to_string()));
    }

    #[test]
    fn test_rset_command() {
        let (addr, rx) = start_test_server();

        // Connect to server
        let mut stream = TcpStream::connect(&addr).unwrap();

        // Read greeting
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut greeting = String::new();
        reader.read_line(&mut greeting).unwrap();

        // Start transaction
        send_command(&mut stream, "HELO client.local").unwrap();
        send_command(&mut stream, "MAIL FROM:<sender@example.com>").unwrap();
        send_command(&mut stream, "RCPT TO:<recipient@example.com>").unwrap();

        // Reset transaction
        let response = send_command(&mut stream, "RSET").unwrap();
        assert!(response.starts_with("250"));

        // Should be able to start new transaction
        send_command(&mut stream, "MAIL FROM:<newsender@example.com>").unwrap();
        send_command(&mut stream, "RCPT TO:<newrecipient@example.com>").unwrap();
        send_command(&mut stream, "DATA").unwrap();

        writeln!(stream, "Subject: After Reset").unwrap();
        writeln!(stream).unwrap();
        writeln!(stream, "This message came after RSET").unwrap();
        writeln!(stream, ".").unwrap();
        stream.flush().unwrap();

        // Read response
        let mut response = String::new();
        reader.read_line(&mut response).unwrap();
        assert!(response.starts_with("250"));

        send_command(&mut stream, "QUIT").unwrap();

        // Should only receive the email after RSET
        let email = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(email.from, "newsender@example.com");
        assert_eq!(email.to, vec!["newrecipient@example.com"]);

        // Should not receive any more emails
        assert!(rx.recv_timeout(Duration::from_millis(50)).is_err());
    }

    #[cfg(feature = "ehlo")]
    #[test]
    fn test_ehlo_command() {
        let (addr, rx) = start_test_server();

        // Connect to server
        let mut stream = TcpStream::connect(&addr).unwrap();

        // Read greeting
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut greeting = String::new();
        reader.read_line(&mut greeting).unwrap();
        assert!(greeting.starts_with("220"));

        // Send EHLO
        let response = send_command(&mut stream, "EHLO client.local").unwrap();
        assert!(response.starts_with("250"));

        // Send MAIL FROM
        let response = send_command(&mut stream, "MAIL FROM:<test@example.com>").unwrap();
        assert!(response.starts_with("250"));

        // Send RCPT TO
        let response = send_command(&mut stream, "RCPT TO:<recipient@example.com>").unwrap();
        assert!(response.starts_with("250"));

        // Send DATA
        let response = send_command(&mut stream, "DATA").unwrap();
        assert!(response.starts_with("354"));

        // Send email content
        writeln!(stream, "Subject: EHLO Test Email").unwrap();
        writeln!(stream).unwrap();
        writeln!(stream, "This is a test.").unwrap();
        writeln!(stream, ".").unwrap();
        stream.flush().unwrap();

        // Read final response
        let mut final_response = String::new();
        reader.read_line(&mut final_response).unwrap();
        assert!(final_response.starts_with("250"));

        // Send QUIT
        let response = send_command(&mut stream, "QUIT").unwrap();
        assert!(response.starts_with("221"));

        // Wait for email to be processed
        let email = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(email.from, "test@example.com");
        assert_eq!(email.to, vec!["recipient@example.com"]);
        assert!(email.data.contains("Subject: EHLO Test Email"));
        assert!(email.data.contains("This is a test."));
    }
}
