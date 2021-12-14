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
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use toml::Value;

#[derive(Deserialize, Serialize, Clone)]
pub struct TomlConfig {
    server: TomlServer,
    telegram: TomlTelegram,
    repository: Option<Vec<TomlRepository>>,
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
    pub fn telegram(&self) -> &TomlTelegram {
        &self.telegram
    }
    pub fn repository(&self) -> &Option<Vec<TomlRepository>> {
        &self.repository
    }

    pub fn convert_hashmap(&self) -> HashMap<String, Repository> {
        let mut m = HashMap::new();
        if let Some(repositories) = &self.repository() {
            for repository in repositories {
                m.insert(repository.full_name().clone(), Repository::from(repository));
            }
        }
        m
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TomlRepository {
    full_name: String,
    send_to: Option<Value>,
    branch_ignore: Option<Vec<String>>,
    secrets: Option<String>,
}

impl TomlRepository {
    pub fn full_name(&self) -> &String {
        &self.full_name
    }
    pub fn send_to(&self) -> &Option<Value> {
        &self.send_to
    }
    pub fn branch_ignore(&self) -> &Option<Vec<String>> {
        &self.branch_ignore
    }
    pub fn secrets(&self) -> &Option<String> {
        &self.secrets
    }
}

#[derive(Debug, Clone)]
pub struct Repository {
    //full_name: String,
    send_to: Vec<i64>,
    branch_ignore: Vec<String>,
    secrets: String,
}

impl Repository {
    /*pub fn full_name(&self) -> &str {
        &self.full_name
    }*/
    pub fn send_to(&self) -> &Vec<i64> {
        &self.send_to
    }
    pub fn branch_ignore(&self) -> &Vec<String> {
        &self.branch_ignore
    }
    pub fn secrets(&self) -> &String {
        &self.secrets
    }
}

impl From<&TomlRepository> for Repository {
    fn from(repo: &TomlRepository) -> Self {
        Self {
            //full_name: repo.full_name().clone(),
            send_to: match repo.send_to() {
                None => vec![],
                Some(v) => parse_value(v),
            },
            branch_ignore: match repo.branch_ignore() {
                Some(v) => v.clone(),
                None => vec![],
            },
            secrets: match repo.secrets() {
                None => "".to_string(),
                Some(ref secret) => secret.clone(),
            },
        }
    }
}

/*impl From<&Config> for Repository {
    fn from(s: &Config) -> Self {
        Self {
            send_to: s.telegram().send_to().clone(),
            secrets: Some(s.server().secrets().clone()),
            ..Default::default()
        }
    }
}*/

#[derive(Debug, Default, Clone)]
pub struct RepositoryBuilder {
    send_to: Vec<i64>,
    branch_ignore: Vec<String>,
    secrets: String,
}

impl RepositoryBuilder {
    pub fn set_send_to(&mut self, send_to: Vec<i64>) -> &mut Self {
        self.send_to = send_to;
        self
    }
    #[allow(unused)]
    pub fn set_branch_ignore(&mut self, branch_ignore: Vec<String>) -> &mut Self {
        self.branch_ignore = branch_ignore;
        self
    }
    pub fn set_secrets(&mut self, secrets: &String) -> &mut Self{
        self.secrets = secrets.clone();
        self
    }
    pub fn build(&self) -> Repository {
        Repository {
            send_to: self.send_to.clone(),
            branch_ignore: self.branch_ignore.clone(),
            secrets: self.secrets.clone(),
        }
    }
    pub fn new() -> Self {
        Self { ..Default::default() }
    }
}


#[derive(Debug, Clone)]
pub struct Telegram {
    bot_token: String,
    api_server: Option<String>,
    send_to: Vec<i64>,
}

impl Telegram {
    pub fn bot_token(&self) -> &str {
        &self.bot_token
    }
    pub fn api_server(&self) -> &Option<String> {
        &self.api_server
    }
    pub fn send_to(&self) -> &Vec<i64> {
        &self.send_to
    }
}

pub fn parse_value(value: &Value) -> Vec<i64> {
    match value {
        Value::String(s) => {
            vec![i64::from_str(s.as_str()).expect("Can't parse string value to i64")]
        }
        Value::Integer(i) => vec![*i],
        Value::Array(v) => v
            .iter()
            .map(|x| match x {
                Value::String(s) => i64::from_str(s).expect("Can't parse array string to i64"),
                Value::Integer(i) => *i,
                _ => panic!("Unexpected value {:?}", x),
            })
            .collect(),
        _ => panic!("Unexpected value {:?}", value),
    }
}

impl From<&TomlTelegram> for Telegram {
    fn from(value: &TomlTelegram) -> Self {
        let receivers: Vec<i64> = parse_value(value.send_to());
        Self {
            bot_token: value.bot_token().clone(),
            api_server: value.api_server().clone(),
            send_to: receivers,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TomlTelegram {
    bot_token: String,
    api_server: Option<String>,
    send_to: Value,
}

impl TomlTelegram {
    pub fn bot_token(&self) -> &String {
        &self.bot_token
    }
    pub fn api_server(&self) -> &Option<String> {
        &self.api_server
    }
    pub fn send_to(&self) -> &Value {
        &self.send_to
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    server: Server,
    telegram: Telegram,
    repo_mapping: HashMap<String, Repository>,
}

impl Config {
    pub fn server(&self) -> &Server {
        &self.server
    }
    pub fn telegram(&self) -> &Telegram {
        &self.telegram
    }
    pub fn repo_mapping(&self) -> &HashMap<String, Repository> {
        &self.repo_mapping
    }
}

impl Config {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
        let config = TomlConfig::new(path)?;
        Ok(Self::from(&config))
    }

    pub fn fetch_repository_configure(&self, branch_name: &str) -> Repository {
        let conf = self.repo_mapping().get(branch_name);
        match conf {
            None => {
                RepositoryBuilder::new()
                    .set_send_to(self.telegram().send_to().clone())
                    .set_secrets(self.server().secrets())
                    .build()
            }
            Some(repository) => repository.clone()
        }
    }
}

impl From<&TomlConfig> for Config {
    fn from(config: &TomlConfig) -> Self {
        Self {
            server: Server::from(config.server()),
            telegram: Telegram::from(config.telegram()),
            repo_mapping: config.convert_hashmap(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
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

#[derive(Debug, Clone)]
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
