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

#[cfg(test)]
mod test {
    use crate::configure::Config;

    #[test]
    fn test_configure() {
        let cfg = Config::new("example/sample.toml").unwrap();
        assert_eq!(cfg.server().bind(), "0.0.0.0:11451");
        assert_eq!(cfg.server().secrets(), "1145141919810");
        assert!(cfg.server().token().is_empty());
        assert_eq!(cfg.telegram().bot_token(), "1145141919:810abcdefg");
        let result = vec![114514, 1919810i64];
        assert_eq!(cfg.telegram().send_to().len(), result.len());
        assert_eq!(cfg.telegram().send_to().into_iter().zip(&result).filter(|&(a,b)| a == b).count(), result.len());
    }
}