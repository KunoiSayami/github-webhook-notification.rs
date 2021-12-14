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
use actix_web::http::Method;
use actix_web::web::Data;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use hmac::{Hmac, Mac};
use log::{debug, error, info, warn};
use sha2::Sha256;
use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;
use teloxide::prelude::{Request, Requester, RequesterExt, StreamExt};
use teloxide::types::ParseMode;
use teloxide::Bot;
use tokio::sync::{mpsc, Mutex};

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
                    let mut payload = bot.send_message(*send_to, bundle.text());
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
    request: HttpRequest,
    mut payload: web::Payload,
    configure: web::Data<Config>,
    data: web::Data<Arc<Mutex<ExtraData>>>,
) -> actix_web::Result<HttpResponse> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;

        if (body.len() + chunk.len()) > 262_144 {
            return Ok(HttpResponse::BadRequest().json(Response::reason(400, "overflow")));
        }

        body.extend_from_slice(&chunk);
    }

    let body = body;

    let sender = data.lock().await;
    let object = serde_json::from_slice::<GitHubEarlyParse>(&body);
    if let Err(ref e) = object {
        error!("Get parser error in pre-check stage: {:?}", &e);
        error!("Raw data => {:?}", &body);
        return Ok(HttpResponse::InternalServerError().finish());
    };
    let object = object?;
    let settings = configure
        .fetch_repository_configure(object.get_full_name());

    let secrets = settings.secrets();
    if !secrets.is_empty() {
        type HmacSha256 = Hmac<Sha256>;
        let mut h = HmacSha256::new_from_slice(secrets.as_bytes()).unwrap();
        h.update(&*body);
        let result = h.finalize();
        let sha256val = format!("sha256={:x}", result.into_bytes()).to_lowercase();
        if let Some(val) = request.headers().get("X-Hub-Signature-256") {
            if !sha256val.eq(val) {
                return Ok(HttpResponse::Forbidden().json(Response::reason(403, "Checksum error")));
            }
        } else {
            return Ok(
                HttpResponse::Forbidden().json(Response::reason(403, "Checksum header not found"))
            );
        }
    }

    let event_header = request.headers().get("X-GitHub-Event");
    if event_header.is_none() {
        error!("Unknown request: {:?}", request);
        return Ok(HttpResponse::InternalServerError().finish());
    }
    let event_header = event_header.unwrap().to_str();
    if let Err(ref e) = event_header {
        error!("Parse X-GitHub-Event error: {:?}", e);
        return Ok(HttpResponse::InternalServerError().finish());
    }
    let event_header = event_header.unwrap();
    match event_header {
        "ping" => {
            let request_body = serde_json::from_slice::<GitHubPingEvent>(&body)?;
            Ok(HttpResponse::Ok().json(Response::reason(200, request_body.zen())))
        }
        "push" => {
            let event = serde_json::from_slice::<GitHubPushEvent>(&body)?;
            if check_0(event.after()) || check_0(event.before()) {
                return Ok(HttpResponse::NoContent().finish());
            }
            if settings.branch_ignore().contains(&event.branch_name()) {
                Ok(HttpResponse::Ok().json(Response::reason(204, "Skipped.")))
            } else {
                sender
                    .bot_tx
                    .send(Command::Bundle(CommandBundle::new(
                        settings.send_to().clone(),
                        event.to_string(),
                    )))
                    .await
                    .unwrap();
                Ok(HttpResponse::Ok().json(Response::new_ok()))
            }
        }
        _ => Ok(HttpResponse::BadRequest().json(Response::reason(
            400,
            format!("Unsupported event type {:?}", event_header),
        ))),
    }
}

async fn async_main<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    let config = crate::configure::Config::new(path)?;

    let (bot_tx, bot_rx) = mpsc::channel(1024);

    let authorization_guard =
        crate::datastructures::AuthorizationGuard::from(config.server().token());

    let extra_data = Arc::new(Mutex::new(ExtraData {
        bot_tx: bot_tx.clone(),
    }));
    let msg_sender = tokio::spawn(process_send_message(
        config.telegram().bot_token().to_string(),
        config.telegram().api_server().clone(),
        bot_rx,
    ));

    let bind = config.server().bind().clone();
    info!("Bind address: {}", bind);

    let server = tokio::spawn(
        HttpServer::new(move || {
            App::new()
                .wrap(actix_web::middleware::Logger::default())
                .service(
                    web::scope("/")
                        .guard(authorization_guard.to_owned())
                        .app_data(Data::new(config.clone()))
                        .app_data(Data::new(extra_data.clone()))
                        .route("", web::method(Method::POST).to(route_post)),
                )
                .service(web::scope("/").route(
                    "",
                    web::method(Method::GET).to(|| HttpResponse::Ok().json(Response::new_ok())),
                ))
                .route("/", web::to(HttpResponse::Forbidden))
        })
        .bind(bind)?
        .run(),
    );

    server.await??;
    bot_tx.send(Command::Terminate).await?;
    msg_sender.await??;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_module("rustls::client", log::LevelFilter::Warn)
        .init();

    let arg_matches = clap::App::new("github-webhook-notification")
        .arg(
            clap::Arg::with_name("cfg")
                .long("cfg")
                .short("c")
                .default_value("data/config.toml")
                .help("Specify configure file location")
                .takes_value(true),
        )
        .version(SERVER_VERSION)
        .get_matches();

    let system = actix::System::new();
    info!("Server version: {}", SERVER_VERSION);

    system.block_on(async_main(arg_matches.value_of("cfg").unwrap()))?;

    system.run()?;

    Ok(())
}
