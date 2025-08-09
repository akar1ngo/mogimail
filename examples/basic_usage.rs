//! Basic usage example for MogiMail SMTP server
//!
//! This example demonstrates how to use the MogiMail library to create
//! an in-memory SMTP server for testing purposes.

use mogimail::SmtpServer;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn main() {
    println!("MogiMail Basic Usage Example");
    println!("============================");

    // Create a new SMTP server
    let (tx, rx) = mpsc::channel();
    let server = SmtpServer::new("example.local");
    println!("Created SMTP server with hostname: example.local");

    // Start server in a background thread
    let _server_thread = thread::spawn(move || {
        if let Err(e) = server.start("127.0.0.1:2525", tx) {
            eprintln!("Server error: {e}");
        }
    });

    println!("Server started on 127.0.0.1:2525");

    // Send a test email using a simple SMTP client
    println!("\nSending test email...");
    if let Err(e) = send_test_email() {
        eprintln!("Failed to send email: {e}");
        return;
    }

    // Wait for the email to be received
    println!("\nWaiting for email...");
    match rx.recv_timeout(Duration::from_secs(1)) {
        Ok(email) => {
            println!("Email received:");
            println!("  From: {}", email.from);
            println!("  To: {:?}", email.to);
            println!("  Timestamp: {:?}", email.timestamp);
            println!("  Data:");
            for line in email.data.lines() {
                println!("    {line}");
            }
        }
        Err(_) => {
            eprintln!("Timeout: No email received within 1 second");
            return;
        }
    }

    println!("\nSending second test email...");
    if let Err(e) = send_second_test_email() {
        eprintln!("Failed to send second email: {e}");
        return;
    }

    println!("\nCollecting emails...");
    let mut emails = Vec::new();
    while let Ok(email) = rx.recv_timeout(Duration::from_millis(100)) {
        emails.push(email);
    }

    println!("Collected {} email(s) total", emails.len());

    // Filter emails by recipient
    let emails_for_recipient: Vec<_> = emails
        .iter()
        .filter(|email| email.has_recipient("recipient@example.com"))
        .collect();
    println!(
        "Emails for recipient@example.com: {}",
        emails_for_recipient.len()
    );

    // Filter emails by sender
    let emails_from_sender: Vec<_> = emails
        .iter()
        .filter(|email| email.is_from_sender("sender@example.com"))
        .collect();
    println!(
        "Emails from sender@example.com: {}",
        emails_from_sender.len()
    );
}

fn send_test_email() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the server
    let mut stream = TcpStream::connect("127.0.0.1:2525")?;
    let mut reader = BufReader::new(stream.try_clone()?);

    // Read greeting
    let mut response = String::new();
    reader.read_line(&mut response)?;
    print!("S: {response}");

    // Send HELO
    writeln!(stream, "HELO client.example.com")?;
    response.clear();
    reader.read_line(&mut response)?;
    print!("S: {response}");

    // Send MAIL FROM
    writeln!(stream, "MAIL FROM:<sender@example.com>")?;
    response.clear();
    reader.read_line(&mut response)?;
    print!("S: {response}");

    // Send RCPT TO
    writeln!(stream, "RCPT TO:<recipient@example.com>")?;
    response.clear();
    reader.read_line(&mut response)?;
    print!("S: {response}");

    // Send DATA
    writeln!(stream, "DATA")?;
    response.clear();
    reader.read_line(&mut response)?;
    print!("S: {response}");

    // Send email content
    writeln!(stream, "From: sender@example.com")?;
    writeln!(stream, "To: recipient@example.com")?;
    writeln!(stream, "Subject: Test Email from MogiMail")?;
    writeln!(stream)?;
    writeln!(stream, "This is a test email sent to demonstrate")?;
    writeln!(stream, "the MogiMail SMTP server functionality.")?;
    writeln!(stream)?;
    writeln!(stream)?;
    writeln!(stream, "Best regards,")?;
    writeln!(stream, "MogiMail Example")?;
    writeln!(stream, ".")?; // End of data marker

    // Read final response
    response.clear();
    reader.read_line(&mut response)?;
    print!("S: {response}");

    // Send QUIT
    writeln!(stream, "QUIT")?;
    response.clear();
    reader.read_line(&mut response)?;
    print!("S: {response}");

    Ok(())
}

fn send_second_test_email() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the server
    let mut stream = TcpStream::connect("127.0.0.1:2525")?;
    let mut reader = BufReader::new(stream.try_clone()?);

    // Read greeting
    let mut response = String::new();
    reader.read_line(&mut response)?;

    // Send HELO
    writeln!(stream, "HELO client.example.com")?;
    response.clear();
    reader.read_line(&mut response)?;

    // Send MAIL FROM
    writeln!(stream, "MAIL FROM:<sender@example.com>")?;
    response.clear();
    reader.read_line(&mut response)?;

    // Send RCPT TO (multiple recipients)
    writeln!(stream, "RCPT TO:<recipient@example.com>")?;
    response.clear();
    reader.read_line(&mut response)?;

    writeln!(stream, "RCPT TO:<another@example.com>")?;
    response.clear();
    reader.read_line(&mut response)?;

    // Send DATA
    writeln!(stream, "DATA")?;
    response.clear();
    reader.read_line(&mut response)?;

    // Send email content
    writeln!(stream, "From: sender@example.com")?;
    writeln!(stream, "To: recipient@example.com, another@example.com")?;
    writeln!(stream, "Subject: Second Test Email")?;
    writeln!(stream)?;
    writeln!(
        stream,
        "This is the second test email with multiple recipients."
    )?;
    writeln!(stream, ".")?; // End of data marker

    // Read final response
    response.clear();
    reader.read_line(&mut response)?;

    // Send QUIT
    writeln!(stream, "QUIT")?;
    response.clear();
    reader.read_line(&mut response)?;

    Ok(())
}
