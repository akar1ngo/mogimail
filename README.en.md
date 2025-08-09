[日本語版](./README.md)

# MogiMail

MogiMail is an embedded SMTP server for testing.

It enables testing of email code without using mocks.

## Installation

Add the following to `Cargo.toml`.

```toml
[dev-dependencies]
mogimail = { git = "https://github.com/akar1ngo/mogimail.git" }
```

## Usage

### As a library

```rust
use mogimail::SmtpServer;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
fn test_email_sending() {
    // Create and start the server
    let (tx, rx) = mpsc::channel();
    let server = SmtpServer::new("test.local");

    thread::spawn(move || {
        server.start("127.0.0.1:2525", tx).expect("failed starting server");
    });

    // Execute app code
    send_mail("127.0.0.1:2525", "test@example.com", "recipient@example.com", "Test Subject", "Test Body");

    // Wait for the server process to complete
    let email = rx.recv_timeout(Duration::from_secs(1)).expect("timeout exceeded");

    // Check the contents of the sent email
    assert_eq!(email.from, "test@example.com");
    assert_eq!(email.to, vec!["recipient@example.com"]);
    assert!(email.data.contains("Test Subject"));
}
```

### As a standalone server

```bash
# Start with default settings (localhost:2525)
cargo run

# Start with specified address and hostname
cargo run -- 127.0.0.1:8025 myhost.local
```

The standalone server will output received emails to standard output in order.

## Running example code

```bash
cargo run --example basic_usage
```

## Testing

Run the test suite:

```bash
cargo test
```

## Supported SMTP commands

- `HELO` - Identify the sender
- `MAIL FROM` - Specify the sender's address
- `RCPT TO` - Specify the destination (multiple destinations are supported)
- `DATA` - Send the body
- `RSET` - Reset the current transaction
- `NOOP` - Do nothing
- `QUIT` - Close connection

## Notes

- Only the "minimal implementation" defined in RFC 821 is implemented.
- Runs in-memory only. Email persistence is not supported.
- SMTP authentication is not supported.
- SSL/TLS connection is not supported.
- Mail relay is not supported.

## License

This library is licensed under the MIT License.
See the LICENSE file for details.
