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

use crate::configure::{Config, Repository};
use crate::datastructures::{DisplayableEvent, GitHubPingEvent, GitHubPushEvent, Response};
use actix_web::http::Method;
use actix_web::web::Data;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use hmac::{Hmac, Mac};
use log::{debug, error, info, warn};
use sha2::Sha256;
use std::collections::HashMap;
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
    #[deprecated(since = "2.1.0", note = "You should use Bundle instead send GitHub Data directly, this field will removed in next version.")]
    Data(Box<dyn DisplayableEvent>),
    Bundle((Vec<i64>, String)),
}

struct ExtraData {
    bot_tx: mpsc::Sender<Command>,
}

async fn process_send_message(
    bot_token: String,
    api_server: Option<String>,
    receiver: Vec<i64>,
    specify_configures: HashMap<String, Repository>,
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
            Command::Data(event) => {
                if let Some(repository) = specify_configures.get(event.get_full_name()) {
                    if repository.branch_ignore().contains(&event.branch_name()) {
                        continue;
                    }
                    let target = if repository.send_to().is_empty() {
                        receiver.clone()
                    } else {
                        repository.send_to().clone()
                    };
                    for send_to in target {
                        let mut payload = bot.send_message(send_to, event.to_string());
                        payload.disable_web_page_preview = Option::from(true);
                        if let Err(e) = payload.send().await {
                            error!("Got error in send message {:?}", e);
                        }
                    }
                } else {
                    for send_to in receiver.clone() {
                        let mut payload = bot.send_message(send_to, event.to_string());
                        payload.disable_web_page_preview = Option::from(true);
                        if let Err(e) = payload.send().await {
                            error!("Got error in send message {:?}", e);
                        }
                    }
                }
            }
            Command::Bundle((receiver, text)) => {
                for send_to in receiver {
                    let mut payload = bot.send_message(send_to, &text);
                    payload.disable_web_page_preview = Option::from(true);
                    if let Err(e) = payload.send().await {
                        error!("Got error in send message {:?}", e);
                    }
                }
            },
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

    let sender = data.lock().await;
    let secrets =configure.server().secrets();
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

    if let Some(event) = request.headers().get("X-GitHub-Event") {
        let event = event.to_str();
        if let Err(ref e) = event {
            error!("Parse X-GitHub-Event error: {:?}", e);
            return Ok(HttpResponse::InternalServerError().finish());
        }
        let event = event.unwrap();
        match event {
            "ping" => {
                let request_body = serde_json::from_slice::<GitHubPingEvent>(&body)?;
                Ok(HttpResponse::Ok().json(Response::reason(200, request_body.zen())))
            }
            "push" => {
                let request_body = serde_json::from_slice::<GitHubPushEvent>(&body)?;
                if check_0(request_body.after()) || check_0(request_body.before())
                {
                    return Ok(HttpResponse::NoContent().finish());
                }
                sender
                    .bot_tx
                    .send(Command::Data(Box::new(request_body)))
                    .await
                    .unwrap();
                Ok(HttpResponse::Ok().json(Response::new_ok()))
            }
            _ => Ok(HttpResponse::BadRequest().json(Response::reason(
                400,
                format!("Unsupported event type {:?}", event),
            ))),
        }
    } else {
        error!("Unknown request: {:?}", request);
        Ok(HttpResponse::InternalServerError().finish())
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
        config.telegram().send_to().clone(),
        config.repo_mapping().clone(),
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
