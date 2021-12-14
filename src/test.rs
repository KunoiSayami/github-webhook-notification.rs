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
    use crate::{DisplayableEvent, GitHubEarlyParse, GitHubPingEvent, GitHubPushEvent};
    use walkdir::WalkDir;

    #[test]
    fn test_configure() {
        let cfg = Config::new("example/sample.toml").unwrap();
        assert_eq!(cfg.server().bind(), "127.0.0.1:11451");
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

        // Test first repository
        let repo = cfg.fetch_repository_configure("MonsterSenpai/SummerNight-HornyFantasy");
        assert!(!repo.is_default());
        assert!(repo.branch_ignore().is_empty());
        assert!(!repo.send_to().is_empty());
        assert_eq!(repo.send_to().len(), 6);

        // Test second repository
        let repo = cfg.fetch_repository_configure("BillyKing/Wrestling");
        assert!(!repo.is_default());
        assert_eq!(repo.send_to().len(), 1);
        assert_eq!(repo.branch_ignore().len(), 2);
        assert_eq!(repo.secrets(), cfg.server().secrets());

        // Test third repository
        let repo = cfg.fetch_repository_configure("sample/test");
        assert!(!repo.is_default());
        assert!(repo.branch_ignore().is_empty());
        assert_eq!(
            repo.send_to()
                .into_iter()
                .zip(&result)
                .filter(|(a, b)| a == b)
                .count(),
            result.len()
        );
        assert_eq!(repo.secrets(), "2333");

        // Test not exist repository
        let r_missing = cfg.fetch_repository_configure("114514/1919810");
        assert_eq!(r_missing.secrets(), cfg.server().secrets());
        assert!(r_missing.branch_ignore().is_empty());
        assert_eq!(r_missing.send_to(), cfg.telegram().send_to());
    }

    // src: https://docs.rs/actix-web/4.0.0-beta.14/actix_web/test/struct.TestRequest.html
    #[actix_web::test]
    async fn test_init_service() {
        use actix_web::dev::Service;
        let app = actix_web::test::init_service(
            actix_web::App::new().service(actix_web::web::resource("/test").to(|| async { "OK" })),
        )
        .await;

        // Create request object
        let req = actix_web::test::TestRequest::with_uri("/test").to_request();

        // Execute application
        let resp = app.call(req).await.unwrap();
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
    }

    #[test]
    fn test_parse_ping() {
        let s = std::fs::read_to_string("example/ping.json").unwrap();
        let event: GitHubPingEvent = serde_json::from_str(s.as_str()).unwrap();
        assert_eq!(event.zen(), "Half measures are as bad as nothing at all.");
    }

    #[test]
    fn test_parse_push() {
        let s = std::fs::read_to_string("example/push.json").unwrap();
        let event: GitHubPushEvent = serde_json::from_str(s.as_str()).unwrap();
        assert_eq!(event.repository().full_name(), "MagomeYae/test-action");
        assert_eq!(event.branch_name(), "master");
    }

    #[test]
    fn test_basic_parse() {
        for entry in WalkDir::new("example") {
            let entry = entry.unwrap();
            if entry.path().ends_with(".json") {
                let s = std::fs::read_to_string(entry.path()).unwrap();
                let _event: GitHubEarlyParse = serde_json::from_str(s.as_str()).unwrap();
            }
        }
    }
}
