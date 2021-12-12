# GitHub webhook notifications 

A simple webhook server that helps you forward GitHub webhook messages to Telegram.



## Compile

It consumes around 1.2GiB of RAM at maximum, together with a disk usage of 2GiB. 

**Please make sure you have abundant resources.**

And you need an available rust compiler, `rustup`, for instance.

```sh
git clone https://github.com/KunoiSayami/github-webhook-notification.rs.git
cd github-webhook-notification.rs
cargo build --release
```

Then go to `target/` and you will find the executable binary file. Copy it to the place you want to destinate it to.



## Install From Pre-built Executable

If you are unable to compile, it's OK to download from the [latest release](https://github.com/KunoiSayami/github-webhook-notification.rs/releases/latest/).

<!--sudo curl -L https://github.com/KunoiSayami/github-webhook-notification.rs/releases/latest/download/github-webhook-notification_linux_amd64 -o /usr/local/bin/github-webhook-notification_linux_amd64-->



## Configuration

It looks like this. 

You can place it anywhere you like. But you must use `-c` parameter to specify the path of your configuration file. 

```toml
# ./data/config.toml
# /usr/local/etc/gh-wbhk-tg/config.toml
[server]
bind = ""
port = 11451
secrets = "1145141919810"
#token = ""

[telegram]
bot_token = "1145141919:810abcdefg"
send_to = [114514, 1919810]
```

- `bind` 

  is the address you want this server to listen.

- `port` 

  is the listening port. 

  Set any available value for your server as you like. 

- `secrets` 

  is for client authentication.

  It is highly recommended to set this to secure your service.

  Should correspond to the "secret" field value in your GitHub webhook settings.
  
- `token`

  Token embedded in the URL. 

  When using it, please append  `/?token=<your_token>` to your URL.

- `bot_token` 

  is the bot token of your Telegram bot. 

  You can find it in  [Telegram@Botfather](https://t.me/botfather). 

  If you don't have a bot token, you can turn to it to create a new bot, too.

- `send_to` 

  is the set of the group/channel/pm(s) you want to send your message to. 

  You need to fill the "chat_id" of these chats in the bracket. 

  As for the acquisition of "chat_id", you can search Google.



This usage will be mentioned below.

## Deploy

Type `github-webhook-notification --help` to get more usages.

It's OK to simply run it. For example:

```sh
github-webhook-notification -c data/config.toml
```

But it's better to set up a service.

Take this for an example.

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
ExecStart=/usr/local/bin/github-webhook-notification -c /usr/local/etc/gh-wbhk-tg/config.toml

[Install]
WantedBy=multi-user.target

```

It's better to launch it after the web server starts to work. 

You can customize the `Execstart` command as you like.



## License

[![](https://www.gnu.org/graphics/agplv3-155x51.png)](https://www.gnu.org/licenses/agpl-3.0.txt)

Copyright (C) 2021 KunoiSayami

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
