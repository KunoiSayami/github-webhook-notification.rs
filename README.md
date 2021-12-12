# GitHub webhook notifications 

A simple webhook server that helps you forward GitHub webhook messages to Telegram.



## Compile

It consumes around 1.2GiB of RAM at maximum, together with a disk usage of 2GiB. 

**Please make sure you have abundant resources before compiling.**

And you need an available rust compiler, `rustup`, for instance.

```sh
git clone https://github.com/KunoiSayami/github-webhook-notification.rs.git
cd github-webhook-notification.rs
cargo build --release
```

Then go to `target/` and you will find the executable binary file. Copy it to the place you want to destinate it to. 

A typical location is `/usr/bin` for most Linux distributions.



## Install From Pre-built Executable

If you are unable to compile, it's OK to download pre-built binary file from the [release page](https://github.com/KunoiSayami/github-webhook-notification.rs/releases/).

Remember to make it executable.

<!--sudo curl -L https://github.com/KunoiSayami/github-webhook-notification.rs/releases/latest/download/github-webhook-notification_linux_amd64 -o /usr/bin/github-webhook-notification-->



## Configuration

It looks like this. 

You can place it anywhere you like. But you must use `-c` parameter to specify the path of your configuration file. 

```toml
# ./data/config.toml
# /etc/ksutils/webhook/config.toml
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

Settings for the server.

- `bind` 

  is the address you want this server to listen. 

  It's best to listen localhost.

- `port` 

  is the listening port. 

  Set any available value for your server as you like. 

- `secrets` 

  is for client authentication.

  It is **highly recommended** to set this to secure your service.

  It should match the "secret" field value in your GitHub webhook settings.
  
- `token`

  Token embedded in the URL. 

  When using it, please append  `/?token=<your_token>` to your URL.

`[telegram]`

Global settings regarding Telegram.

- `bot_token` 

  is the bot token of your Telegram bot. 

  You can find it in  [Telegram@Botfather](https://t.me/botfather). 

  If you don't have a bot token, you can turn to it to create a new bot.

- `send_to` 

  is the default set of the group/channel/pm(s) you want to send your message to. 

  You just need to fill the "chat_id" of these chats in the bracket. 

  It's OK to leave it blank, but in this case you must specify the `send_to` per repository. 
  
  As for the acquisition of "chat_id", you can search Google. 

`[[repository]]`

Individual settings for each repository.

- `full_name`

  is the path of the repository, formatted in `owner/repository_name`.

- `send_to`

  specifies the (list of) chat_id(s), to which you want to send messages from this `owner/repo`. 

  If left blank, messages will be sent to all chats listed in `telegram.send_to`.

- `branch_ignore` 

  is the branch(es) that you want to ignore. 
  
  Events from this/these branch(es) will not be sent.

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
ExecStart=/usr/bin/github-webhook-notification -c /etc/ksutils/webhook/config.toml

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
