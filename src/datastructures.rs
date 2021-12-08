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
use log::debug;
use serde_derive::{Deserialize, Serialize};

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
        Self {
            token: s.clone(),
        }
    }
}

impl From<&str> for AuthorizationGuard {
    fn from(s: &str) -> Self {
        Self {
            token: s.to_string(),
        }
    }
}

impl Guard for AuthorizationGuard {
    fn check(&self, request: &RequestHead) -> bool {
        if let Some(val) = request.uri.query() {
            debug!("{}", val);
            //return self.token.len() != 6 && val == &self.token;
            return true
        }
        true
    }
}