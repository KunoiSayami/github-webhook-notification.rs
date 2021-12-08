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

use actix_web::dev::RequestHead;
use actix_web::guard::Guard;
use log::info;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Formatter;
use std::ops::Index;

#[derive(Deserialize, Serialize, Debug)]
pub struct Request {
    #[serde(rename = "ref")]
    remote_ref: String,
    commits: Vec<Commit>,
    compare: String,
}

impl Request {
    pub fn remote_ref(&self) -> &str {
        &self.remote_ref
    }
    pub fn commits(&self) -> &Vec<Commit> {
        &self.commits
    }
    pub fn compare(&self) -> &str {
        &self.compare
    }
}

impl std::fmt::Display for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.commits.len() == 1 {
            let item = self.commits.index(0);
            write!(
                f,
                "🔨 <a href=\"{url}\">{count} new commit</a> <b>to {git_ref}</b>:\n{commits}",
                url = item.url(),
                count = 1,
                git_ref = self.remote_ref(),
                commits = item
            )
        } else {
            let l = self
                .commits
                .iter()
                .map(|x| x.display(true))
                .collect::<Vec<String>>()
                .join("\n");
            write!(
                f,
                "🔨 <a href=\"{url}\">{count} new commits</a> <b>to {git_ref}</b>:\n{commits}",
                url = self.compare(),
                count = self.commits.len(),
                git_ref = self.remote_ref(),
                commits = l,
            )
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Commit {
    id: String,
    message: String,
    url: String,
}

impl Commit {
    pub fn id(&self) -> &String {
        &self.id
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn display(&self, title_only: bool) -> String {
        let content = if title_only {
            if self.message.contains('\n') {
                self.message().split_once("\n").unwrap().0
            } else {
                self.message()
            }
        } else {
            self.message()
        };
        format!(
            "<a href=\"{url}\">{commit_id}</a>: {content}",
            url = self.url(),
            commit_id = &self.id()[..8],
            content = content
        )
    }
}

impl std::fmt::Display for Commit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display(false))
    }
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Response {
    version: String,
    status: i64,
}

impl Response {
    #[allow(dead_code)]
    pub fn new(status: i64) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            status,
        }
    }

    pub fn new_ok() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            status: 200,
        }
    }
}

#[derive(Clone)]
pub struct AuthorizationGuard {
    token: String,
}

impl From<Option<String>> for AuthorizationGuard {
    fn from(s: Option<String>) -> Self {
        Self::from(&match s {
            Some(s) => s,
            None => "".to_string(),
        })
    }
}

impl From<&String> for AuthorizationGuard {
    fn from(s: &String) -> Self {
        Self { token: s.clone() }
    }
}

impl From<&str> for AuthorizationGuard {
    fn from(s: &str) -> Self {
        Self {
            token: s.to_string(),
        }
    }
}

// TODO: Fix this authorization
impl Guard for AuthorizationGuard {
    fn check(&self, request: &RequestHead) -> bool {
        info!("calling");
        if let Some(val) = request.uri.query() {
            info!("{}", val);
            //return self.token.len() != 6 && val == &self.token;
            return true;
        }
        true
    }
}
