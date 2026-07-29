//! 入力値を検証するスキーマ。
//!
//! スキーマファイルは 1 行に 1 つのキーと型を `key -> type` の形式で書く。
//! 空行と `#` / `;` で始まる行は無視する。
//!
//! ```text
//! endpoint -> string
//! debug -> bool
//! log.file -> string
//! retry -> integer
//! ```

use std::fmt;
use std::path::Path;

use crate::config::SysctlConfig;
use crate::parser::ParseError;
use crate::value::Value;

/// スキーマが要求する値の型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    /// 文字列。
    String,
    /// `true` または `false`。
    Bool,
    /// 整数。
    Integer,
}

impl ValueType {
    /// スキーマファイルの型名から変換する。
    fn parse(text: &str) -> Option<Self> {
        match text {
            "string" => Some(Self::String),
            "bool" => Some(Self::Bool),
            "integer" => Some(Self::Integer),
            _ => None,
        }
    }

    /// 値の見た目から型を判定する。
    ///
    /// sysctl.conf の値はすべて文字列なので、`true` / `false` は [`ValueType::Bool`]、
    /// 整数として読める値は [`ValueType::Integer`]、それ以外を [`ValueType::String`] とみなす。
    fn infer(value: &str) -> Self {
        if matches!(value, "true" | "false") {
            Self::Bool
        } else if value.parse::<i64>().is_ok() {
            Self::Integer
        } else {
            Self::String
        }
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::String => "string",
            Self::Bool => "bool",
            Self::Integer => "integer",
        })
    }
}

/// スキーマに従っていない箇所をまとめたエラー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError(pub Vec<String>);

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("schema validation failed:")?;
        for message in &self.0 {
            write!(f, "\n  - {message}")?;
        }
        Ok(())
    }
}

/// キーと期待する型の一覧。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schema {
    entries: Vec<(String, ValueType)>,
}

impl Schema {
    /// スキーマを文字列からパースする。
    pub fn parse_str(input: &str) -> Result<Self, ParseError> {
        let mut entries: Vec<(String, ValueType)> = Vec::new();

        for (index, raw_line) in input.lines().enumerate() {
            let line_number = index + 1;
            let line = if index == 0 {
                raw_line.trim_start_matches('\u{feff}')
            } else {
                raw_line
            };

            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }

            let error = |message: &str| ParseError::new(line_number, line, message);

            let Some((raw_key, raw_type)) = trimmed.split_once("->") else {
                return Err(error("missing `->` separator"));
            };

            let key = raw_key.trim();
            if key.is_empty() {
                return Err(error("key is empty"));
            }
            if key.split('.').any(str::is_empty) {
                return Err(error("key contains an empty segment"));
            }
            if entries.iter().any(|(existing, _)| existing == key) {
                return Err(error("duplicate key"));
            }

            let value_type = ValueType::parse(raw_type.trim())
                .ok_or_else(|| error("unknown type (expected string, bool or integer)"))?;
            entries.push((key.to_string(), value_type));
        }

        Ok(Self { entries })
    }

    /// スキーマをファイルからパースする。
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        Self::parse_str(&contents)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))
    }

    /// ロードした設定がスキーマに従っているか検証する。
    ///
    /// 不備は 1 件目で打ち切らず、すべて集めて返す。
    pub fn validate(&self, config: &SysctlConfig) -> Result<(), ValidationError> {
        let mut messages = Vec::new();

        for (key, expected) in &self.entries {
            match config.get(key) {
                None => messages.push(format!("key `{key}`: required by the schema but missing")),
                Some(Value::Table(_)) => messages.push(format!(
                    "key `{key}`: expected {expected}, but got nested keys"
                )),
                Some(Value::String(value)) => {
                    let actual = ValueType::infer(value);
                    if actual != *expected {
                        messages.push(format!(
                            "key `{key}`: expected {expected}, but got {actual} (`{value}`)"
                        ));
                    }
                }
            }
        }

        for (key, _) in config.flatten() {
            if !self.entries.iter().any(|(defined, _)| *defined == key) {
                messages.push(format!("key `{key}`: not defined in the schema"));
            }
        }

        if messages.is_empty() {
            Ok(())
        } else {
            Err(ValidationError(messages))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_str;

    const SCHEMA: &str = "\
# コメントは無視する
endpoint -> string
debug -> bool
log.file -> string
log.name -> string
retry -> integer
";

    fn schema() -> Schema {
        Schema::parse_str(SCHEMA).expect("valid schema")
    }

    #[test]
    fn schema_is_parsed_in_order() {
        let parsed = schema();
        assert_eq!(
            parsed.entries,
            vec![
                ("endpoint".to_string(), ValueType::String),
                ("debug".to_string(), ValueType::Bool),
                ("log.file".to_string(), ValueType::String),
                ("log.name".to_string(), ValueType::String),
                ("retry".to_string(), ValueType::Integer),
            ]
        );
    }

    #[test]
    fn missing_separator_is_an_error() {
        let err = Schema::parse_str("endpoint string\n").expect_err("line 1 is invalid");
        assert_eq!(
            err.to_string(),
            "line 1: missing `->` separator (`endpoint string`)"
        );
    }

    #[test]
    fn unknown_type_is_an_error() {
        let err = Schema::parse_str("retry -> number\n").expect_err("number is unknown");
        assert_eq!(
            err.to_string(),
            "line 1: unknown type (expected string, bool or integer) (`retry -> number`)"
        );
    }

    #[test]
    fn empty_key_is_an_error() {
        let err = Schema::parse_str("-> string\n").expect_err("key is missing");
        assert_eq!(err.to_string(), "line 1: key is empty (`-> string`)");
    }

    #[test]
    fn duplicated_key_is_an_error() {
        let err =
            Schema::parse_str("debug -> bool\ndebug -> string\n").expect_err("debug appears twice");
        assert_eq!(err.to_string(), "line 2: duplicate key (`debug -> string`)");
    }

    #[test]
    fn config_following_the_schema_is_accepted() {
        let config = parse_str(
            "\
endpoint = localhost:3000
debug = true
log.file = /var/log/console.log
log.name = default.log
retry = 3
",
        )
        .expect("valid config");

        assert_eq!(schema().validate(&config), Ok(()));
    }

    #[test]
    fn every_violation_is_reported() {
        let config = parse_str(
            "\
endpoint = 123
debug = dummy
log.file = 123
retry = dummy
",
        )
        .expect("valid syntax");

        let err = schema().validate(&config).expect_err("values are invalid");
        assert_eq!(
            err.0,
            vec![
                "key `endpoint`: expected string, but got integer (`123`)",
                "key `debug`: expected bool, but got string (`dummy`)",
                "key `log.file`: expected string, but got integer (`123`)",
                "key `log.name`: required by the schema but missing",
                "key `retry`: expected integer, but got string (`dummy`)",
            ]
        );
    }

    #[test]
    fn keys_outside_the_schema_are_reported() {
        let config = parse_str("debug = true\nlog.level = info\n").expect("valid syntax");
        let err = Schema::parse_str("debug -> bool\n")
            .expect("valid schema")
            .validate(&config)
            .expect_err("log.level is not defined");

        assert_eq!(err.0, vec!["key `log.level`: not defined in the schema"]);
    }

    #[test]
    fn nested_keys_where_a_value_is_expected_are_reported() {
        let config = parse_str("log.file = /var/log/console.log\n").expect("valid syntax");
        let err = Schema::parse_str("log -> string\n")
            .expect("valid schema")
            .validate(&config)
            .expect_err("log holds nested keys");

        assert_eq!(
            err.0,
            vec![
                "key `log`: expected string, but got nested keys",
                "key `log.file`: not defined in the schema",
            ]
        );
    }

    #[test]
    fn validation_error_is_displayed_as_a_list() {
        let err = ValidationError(vec!["first".to_string(), "second".to_string()]);
        assert_eq!(
            err.to_string(),
            "schema validation failed:\n  - first\n  - second"
        );
    }

    #[test]
    fn example_files_show_success_and_failure() {
        let schema = Schema::parse_file("config/schema.conf").expect("schema file is valid");

        let valid = crate::parse_file("config/example1.conf").expect("example1.conf is valid");
        assert_eq!(schema.validate(&valid), Ok(()));

        let invalid = crate::parse_file("config/example2.conf").expect("example2.conf parses");
        assert_eq!(schema.validate(&invalid).expect_err("errors").0.len(), 5);
    }
}
