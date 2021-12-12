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

#[allow(dead_code)]
#[cfg(test)]
mod test {
    use crate::configure::Config;
    use actix_web::{App, HttpServer};
    use std::time::Duration;

    #[test]
    fn test_configure() {
        let cfg = Config::new("example/sample.toml").unwrap();
        assert_eq!(cfg.server().bind(), "0.0.0.0:11451");
        assert_eq!(cfg.server().secrets(), "1145141919810");
        assert!(cfg.server().token().is_empty());
        assert_eq!(cfg.telegram().bot_token(), "1145141919:810abcdefg");
        let result = vec![114514, 1919810i64];
        assert_eq!(cfg.telegram().send_to().len(), result.len());
        assert_eq!(
            cfg.telegram()
                .send_to()
                .into_iter()
                .zip(&result)
                .filter(|&(a, b)| a == b)
                .count(),
            result.len()
        );
        let repositories = cfg.repo_mapping();
        assert_eq!(repositories.len(), 2);
        let r1 = repositories.get("114514/1919810");
        assert!(r1.is_some());
        let r1 = r1.unwrap();
        assert!(r1.branch_ignore().is_empty());
        assert!(!r1.send_to().is_empty());
        assert_eq!(r1.send_to().len(), 6);
        let r2 = repositories.get("2147483647/114514");
        assert!(r2.is_some());
        let r2 = r2.unwrap();
        assert_eq!(r2.send_to().len(), 1);
        assert_eq!(r2.branch_ignore().len(), 2);
    }

    async fn server() -> tokio::io::Result<()> {
        let future = HttpServer::new(move || {
            App::new().route(
                "/",
                actix_web::web::to(|| actix_web::web::HttpResponse::Ok().finish()),
            )
        })
        .bind("127.0.0.1:11451")
        .unwrap()
        .run();
        let handler = future.handle();
        let server = tokio::spawn(future);
        tokio::time::sleep(Duration::from_secs(1)).await;
        handler.stop(false).await;
        server.await?
    }

    #[ignore]
    #[test]
    #[ntest::timeout(5000)]
    fn test_server_availability() {
        let system = actix::System::new();
        system.block_on(server()).unwrap();
        system.run().unwrap();
    }
}
