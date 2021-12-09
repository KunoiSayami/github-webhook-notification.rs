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

use log::{error, warn};
use serde_derive::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize, Serialize)]
pub struct TomlConfig {
    server: TomlServer,
    telegram: Telegram,
}

impl TryFrom<&str> for TomlConfig {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(toml::from_str(value)?)
    }
}

impl TomlConfig {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<TomlConfig> {
        let contents = std::fs::read_to_string(&path);
        if let Err(ref e) = contents {
            error!(
                "Unable read file {}, Error: {:?}",
                path.as_ref().display(),
                e
            );
        };
        let contents = contents?;
        Self::try_from(contents.as_str())
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
    send_to: i64,
}

impl Telegram {
    pub fn bot_token(&self) -> &str {
        &self.bot_token
    }
    pub fn api_server(&self) -> &Option<String> {
        &self.api_server
    }
    pub fn send_to(&self) -> i64 {
        self.send_to
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
            telegram: config.telegram().clone(),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct TomlServer {
    bind: String,
    port: u16,
    secrets: Option<String>,
    token: Option<String>,
}

impl TomlServer {
    pub fn bind(&self) -> &str {
        &self.bind
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn secrets(&self) -> &Option<String> {
        &self.secrets
    }
    pub fn token(&self) -> &Option<String> {
        &self.token
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Server {
    bind: String,
    secrets: String,
    token: String,
}

impl From<&TomlServer> for Server {
    fn from(s: &TomlServer) -> Self {
        let warning = "Both secrets and token is blank, please fill last one field to make sure your webhook server safe";
        if s.secrets().is_none() && s.token().is_none() {
            eprintln!("{}", warning);
            warn!("{}", warning);
        } else if let Some(ref secrets) = s.secrets() {
            if secrets.is_empty() {
                if let Some(ref token) = s.token() {
                    if token.is_empty() {
                        eprintln!("{}", warning);
                        warn!("{}", warning)
                    }
                }
            }
        }
        Self {
            bind: format!("{}:{}", s.bind(), s.port()),
            secrets: s.secrets().clone().unwrap_or_else(|| "".to_string()),
            token: s.token().clone().unwrap_or_else(|| "".to_string()),
        }
    }
}

impl Server {
    pub fn bind(&self) -> &String {
        &self.bind
    }
    pub fn secrets(&self) -> &String {
        &self.secrets
    }
    pub fn token(&self) -> &str {
        &self.token
    }
}
