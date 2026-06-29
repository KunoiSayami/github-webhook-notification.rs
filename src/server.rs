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

use crate::configure::Config;
use crate::datastructures::{
    CommandBundle, DisplayableEvent, GitHubEarlyParse, GitHubPingEvent, GitHubPushEvent, Response,
};
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request as HttpRequest, StatusCode};
use axum::response::IntoResponse;
use hex;
use hmac::{Hmac, KeyInit, Mac};
use http_body_util::BodyExt;
use log::{debug, error, info, warn};
use sha2::Sha256;
use std::sync::Arc;
use teloxide::Bot;
use teloxide::prelude::{Request, Requester, RequesterExt};
use teloxide::sugar::request::RequestLinkPreviewExt;
use teloxide::types::{ChatId, ParseMode};
use tokio::sync::{RwLock, mpsc};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

/// Compute the GitHub-style `X-Hub-Signature-256` value (`sha256=<hexdigest>`)
/// for the given secret and request body.
pub fn compute_signature(secret: &[u8], body: &[u8]) -> String {
    type HmacSha256 = Hmac<Sha256>;
    let mut h = HmacSha256::new_from_slice(secret).unwrap();
    h.update(body);
    let result = h.finalize();
    format!("sha256={}", hex::encode(result.into_bytes()))
}

fn check_0(s: &str) -> bool {
    s.chars().all(|x| x == '0')
}

#[derive(Debug)]
pub enum Command {
    Terminate,
    Bundle(CommandBundle),
}

pub struct ExtraData {
    pub bot_tx: mpsc::Sender<Command>,
}

/// Shared application state injected into axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub extra: Arc<RwLock<ExtraData>>,
    /// Bearer/query-param token for request authorization. Empty means disabled.
    pub auth_token: String,
}

pub async fn process_send_message(
    bot_token: String,
    api_server: Option<String>,
    mut rx: mpsc::Receiver<Command>,
) -> anyhow::Result<()> {
    if bot_token.is_empty() {
        warn!("Token is empty, skipped all send message request.");
        while let Some(cmd) = rx.recv().await {
            if let Command::Terminate = cmd {
                break;
            }
        }
        return Ok(());
    }
    let bot = Bot::new(bot_token);
    let bot = match api_server {
        Some(api) => bot.set_api_url(api.parse()?),
        None => bot,
    };

    let bot = bot.parse_mode(ParseMode::Html);
    while let Some(cmd) = rx.recv().await {
        match cmd {
            Command::Bundle(bundle) => {
                for send_to in bundle.receiver() {
                    let payload = bot
                        .send_message(ChatId(*send_to), bundle.text())
                        .disable_link_preview(true);
                    if let Err(e) = payload.send().await {
                        error!("Got error in send message {:?}", e);
                    }
                }
            }
            Command::Terminate => break,
        }
    }
    debug!("Send message daemon exiting...");
    Ok(())
}

async fn route_post(
    State(state): State<AppState>,
    request: HttpRequest<Body>,
) -> impl IntoResponse {
    // Authorization check
    if !state.auth_token.is_empty() {
        let authorized = request
            .uri()
            .query()
            .map(|queries| {
                queries.split('&').any(|q| {
                    q.split_once('=')
                        .is_some_and(|(k, v)| k == "token" && v == state.auth_token)
                })
            })
            .unwrap_or(false);
        if !authorized {
            return Response::reason(403, "Forbidden").into_response();
        }
    }

    let (parts, body_stream) = request.into_parts();
    let body = match body_stream.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return Response::reason(400, "failed to read body").into_response(),
    };
    if body.len() > 262_144 {
        return Response::reason(400, "overflow").into_response();
    }

    let object = serde_json::from_slice::<GitHubEarlyParse>(&body);
    if let Err(ref e) = object {
        error!("Get parser error in pre-check stage: {:?}", &e);
        error!("Raw data => {:?}", String::from_utf8_lossy(&body));
        return Response::new(500).into_response();
    };
    let object = object.unwrap();
    let settings = state
        .config
        .fetch_repository_configure(object.get_full_name());

    let secrets = settings.secrets();
    if !secrets.is_empty() {
        let sha256val = compute_signature(secrets.as_bytes(), &body);
        if let Some(val) = parts.headers.get("X-Hub-Signature-256") {
            if !sha256val.eq(val) {
                return Response::reason(403, "Checksum error").into_response();
            }
        } else {
            return Response::reason(403, "Checksum header not found").into_response();
        }
    }

    let event_header = parts.headers.get("X-GitHub-Event");
    if event_header.is_none() {
        error!("Unknown request: {:?}", parts);
        return Response::new(500).into_response();
    }
    let event_header = event_header.unwrap().to_str();
    if let Err(ref e) = event_header {
        error!("Parse X-GitHub-Event error: {:?}", e);
        return Response::new(500).into_response();
    }
    let event_header = event_header.unwrap();
    match event_header {
        "ping" => {
            let request_body = match serde_json::from_slice::<GitHubPingEvent>(&body) {
                Ok(ret) => ret,
                Err(e) => return Response::new_parse_error(e).into_response(),
            };
            Response::reason(200, request_body.zen()).into_response()
        }
        "push" => {
            let event = match serde_json::from_slice::<GitHubPushEvent>(&body) {
                Ok(ret) => ret,
                Err(e) => return Response::new_parse_error(e).into_response(),
            };
            if check_0(event.after()) || check_0(event.before()) {
                return Response::new_empty().into_response();
            }
            if settings.branch_ignore().contains(&event.branch_name()) {
                Response::reason(204, "Skipped.").into_response()
            } else {
                let sender = state.extra.write().await;
                sender
                    .bot_tx
                    .send(Command::Bundle(CommandBundle::new(
                        settings.send_to().clone(),
                        event.to_string(),
                    )))
                    .await
                    .unwrap();
                Response::new_ok().into_response()
            }
        }
        _ => Response::reason(400, format!("Unsupported event type {:?}", event_header))
            .into_response(),
    }
}

/// Build the axum [`Router`] for the webhook server.
///
/// Call this when embedding the server in another application. Bind and serve
/// the returned router yourself, or pass it to [`run`].
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", axum::routing::post(route_post))
        .route("/", axum::routing::get(|| async { Response::new_ok() }))
        .route("/", axum::routing::any(|| async { StatusCode::FORBIDDEN }))
        .with_state(state)
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
}

/// Start the webhook server from a [`Config`], binding on the address in the config.
///
/// This is the high-level entry point for both the standalone binary and
/// library callers that want the default opinionated setup.
pub async fn run(config: Config) -> anyhow::Result<()> {
    let (bot_tx, bot_rx) = mpsc::channel(1024);

    let auth_token = config.server().token().to_string();
    let extra = Arc::new(RwLock::new(ExtraData {
        bot_tx: bot_tx.clone(),
    }));

    let msg_sender = tokio::spawn(process_send_message(
        config.telegram().bot_token().to_string(),
        config.telegram().api_server().clone(),
        bot_rx,
    ));

    let bind = config.server().bind().clone();
    info!("Bind address: {}", bind);

    let state = AppState {
        config,
        extra,
        auth_token,
    };
    let router = build_router(state);

    let handler = axum_server::Handle::<std::net::SocketAddr>::new();
    let server = tokio::spawn(
        axum_server::bind(bind.parse().unwrap())
            .handle(handler.clone())
            .serve(router.into_make_service()),
    );

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            handler.graceful_shutdown(None);
        }
        ret = server => {
            ret??;
        }
    }

    bot_tx.send(Command::Terminate).await?;

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            warn!("Force exit from message sender");
        }
        ret = msg_sender => {
            ret??;
        }
    }

    Ok(())
}

/// Convenience wrapper: load config from `path` then call [`run`].
pub async fn run_from_config_file<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<()> {
    let config = Config::new(path)?;
    run(config).await
}
