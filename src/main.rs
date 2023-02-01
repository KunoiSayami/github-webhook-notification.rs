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
    AuthorizationGuard, CommandBundle, DisplayableEvent, GitHubEarlyParse, GitHubPingEvent,
    GitHubPushEvent, Response,
};
use axum::body::{Body, HttpBody};
use axum::http::{Request as HttpRequest, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Router};
use clap::arg;
use hmac::{Hmac, Mac};
use log::{debug, error, info, warn};
use once_cell::sync::OnceCell;
use sha2::Sha256;
use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;
use teloxide::prelude::{Request, Requester, RequesterExt};
use teloxide::types::{ChatId, ParseMode};
use teloxide::Bot;
use tokio::sync::{mpsc, RwLock};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

static AUTH_TOKEN: OnceCell<String> = OnceCell::new();

mod configure;
mod datastructures;
#[cfg(test)]
mod test;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
enum Command {
    Terminate,
    Bundle(CommandBundle),
}

struct ExtraData {
    bot_tx: mpsc::Sender<Command>,
}

async fn process_send_message(
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
                    let mut payload = bot.send_message(ChatId(*send_to), bundle.text());
                    payload.disable_web_page_preview = Option::from(true);
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

fn check_0(s: &str) -> bool {
    s.chars().into_iter().all(|x| x == '0')
}

async fn route_post(
    mut request: HttpRequest<Body>,
    Extension(configure): Extension<Config>,
    Extension(data): Extension<Arc<RwLock<ExtraData>>>,
) -> impl IntoResponse {
    //let mut body = web::BytesMut::new();
    let mut body: Vec<u8> = Vec::new();
    while let Some(Ok(ref chunk)) = request.body_mut().data().await {
        body.extend(chunk);
        if (body.len() + chunk.len()) > 262_144 {
            return Response::reason(400, "overflow");
        }
    }

    let body = body;

    let object = serde_json::from_slice::<GitHubEarlyParse>(&body);
    if let Err(ref e) = object {
        error!("Get parser error in pre-check stage: {:?}", &e);
        error!("Raw data => {:?}", String::from_utf8_lossy(&body));
        return Response::new(500);
    };
    let object = object.unwrap();
    let settings = configure.fetch_repository_configure(object.get_full_name());

    let secrets = settings.secrets();
    if !secrets.is_empty() {
        type HmacSha256 = Hmac<Sha256>;
        let mut h = HmacSha256::new_from_slice(secrets.as_bytes()).unwrap();
        h.update(&*body);
        let result = h.finalize();
        let sha256val = format!("sha256={:x}", result.into_bytes()).to_lowercase();
        if let Some(val) = request.headers().get("X-Hub-Signature-256") {
            if !sha256val.eq(val) {
                return Response::reason(403, "Checksum error");
            }
        } else {
            return Response::reason(403, "Checksum header not found");
        }
    }

    let event_header = request.headers().get("X-GitHub-Event");
    if event_header.is_none() {
        error!("Unknown request: {:?}", request);
        return Response::new(500);
    }
    let event_header = event_header.unwrap().to_str();
    if let Err(ref e) = event_header {
        error!("Parse X-GitHub-Event error: {:?}", e);
        return Response::new(500);
    }
    let event_header = event_header.unwrap();
    match event_header {
        "ping" => {
            let request_body = match serde_json::from_slice::<GitHubPingEvent>(&body) {
                Ok(ret) => ret,
                Err(e) => return Response::new_parse_error(e),
            };
            Response::reason(200, request_body.zen())
        }
        "push" => {
            let event = match serde_json::from_slice::<GitHubPushEvent>(&body) {
                Ok(ret) => ret,
                Err(e) => return Response::new_parse_error(e),
            };
            if check_0(event.after()) || check_0(event.before()) {
                return Response::new_empty();
            }
            if settings.branch_ignore().contains(&event.branch_name()) {
                Response::reason(204, "Skipped.")
            } else {
                let sender = data.write().await;
                sender
                    .bot_tx
                    .send(Command::Bundle(CommandBundle::new(
                        settings.send_to().clone(),
                        event.to_string(),
                    )))
                    .await
                    .unwrap();
                Response::new_ok()
            }
        }
        _ => Response::reason(400, format!("Unsupported event type {:?}", event_header)),
    }
}

async fn async_main<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    let config = Config::new(path)?;

    let (bot_tx, bot_rx) = mpsc::channel(1024);

    AUTH_TOKEN.set(config.server().token().to_string()).unwrap();

    let extra_data = Arc::new(RwLock::new(ExtraData {
        bot_tx: bot_tx.clone(),
    }));
    let msg_sender = tokio::spawn(process_send_message(
        config.telegram().bot_token().to_string(),
        config.telegram().api_server().clone(),
        bot_rx,
    ));

    let bind = config.server().bind().clone();
    info!("Bind address: {}", bind);

    let router = Router::new()
        .route(
            "/",
            axum::routing::post(route_post)
                .layer(axum::middleware::from_extractor::<AuthorizationGuard>())
                .layer(Extension(config.clone()))
                .layer(Extension(extra_data.clone())),
        )
        .route("/", axum::routing::get(|| async { Response::new_ok() }))
        .route("/", axum::routing::any(|| async { StatusCode::FORBIDDEN }))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    let handler = axum_server::Handle::new();

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

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_module("rustls::client", log::LevelFilter::Warn)
        .init();

    let arg_matches = clap::Command::new("github-webhook-notification")
        .arg(arg!(-c --cfg <CONFIG> "Specify configure file location"))
        .version(SERVER_VERSION)
        .get_matches();

    info!("Server version: {}", SERVER_VERSION);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(
            arg_matches.value_of("cfg").unwrap_or("data/config.toml"),
        ))?;

    Ok(())
}
