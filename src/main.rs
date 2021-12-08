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

use std::sync::Arc;
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};
use actix_web::http::Method;
use tokio::sync::{mpsc, Mutex};
use log::{debug, error, info};
use teloxide::Bot;
use teloxide::prelude::{Request, Requester, RequesterExt};
use teloxide::types::ParseMode;
use crate::Command::Text;
use crate::datastructures::Response;

mod configure;
mod datastructures;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
enum Command {
    Terminate,
    Text(String),
}

struct ExtraData {
    bot_tx: mpsc::Sender<Command>,
}

async fn process_send_message(
    bot_token: String,
    api_server: Option<String>,
    owner: i64,
    mut rx: mpsc::Receiver<Command>,
) -> anyhow::Result<()> {
    if bot_token.is_empty() {
        info!("Token is empty, skipped all send message request.");
        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::Terminate => break,
                _ => {}
            }
        }
        return Ok(())
    }
    let bot = Bot::new(bot_token);
    let bot = match api_server {
        Some(api) => bot.set_api_url(api.parse()?),
        None => bot,
    };
    let bot = bot.parse_mode(ParseMode::Html);
    while let Some(cmd) = rx.recv().await {
        match cmd {
            Command::Text(text) => {
                if let Err(e) = bot.send_message(owner, text).send().await {
                    error!("Got error in send message {:?}", e);
                }
            }
            Command::Terminate => break,
        }
    }
    debug!("Send message daemon exiting...");
    Ok(())
}


async fn route_post(
    _req: HttpRequest,
    payload: web::Json<datastructures::Request>,
    data: web::Data<Arc<Mutex<ExtraData>>>,
) -> actix_web::Result<HttpResponse> {
    let d = data.lock().await;
    d.bot_tx.send(Text(payload.to_string())).await.unwrap();
    Ok(HttpResponse::Ok().json(Response::new_ok()))
}



async fn async_main() -> anyhow::Result<()> {
    let config = crate::configure::Config::new("data/config.toml")?;

    let (bot_tx, bot_rx) = mpsc::channel(1024);

    let authorization_guard = crate::datastructures::AuthorizationGuard::from(config.server().token());
    let bind_addr = config.get_bind_params();

    let extra_data = Arc::new(Mutex::new(ExtraData {
        bot_tx: bot_tx.clone(),
    }));
    let msg_sender = tokio::spawn(process_send_message(
        config.telegram().bot_token().to_string(),
        config.telegram().api_server().clone(),
        config.telegram().owner(),
        bot_rx,
    ));

    info!("Bind address: {}", &bind_addr);

    let server = tokio::spawn(
        HttpServer::new(move || {
            App::new()
                .wrap(actix_web::middleware::Logger::default())
                .service(
                    web::scope("/")
                        .guard(authorization_guard.to_owned())
                        .data(extra_data.clone())
                        .route("", web::method(Method::POST).to(route_post)),
                )
                .service(
                    web::scope("/")
                    .route(
                    "",
                    web::method(Method::GET).to(|| HttpResponse::Ok().json(Response::new_ok())),
                ))
                .route("/", web::to(HttpResponse::Forbidden))
        })
            .bind(&bind_addr)?
            .run(),
    );

    server.await??;
    bot_tx.send(Command::Terminate).await?;
    msg_sender.await??;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_default_env()
        .init();

    clap::App::new("github-webhook-notification")
        .version(SERVER_VERSION)
        .get_matches();

    let system = actix::System::new();
    info!("Server version: {}", SERVER_VERSION);

    system.block_on(async_main())?;

    system.run()?;

    Ok(())
}