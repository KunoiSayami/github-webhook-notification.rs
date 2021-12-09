# GitHub webhook notifications 

A simple webhook server forward message to telegram.

## Install

Download the latest release binary file.

```sh
wget https://github.com/KunoiSayami/github-webhook-notification.rs/release/latest/download/github-webhook-notification_linux_amd64
```

Make it executable.

```sh
chmod +x github-webhook-notification_linux_amd64
```

Then copy it wherever you want it to be.

For example:

```sh
cp github-webhook-notification_linux_amd64 /usr/local/bin/
```

## Config

It looks like this. 

You can place it anywhere you like. But you must use `-c` paramter to specify the path of your configuation file. 

```toml
# ./data/config.toml
# /usr/local/etc/gh-wbhk-tg/config.toml
[server]
bind = ""
port = 25511
secrets = ""

[telegram]
bot_token = ""
send_to = 0
```

`bind` is the address you want this server to listen.

`port` is the listening port. Set any available value for your server as you like. 

`secrets` is optional. Should correspond to the "secret" field value in GitHub's webhook settings.

`bot_token` is the bot token of your telegram bot. You can find it in  Telegram@Botfather. 

`send_to` is the "chat_id" of the group/channel/pm you want to send your message to. As for the acquisition of "chat_id", you can search Google.



This usage will be mentioned below.

## Deploy

Type `./github-webhook-notification_linux_amd64 --help` to get more usages.

It's OK to simply run it.

```sh
./github-webhook-notification_linux_amd64 -c data/config.toml
```

But it's better to set up a `systemctl` service.

Take this for an example.

```ini
[Unit]
Description=github-webhook-telegram
Wants=network.target nginx.service
After=network.target nginx.service

[Service]
Type=simple
ExecStart=/usr/local/bin/github-webhook-notification_linux_amd64 -c /usr/local/etc/gh-wbhk-tg/config.toml

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