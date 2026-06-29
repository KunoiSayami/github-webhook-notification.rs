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

use clap::arg;
use github_webhook_notification::server::run_from_config_file;
use log::info;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        .block_on(run_from_config_file(
            arg_matches
                .get_one::<String>("cfg")
                .map(|s| s.as_str())
                .unwrap_or("data/config.toml"),
        ))?;

    Ok(())
}
