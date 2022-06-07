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

use crate::{IntoResponse, StatusCode, AUTH_TOKEN};
use axum::extract::{FromRequest, RequestParts};
use serde_derive::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::ops::Index;

pub trait DisplayableEvent: std::fmt::Display + Debug + Send + Sync {
    fn get_full_name(&self) -> &String;

    fn branch_name(&self) -> String;
}

impl<F: ?Sized + Send + Sync> DisplayableEvent for Box<F>
where
    F: DisplayableEvent,
{
    fn get_full_name(&self) -> &String {
        (**self).get_full_name()
    }

    fn branch_name(&self) -> String {
        (**self).branch_name()
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GitHubEarlyParse {
    repository: Repository,
}

impl GitHubEarlyParse {
    pub fn repository(&self) -> &Repository {
        &self.repository
    }

    pub fn get_full_name(&self) -> &String {
        self.repository().full_name()
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GitHubPingEvent {
    zen: String,
}

impl GitHubPingEvent {
    pub fn zen(&self) -> &str {
        &self.zen
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GitHubPushEvent {
    #[serde(rename = "ref")]
    remote_ref: String,
    after: String,
    before: String,
    commits: Vec<Commit>,
    compare: String,
    repository: Repository,
}

impl GitHubPushEvent {
    pub fn remote_ref(&self) -> &str {
        &self.remote_ref
    }
    pub fn commits(&self) -> &Vec<Commit> {
        &self.commits
    }
    pub fn compare(&self) -> &str {
        &self.compare
    }
    pub fn repository(&self) -> &Repository {
        &self.repository
    }
    pub fn after(&self) -> &str {
        &self.after
    }
    pub fn before(&self) -> &str {
        &self.before
    }
}

impl std::fmt::Display for GitHubPushEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let branch = self.remote_ref().rsplit_once("/").unwrap().1;
        let git_ref = format!("{}:{}", self.repository(), branch);
        if self.commits.len() == 1 {
            let item = self.commits().index(0);
            write!(
                f,
                "🔨 <a href=\"{url}\">{count} new commit</a> <b>to {git_ref}</b>:\n\n{commits}",
                url = item.url(),
                count = 1,
                git_ref = git_ref,
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
                "🔨 <a href=\"{url}\">{count} new commits</a> <b>to {git_ref}</b>:\n\n{commits}",
                url = self.compare(),
                count = self.commits.len(),
                git_ref = git_ref,
                commits = l,
            )
        }
    }
}

impl DisplayableEvent for GitHubPushEvent {
    fn get_full_name(&self) -> &String {
        self.repository().full_name()
    }

    fn branch_name(&self) -> String {
        self.remote_ref().rsplit_once('/').unwrap().1.to_string()
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

#[derive(Deserialize, Serialize, Debug)]
pub struct Repository {
    full_name: String,
}

impl Repository {
    pub fn full_name(&self) -> &String {
        &self.full_name
    }
}

impl std::fmt::Display for Repository {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_name)
    }
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Response {
    version: String,
    status: u16,
    reason: String,
    #[serde(skip)]
    empty: bool,
}

impl Response {
    pub fn new(status: u16) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            status,
            ..Default::default()
        }
    }

    pub fn new_ok() -> Self {
        Self::new(200)
    }

    pub fn reason<T: Into<String>>(status: u16, reason: T) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            status,
            reason: reason.into(),
            empty: false,
        }
    }
    pub fn new_empty() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            status: 204,
            reason: "".to_string(),
            empty: true,
        }
    }
    pub fn new_parse_error(e: serde_json::Error) -> Self {
        Self::reason(500, e.to_string())
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> axum::response::Response {
        if self.empty {
            return (StatusCode::from_u16(204).unwrap(), "").into_response();
        }
        (
            StatusCode::from_u16(self.status).expect("Wrong input in status code"),
            serde_json::to_string(&self).unwrap(),
        )
            .into_response()
    }
}

pub struct AuthorizationGuard {}

#[async_trait::async_trait]
impl<B> FromRequest<B> for AuthorizationGuard
where
    B: Send,
{
    type Rejection = StatusCode;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let token = AUTH_TOKEN.get().unwrap();
        if token.is_empty() {
            return Ok(Self {});
        }

        let checker = |query: &str| {
            if query.contains('=') {
                let (key, value) = query.split_once('=').unwrap();
                if key == "token" && value.eq(token) {
                    return true;
                }
            }
            false
        };
        if let Some(queries) = req.uri().query() {
            for query in queries.split('&') {
                if checker(query) {
                    return Ok(Self {});
                }
            }
        }
        Err(StatusCode::FORBIDDEN)
    }
}

#[derive(Debug, Clone)]
pub struct CommandBundle {
    receiver: Vec<i64>,
    text: String,
}

impl CommandBundle {
    pub fn new(receiver: Vec<i64>, text: String) -> Self {
        Self { receiver, text }
    }
    pub fn receiver(&self) -> &Vec<i64> {
        &self.receiver
    }
    pub fn text(&self) -> &str {
        &self.text
    }
}
