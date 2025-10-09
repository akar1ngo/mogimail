#![cfg(feature = "ehlo")]

use lettre::message::{Mailbox, Message};
use lettre::{SmtpTransport, Transport};
use mogimail::SmtpServer;
use std::error::Error;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
fn basic_lettre_send() -> Result<(), Box<dyn Error>> {
    let (tx, rx) = mpsc::channel();
    let server = SmtpServer::new("localhost");

    thread::spawn(move || {
        server
            .start("127.0.0.1:2525", tx)
            .expect("server start failed")
    });

    let message = Message::builder()
        .from("花子 <hanako@example.com>".parse::<Mailbox>()?)
        .to("太郎 <tarou@example.com>".parse::<Mailbox>()?)
        .subject("件名")
        .body("本文".to_owned())
        .unwrap();

    let mailer = SmtpTransport::builder_dangerous("localhost")
        .port(2525)
        .build();

    mailer.send(&message)?;

    let email = rx.recv_timeout(Duration::from_millis(100))?;
    assert_eq!(email.from, "hanako@example.com");
    assert_eq!(email.to, vec!["tarou@example.com"]);

    Ok(())
}
