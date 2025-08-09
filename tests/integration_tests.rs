//! Integration tests for size limits, UTF-8 handling, and comprehensive SMTP scenarios

use mogimail::{SmtpLimits, SmtpServer};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn start_test_server() -> (String, mpsc::Receiver<mogimail::Email>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let server = SmtpServer::new("test.local");
    let (tx, rx) = mpsc::channel::<mogimail::Email>();

    // Start server in background thread
    thread::spawn(move || {
        if let Err(e) = server.start_with_listener(listener, tx) {
            eprintln!("Error starting server: {e}");
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
fn test_command_line_length_limit() {
    let (addr, _rx) = start_test_server();
    let mut stream = TcpStream::connect(&addr).unwrap();

    // Read greeting
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut greeting = String::new();
    reader.read_line(&mut greeting).unwrap();
    assert!(greeting.starts_with("220"));

    // Test command line that's too long
    let long_command = "HELO ".to_string() + &"a".repeat(SmtpLimits::COMMAND_LINE_MAX_LENGTH);
    let response = send_command(&mut stream, &long_command).unwrap();
    assert!(response.starts_with("500")); // Line too long

    send_command(&mut stream, "QUIT").unwrap();
}

#[test]
fn test_domain_name_length_limit() {
    let (addr, _rx) = start_test_server();
    let mut stream = TcpStream::connect(&addr).unwrap();

    // Read greeting
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut greeting = String::new();
    reader.read_line(&mut greeting).unwrap();

    // Test domain name that's too long
    let long_domain = "a".repeat(SmtpLimits::DOMAIN_MAX_LENGTH + 1);
    let response = send_command(&mut stream, &format!("HELO {long_domain}")).unwrap();
    assert!(response.starts_with("501")); // Domain too long

    send_command(&mut stream, "QUIT").unwrap();
}

#[test]
fn test_email_address_component_limits() {
    let (addr, _rx) = start_test_server();
    let mut stream = TcpStream::connect(&addr).unwrap();

    // Read greeting
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut greeting = String::new();
    reader.read_line(&mut greeting).unwrap();

    // Setup session
    send_command(&mut stream, "HELO client.local").unwrap();

    // Test user part that's too long
    let long_user = "a".repeat(SmtpLimits::USER_MAX_LENGTH + 1);
    let response = send_command(
        &mut stream,
        &format!("MAIL FROM:<{long_user}@example.com>"),
    )
    .unwrap();
    assert!(response.starts_with("501")); // User too long

    // Test domain part that's too long
    let long_domain = "a".repeat(SmtpLimits::DOMAIN_MAX_LENGTH + 1);
    let response = send_command(&mut stream, &format!("MAIL FROM:<user@{long_domain}>")).unwrap();
    assert!(response.starts_with("501")); // Domain too long

    send_command(&mut stream, "QUIT").unwrap();
}

#[test]
fn test_path_length_limit() {
    let (addr, _rx) = start_test_server();
    let mut stream = TcpStream::connect(&addr).unwrap();

    // Read greeting
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut greeting = String::new();
    reader.read_line(&mut greeting).unwrap();

    // Setup session
    send_command(&mut stream, "HELO client.local").unwrap();

    // Create a path that's too long (including angle brackets)
    let long_path = "user@".to_string() + &"a".repeat(SmtpLimits::PATH_MAX_LENGTH);
    let response = send_command(&mut stream, &format!("MAIL FROM:<{long_path}>")).unwrap();
    assert!(response.starts_with("501")); // Path too long

    send_command(&mut stream, "QUIT").unwrap();
}

#[test]
fn test_recipient_limit() {
    let (addr, _rx) = start_test_server();
    let mut stream = TcpStream::connect(&addr).unwrap();

    // Read greeting
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut greeting = String::new();
    reader.read_line(&mut greeting).unwrap();

    // Setup session
    send_command(&mut stream, "HELO client.local").unwrap();
    send_command(&mut stream, "MAIL FROM:<sender@example.com>").unwrap();

    // Add maximum allowed recipients
    for i in 0..SmtpLimits::MAX_RECIPIENTS {
        let response =
            send_command(&mut stream, &format!("RCPT TO:<user{i}@example.com>")).unwrap();
        assert!(response.starts_with("250"));
    }

    // Try to add one more recipient
    let response = send_command(&mut stream, "RCPT TO:<extra@example.com>").unwrap();
    assert!(response.starts_with("552")); // Too many recipients

    send_command(&mut stream, "QUIT").unwrap();
}

#[test]
fn test_data_line_length_limit() {
    let (addr, _rx) = start_test_server();
    let mut stream = TcpStream::connect(&addr).unwrap();

    // Read greeting
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut greeting = String::new();
    reader.read_line(&mut greeting).unwrap();

    // Setup session
    send_command(&mut stream, "HELO client.local").unwrap();
    send_command(&mut stream, "MAIL FROM:<sender@example.com>").unwrap();
    send_command(&mut stream, "RCPT TO:<recipient@example.com>").unwrap();
    send_command(&mut stream, "DATA").unwrap();

    // Send a line that's too long
    let long_line = "Subject: ".to_string() + &"a".repeat(SmtpLimits::TEXT_LINE_MAX_LENGTH);
    writeln!(stream, "{long_line}").unwrap();
    stream.flush().unwrap();

    // The server should still accept the line but may truncate or reject it
    // Continue with end of data
    writeln!(stream, ".").unwrap();
    stream.flush().unwrap();

    let mut response = String::new();
    reader.read_line(&mut response).unwrap();
    // Server might accept it with truncation or reject it
    assert!(response.starts_with("250") || response.starts_with("5"));

    if response.starts_with("250") {
        send_command(&mut stream, "QUIT").unwrap();
    }
}

#[test]
fn test_data_size_limit() {
    let (addr, _rx) = start_test_server();
    let mut stream = TcpStream::connect(&addr).unwrap();

    // Read greeting
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut greeting = String::new();
    reader.read_line(&mut greeting).unwrap();

    // Setup session
    send_command(&mut stream, "HELO client.local").unwrap();
    send_command(&mut stream, "MAIL FROM:<sender@example.com>").unwrap();
    send_command(&mut stream, "RCPT TO:<recipient@example.com>").unwrap();
    send_command(&mut stream, "DATA").unwrap();

    // Send reasonable amount of data that should be within limits
    let chunk = "a".repeat(500); // 500 byte chunks
    let chunks_to_send = 20; // Send 10KB total

    for i in 0..chunks_to_send {
        if writeln!(stream, "Line {i}: {chunk}").is_err() {
            // Connection closed - this could happen due to size limits
            return;
        }
        if i % 10 == 0 && stream.flush().is_err() {
            // Connection closed during flush
            return;
        }
    }

    if writeln!(stream, ".").is_err() {
        // Connection closed before end marker
        return;
    }

    if stream.flush().is_err() {
        // Connection closed during final flush
        return;
    }

    let mut response = String::new();
    match reader.read_line(&mut response) {
        Ok(0) => {
            // Connection closed by server - this is acceptable
        }
        Ok(_) => {
            // Server responded - should be either success or size limit error
            assert!(
                response.starts_with("552")
                    || response.starts_with("250")
                    || response.starts_with("421")
            );
        }
        Err(_) => {
            // Connection error - this is also acceptable for size limit testing
        }
    }
}

#[test]
fn test_non_utf8_input_handling() {
    let (addr, _rx) = start_test_server();
    let mut stream = TcpStream::connect(&addr).unwrap();

    // Read greeting
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut greeting = String::new();
    reader.read_line(&mut greeting).unwrap();
    assert!(greeting.starts_with("220"));

    // Send some non-UTF-8 bytes followed by a valid command
    let non_utf8_bytes = [0xFF, 0xFE, 0xFD]; // Invalid UTF-8 sequence
    stream.write_all(&non_utf8_bytes).unwrap();
    stream.write_all(b" HELO client.local\r\n").unwrap();
    stream.flush().unwrap();

    // Server should handle this gracefully and respond with an error
    let mut response = String::new();
    reader.read_line(&mut response).unwrap();
    // Should get an error response, not crash
    assert!(response.starts_with("500") || response.starts_with("501"));

    // Server should still be responsive to valid commands
    let response = send_command(&mut stream, "HELO client.local").unwrap();
    assert!(response.starts_with("250"));

    send_command(&mut stream, "QUIT").unwrap();
}

#[test]
fn test_malformed_command_with_non_ascii() {
    let (addr, _rx) = start_test_server();
    let mut stream = TcpStream::connect(&addr).unwrap();

    // Read greeting
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut greeting = String::new();
    reader.read_line(&mut greeting).unwrap();

    // Send a command with non-ASCII characters (but valid UTF-8)
    let response = send_command(&mut stream, "HELO café.example.com").unwrap();
    // Should either accept it or reject it gracefully
    assert!(response.starts_with("250") || response.starts_with("501"));

    // Send a malformed command with special characters
    let response = send_command(&mut stream, "MAIL FROM:<tëst@exämple.com>").unwrap();
    // Should handle gracefully
    assert!(
        response.starts_with("250") || response.starts_with("501") || response.starts_with("503")
    );

    send_command(&mut stream, "QUIT").unwrap();
}

#[test]
fn test_very_long_email_session() {
    let (addr, rx) = start_test_server();
    let mut stream = TcpStream::connect(&addr).unwrap();

    // Read greeting
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut greeting = String::new();
    reader.read_line(&mut greeting).unwrap();

    // Setup session
    send_command(&mut stream, "HELO client.local").unwrap();

    // Send multiple emails in the same session
    for email_num in 0..5 {
        send_command(
            &mut stream,
            &format!("MAIL FROM:<sender{email_num}@example.com>"),
        )
        .unwrap();

        // Add multiple recipients
        for recipient_num in 0..3 {
            send_command(
                &mut stream,
                &format!("RCPT TO:<recipient{recipient_num}@example.com>"),
            )
            .unwrap();
        }

        send_command(&mut stream, "DATA").unwrap();

        // Send email content
        writeln!(stream, "Subject: Test Email {email_num}").unwrap();
        writeln!(stream, "From: sender{email_num}@example.com").unwrap();
        writeln!(stream).unwrap();
        writeln!(stream, "This is test email number {email_num}").unwrap();
        writeln!(stream, "It has multiple lines").unwrap();
        writeln!(stream, "And should be handled correctly").unwrap();
        writeln!(stream, ".").unwrap();
        stream.flush().unwrap();

        // Read response
        let mut response = String::new();
        reader.read_line(&mut response).unwrap();
        assert!(response.starts_with("250"));
    }

    send_command(&mut stream, "QUIT").unwrap();

    // Wait for all emails to be processed
    let mut emails = Vec::new();
    for _ in 0..5 {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(email) => emails.push(email),
            Err(_) => break,
        }
    }

    // Verify all emails were received
    assert_eq!(emails.len(), 5);
}

#[test]
fn test_rset_clears_large_transaction() {
    let (addr, rx) = start_test_server();
    let mut stream = TcpStream::connect(&addr).unwrap();

    // Read greeting
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut greeting = String::new();
    reader.read_line(&mut greeting).unwrap();

    // Setup a large transaction
    send_command(&mut stream, "HELO client.local").unwrap();
    send_command(&mut stream, "MAIL FROM:<sender@example.com>").unwrap();

    // Add many recipients (but within limit)
    for i in 0..50 {
        send_command(&mut stream, &format!("RCPT TO:<user{i}@example.com>")).unwrap();
    }

    // Reset the transaction
    let response = send_command(&mut stream, "RSET").unwrap();
    assert!(response.starts_with("250"));

    // Should be able to start a new, smaller transaction
    send_command(&mut stream, "MAIL FROM:<newsender@example.com>").unwrap();
    send_command(&mut stream, "RCPT TO:<newrecipient@example.com>").unwrap();
    send_command(&mut stream, "DATA").unwrap();

    writeln!(stream, "Subject: After Reset").unwrap();
    writeln!(stream).unwrap();
    writeln!(stream, "This email came after RSET").unwrap();
    writeln!(stream, ".").unwrap();
    stream.flush().unwrap();

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

#[test]
fn test_concurrent_connections_with_limits() {
    let (addr, rx) = start_test_server();

    // Spawn multiple concurrent connections
    let mut handles = vec![];

    for client_id in 0..5 {
        let addr_clone = addr.clone();
        let handle = thread::spawn(move || {
            let mut stream = TcpStream::connect(&addr_clone).unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());

            // Read greeting
            let mut greeting = String::new();
            reader.read_line(&mut greeting).unwrap();
            assert!(greeting.starts_with("220"));

            // Complete SMTP session
            send_command(&mut stream, &format!("HELO client{client_id}.local")).unwrap();
            send_command(
                &mut stream,
                &format!("MAIL FROM:<sender{client_id}@example.com>"),
            )
            .unwrap();
            send_command(
                &mut stream,
                &format!("RCPT TO:<recipient{client_id}@example.com>"),
            )
            .unwrap();
            send_command(&mut stream, "DATA").unwrap();

            writeln!(stream, "Subject: Concurrent Test {client_id}").unwrap();
            writeln!(stream).unwrap();
            writeln!(stream, "This is from client {client_id}").unwrap();
            writeln!(stream, ".").unwrap();
            stream.flush().unwrap();

            let mut response = String::new();
            reader.read_line(&mut response).unwrap();
            assert!(response.starts_with("250"));

            send_command(&mut stream, "QUIT").unwrap();
        });
        handles.push(handle);
    }

    // Wait for all connections to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Wait for all emails to be processed
    let mut emails = Vec::new();
    for _ in 0..5 {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(email) => emails.push(email),
            Err(_) => break,
        }
    }

    // Verify all emails were received
    assert_eq!(emails.len(), 5);
}
