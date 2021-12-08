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
use sha2::{Sha256, Digest};

#[derive(Deserialize, Serialize)]
pub struct TomlConfig {
    server: TomlServer,
    telegram: Telegram,
}

impl TomlConfig {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<TomlConfig> {
        let contents = std::fs::read_to_string(&path)?;
        let contents_str = contents.as_str();

        Ok(toml::from_str(contents_str)?)
    }

    pub fn server(&self) -> &TomlServer {
        &self.server
    }
    pub fn telegram(&self) -> &Telegram {
        &self.telegram
    }

}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    server: Server,
    telegram: Telegram,
}

impl Config {
    pub fn server(&self) -> &Server {
        &self.server
    }
    pub fn telegram(&self) -> &Telegram {
        &self.telegram
    }
}

impl Config {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
        let config = TomlConfig::new(path)?;
        Ok(Self::from(&config))
    }

}

impl From<&TomlConfig> for Config {
    fn from(config: &TomlConfig) -> Self {
        Self {
            server: Server::from(config.server()),
            telegram: config.telegram().clone()
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct TomlServer {
    bind: String,
    port: u16,
    secrets: String,
}

impl TomlServer {
    pub fn bind(&self) -> &str {
        &self.bind
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn secrets(&self) -> &str {
        &self.secrets
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Server {
    bind: String,
    secrets_sha256: String,
}

impl From<&TomlServer> for Server {
    fn from(s: &TomlServer) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(s.secrets());
        let result = hasher.finalize();
        Self {
            bind: format!("{}:{}", s.bind(), s.port()),
            secrets_sha256: format!("sha256={:x}", result).to_lowercase()
        }
    }
}

impl Server {
    pub fn bind(&self) -> &String {
        &self.bind
    }
    pub fn secrets(&self) -> &str {
        &self.secrets_sha256
    }
}