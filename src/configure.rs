/*
 ** Copyright (C) 2021 KunoiSayami
 **
 ** This program is free software: you can redistribute it and/or modify
 ** it under the terms of the GNU Affero General Public License as published by
 ** the Free Software Foundation, either version 3 of the License, or
 ** any later version.
 **
 ** This program is distributed in the hope that it will be useful,
 ** but WITHOUT ANY WARRANTY; without even the implied warranty of
 ** MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 ** GNU Affero General Public License for more details.
 **
 ** You should have received a copy of the GNU Affero General Public License
 ** along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use serde_derive::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize, Serialize)]
pub struct Config {
    server: Server,
    telegram: Telegram,
}

impl Config {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
        let contents = std::fs::read_to_string(&path)?;
        let contents_str = contents.as_str();

        Ok(toml::from_str(contents_str)?)
    }

    pub fn server(&self) -> &Server {
        &self.server
    }
    pub fn telegram(&self) -> &Telegram {
        &self.telegram
    }

    pub fn get_bind_params(&self) -> String {
        format!("{}:{}", self.server().bind(), self.server().port())
    }
}

#[derive(Deserialize, Serialize)]
pub struct Telegram {
    bot_token: String,
    api_server: Option<String>,
    owner: i64,
}

impl Telegram {
    pub fn bot_token(&self) -> &str {
        &self.bot_token
    }
    pub fn api_server(&self) -> &Option<String> {
        &self.api_server
    }
    pub fn owner(&self) -> i64 {
        self.owner
    }
}

#[derive(Deserialize, Serialize)]
pub struct Server {
    bind: String,
    port: u16,
    token: String,
}

impl Server {
    pub fn bind(&self) -> &str {
        &self.bind
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn token(&self) -> &str {
        &self.token
    }
}

#[derive(Deserialize, Serialize)]
pub struct Request {
    #[serde(rename = "ref")]
    remote_ref: String,
}
