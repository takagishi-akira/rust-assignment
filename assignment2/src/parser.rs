//! sysctl.conf(5) 形式の文法を解釈するパーサ。

use std::fmt;

use crate::config::SysctlConfig;

/// 行番号付きのパースエラー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl ParseError {
    /// エラーを組み立てる。
    pub fn new(line: usize, content: impl Into<String>, message: impl Into<String>) -> Self {
        Self(format!(
            "line {}: {} (`{}`)",
            line,
            message.into(),
            content.into()
        ))
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// パーサ本体。
#[derive(Debug, Clone, Default)]
pub struct Parser;

impl Parser {
    /// パーサを作る。
    pub fn new() -> Self {
        Self
    }

    /// 文字列をパースする。
    pub fn parse_str(&self, input: &str) -> Result<SysctlConfig, ParseError> {
        let mut config = SysctlConfig::new();
        for (index, line) in input.lines().enumerate() {
            let line = if index == 0 {
                line.trim_start_matches('\u{feff}')
            } else {
                line
            };
            apply_line(&mut config, line, index + 1)?;
        }
        Ok(config)
    }

    /// ファイルをパースする。
    pub fn parse_file<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<SysctlConfig, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        self.parse_str(&contents)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))
    }
}

fn apply_line(config: &mut SysctlConfig, line: &str, line_number: usize) -> Result<(), ParseError> {
    let Some((key, value)) =
        classify(line).map_err(|message| ParseError::new(line_number, line, message))?
    else {
        return Ok(());
    };

    config
        .insert(key, value)
        .map_err(|message| ParseError::new(line_number, line, message))
}

/// 1 行を分類する。空行とコメントでは `None` を返す。
fn classify(line: &str) -> Result<Option<(&str, &str)>, &'static str> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
        return Ok(None);
    }

    let (raw_key, raw_value) = trimmed.split_once('=').ok_or("missing `=` separator")?;

    let mut key = raw_key.trim();
    // sysctl.conf の `-token = value` は「設定失敗時のエラー抑止」指定。
    // 値の格納には影響しないため、キーから `-` だけ取り除く。
    if let Some(stripped) = key.strip_prefix('-') {
        key = stripped.trim();
    }
    if key.is_empty() {
        return Err("key is empty");
    }

    Ok(Some((key, raw_value.trim())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> SysctlConfig {
        Parser::new().parse_str(input).expect("valid input")
    }

    #[test]
    fn blank_lines_and_comments_are_ignored() {
        let config =
            parse("\n# debug = true\n  ; endpoint = ignored\n\nendpoint = localhost:3000\n");

        assert_eq!(config.get_str("endpoint"), Some("localhost:3000"));
    }

    #[test]
    fn whitespace_around_token_and_value_is_ignored() {
        let config = parse("\tendpoint   =   localhost:3000\t\n");
        assert_eq!(config.get_str("endpoint"), Some("localhost:3000"));
    }

    #[test]
    fn whitespace_inside_value_is_preserved() {
        let config = parse("kernel.hostname = my host name\n");
        assert_eq!(config.get_str("kernel.hostname"), Some("my host name"));
    }

    #[test]
    fn value_may_contain_equals_and_hash() {
        let config = parse("query = a=b&c=d # not a comment\n");
        assert_eq!(config.get_str("query"), Some("a=b&c=d # not a comment"));
    }

    #[test]
    fn value_may_be_empty() {
        let config = parse("debug =\n");
        assert_eq!(config.get_str("debug"), Some(""));
    }

    #[test]
    fn duplicated_key_keeps_the_last_value() {
        let config = parse("debug = true\ndebug = false\n");
        assert_eq!(config.get_str("debug"), Some("false"));
    }

    #[test]
    fn leading_dash_is_stripped_from_key() {
        let config = parse("-log.file = /var/log/console.log\n");
        assert_eq!(config.get_str("log.file"), Some("/var/log/console.log"));
    }

    #[test]
    fn crlf_and_bom_are_handled() {
        let config = parse("\u{feff}endpoint = localhost:3000\r\ndebug = true\r\n");
        assert_eq!(config.get_str("endpoint"), Some("localhost:3000"));
        assert_eq!(config.get_str("debug"), Some("true"));
    }

    #[test]
    fn missing_separator_is_an_error_with_line_number() {
        let err = Parser::new()
            .parse_str("endpoint = localhost:3000\ndebug true\n")
            .expect_err("line 2 is invalid");
        assert_eq!(
            err.to_string(),
            "line 2: missing `=` separator (`debug true`)"
        );
    }

    #[test]
    fn empty_key_is_an_error() {
        let err = Parser::new()
            .parse_str("= localhost\n")
            .expect_err("key is missing");
        assert_eq!(err.to_string(), "line 1: key is empty (`= localhost`)");
    }

    #[test]
    fn parse_file_reports_missing_file() {
        let err = Parser::new()
            .parse_file("config/does-not-exist.conf")
            .expect_err("file is missing");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
