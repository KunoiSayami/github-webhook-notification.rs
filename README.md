# GitHub webhook notifications

A webhook server that forwards GitHub webhook events to Telegram. Can be used as a standalone binary or embedded as a library in another Rust project.



## Compile

It consumes around 1.2GiB of RAM at maximum, together with a disk usage of 2GiB.

**Please make sure you have abundant resources before compiling.**

You need an available Rust compiler, `rustup` for example.

```sh
git clone https://github.com/KunoiSayami/github-webhook-notification.rs.git
cd github-webhook-notification.rs
cargo build --release
```

Then go to `target/` and copy the binary to wherever you want, e.g. `/usr/bin`.



## Install From Pre-built Executable

If you are unable to compile, download a pre-built binary from the [release page](https://github.com/KunoiSayami/github-webhook-notification.rs/releases/).

**Remember to make it executable.**



## Configuration

The default config path is `data/config.toml`. Pass `-c` to override it.

```toml
# ./data/config.toml
[server]
bind = "127.0.0.1"
port = 11451
secrets = "1145141919810"
#token = "henghengaaaaaaa"

[telegram]
bot_token = "1145141919:810abcdefg"
send_to = [114514, 1919810]

[[repository]]
full_name = "MonsterSenpai/SummerNight-HornyFantasy"
send_to = [11, 4, 514, 1919, 81, 0]

[[repository]]
full_name = "BillyKing/Wrestling"
send_to = 233
branch_ignore = ["test", "2323"]
```

`[server]`

- `bind` — address to listen on. Listening on localhost behind an SSL/TLS frontend (e.g. nginx) is recommended.
- `port` — listening port.
- `secrets` — HMAC secret for GitHub webhook signature verification (`X-Hub-Signature-256`). Highly recommended.
- `token` *(optional)* — URL token for an extra layer of auth. Append `?token=<your_token>` to the webhook URL when using this.

`[telegram]`

- `bot_token` — Telegram bot token from [@BotFather](https://t.me/botfather).
- `api_server` *(optional)* — custom Telegram Bot API server URL.
- `send_to` — default chat ID(s) to send notifications to. Accepts a single integer or an array.

`[[repository]]`

Per-repository overrides. Any repository not listed here falls back to the global `telegram.send_to` and `server.secrets`.

- `full_name` — repository path in `owner/repo` format.
- `send_to` *(optional)* — chat ID(s) for this repository. Falls back to global `send_to` if omitted.
- `branch_ignore` *(optional)* — branches whose push events are silently skipped.
- `secrets` *(optional)* — per-repository HMAC secret. Falls back to `server.secrets` if omitted.



## Deploy

```sh
github-webhook-notification -c data/config.toml
```

Run `github-webhook-notification --help` for all options.

For production, use a systemd service:

```ini
# /etc/systemd/system/gh-wbhk-tg.service

[Unit]
Description=github-webhook-telegram
Wants=network.target
After=network.target

[Service]
Type=simple
Restart=on-failure
RestartSec=10s
Environment="RUST_LOG=info"
ExecStart=/usr/bin/github-webhook-notification -c /etc/ksutils/webhook/config.toml

[Install]
WantedBy=multi-user.target
```



## Library Usage

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
github-webhook-notification = { git = "https://github.com/KunoiSayami/github-webhook-notification.rs.git" }
```

### High-level: run the full server

```rust
use github_webhook_notification::server::run_from_config_file;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_from_config_file("data/config.toml").await
}
```

Or build the config yourself and pass it in:

```rust
use github_webhook_notification::configure::Config;
use github_webhook_notification::server::run;

let config = Config::new("data/config.toml")?;
run(config).await?;
```

### Low-level: embed the router in your own server

```rust
use github_webhook_notification::configure::Config;
use github_webhook_notification::server::{AppState, ExtraData, Command, build_router, process_send_message};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

let config = Config::new("data/config.toml")?;
let (bot_tx, bot_rx) = mpsc::channel(1024);

tokio::spawn(process_send_message(
    config.telegram().bot_token().to_string(),
    config.telegram().api_server().clone(),
    bot_rx,
));

let state = AppState {
    auth_token: config.server().token().to_string(),
    config,
    extra: Arc::new(RwLock::new(ExtraData { bot_tx })),
};

let router = build_router(state);
// merge `router` into your own axum Router, or bind it directly
```

### Available public API

| Item | Description |
|---|---|
| `configure::Config` | Parsed configuration |
| `configure::Server` / `Telegram` / `Repository` | Config sub-types |
| `server::run(config)` | Start the full server from a `Config` |
| `server::run_from_config_file(path)` | Load config then start the server |
| `server::build_router(state)` | Build the axum `Router` for embedding |
| `server::AppState` | Shared state passed to handlers |
| `server::ExtraData` | Holds the Telegram message sender channel |
| `server::Command` | Message type for the Telegram sender task |
| `server::compute_signature(secret, body)` | Compute `X-Hub-Signature-256` |
| `datastructures::GitHubPushEvent` | Parsed push event |
| `datastructures::GitHubPingEvent` | Parsed ping event |
| `datastructures::GitHubEarlyParse` | Minimal parse to extract repo name |
| `datastructures::DisplayableEvent` | Trait for event formatting |



## License

[![](https://www.gnu.org/graphics/agplv3-155x51.png)](https://www.gnu.org/licenses/agpl-3.0.txt)

Copyright (C) 2021 KunoiSayami

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
