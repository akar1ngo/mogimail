use mogimail::SmtpServer;
use std::env;
use std::sync::mpsc;
use std::thread;

fn main() {
    let args: Vec<String> = env::args().collect();

    let addr = if args.len() > 1 {
        args[1].as_str()
    } else {
        "127.0.0.1:2525"
    };

    let hostname = if args.len() > 2 {
        args[2].as_str()
    } else {
        "mogimail.local"
    };

    println!("Starting MogiMail SMTP server...");
    println!("Address: {addr}");
    println!("Hostname: {hostname}");

    let (tx, rx) = mpsc::channel::<mogimail::Email>();
    let server = SmtpServer::new(hostname);

    thread::spawn(move || {
        let mut count = 0;
        while let Ok(email) = rx.recv() {
            count += 1;
            println!(
                "Received email #{} from: {} to: {:?}",
                count, email.from, email.to
            );
            if let Some(subject) = email.get_subject() {
                println!("  Subject: {subject}");
            }
        }
    });

    if let Err(e) = server.start(addr, tx) {
        eprintln!("Failed to start server: {e}");
        std::process::exit(1);
    }
}
