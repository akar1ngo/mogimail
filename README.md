[English](./README.en.md)

# MogiMail

テスト用の組み込み SMTP サーバです。

モックを使わずにメール通信に関するコードのテストを可能にします。

## インストール

`Cargo.toml` に以下を追加します。

```toml
[dev-dependencies]
mogimail = "0.1.1"
```

## 使い方

### ライブラリとして

```rust
use mogimail::SmtpServer;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
fn test_email_sending() {
    // サーバを作成して起動。
    let (tx, rx) = mpsc::channel();
    let server = SmtpServer::new("test.local");

    thread::spawn(move || {
        server.start("127.0.0.1:2525", tx).expect("failed starting server");
    });

    // アプリコードを実行
    send_mail("127.0.0.1:2525", "test@example.com", "recipient@example.com", "Test Subject", "Test Body");

    // サーバ処理の完了を待機します
    let email = rx.recv_timeout(Duration::from_secs(1)).expect("timeout exceeded");

    // 送信済みメールの中身を確認
    assert_eq!(email.from, "test@example.com");
    assert_eq!(email.to, vec!["recipient@example.com"]);
    assert!(email.data.contains("Test Subject"));
}
```

### スタンドアロンサーバとして

```bash
# デフォルト設定で起動 (localhost:2525)
cargo run

# アドレスとホスト名を指定して起動
cargo run -- 127.0.0.1:8025 myhost.local
```

スタンドアロンサーバは受信したメールを順番に標準出力に流してくれます。

## サンプルコードの実行

```bash
cargo run --example basic_usage
```

## テスト

Run the test suite:

```bash
cargo test
```

## 対応している SMTP コマンド

- `HELO` - 送信者を識別
- `MAIL FROM` - 送信者のアドレスを指定
- `RCPT TO` - 宛先を指定（複数可）
- `DATA` - 本文の送信
- `RSET` - 現行のトランザクションをリセット
- `NOOP` - 何もしない
- `QUIT` - 接続を終了

## 追加機能

`ehlo` 機能を有効にすると `EHLO` コマンドも利用できます。

## 注意事項

- RFC 821 で定義される「最小装備」のみ実装しています。
- インメモリのみで動作します。メールの永続化はできません。
- SMTP 認証は未対応。
- SSL/TLS 接続は未対応です。
- メールの転送は行いません。

## 利用規約

このライブラリは MIT ライセンスで提供されています。
詳細は LICENSE ファイルをご覧ください。
