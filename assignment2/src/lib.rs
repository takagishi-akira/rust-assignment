//! linux の `sysctl.conf` と同じ文法のファイルをロードし、キーと値の対応として保持するライブラリ。
//!
//! ロードした値はスキーマ ([`Schema`]) で検証でき、型が合わないキーや
//! スキーマに定義がないキーを [`ValidationError`] として報告する。

#![deny(missing_docs)]

mod config;
mod parser;
mod schema;
mod value;

pub use crate::config::SysctlConfig;
pub use crate::parser::{ParseError, Parser};
pub use crate::schema::{Schema, ValidationError, ValueType};
pub use crate::value::{Table, Value};

use std::path::Path;

/// 文字列をパースする。
pub fn parse_str(input: &str) -> Result<SysctlConfig, ParseError> {
    Parser::new().parse_str(input)
}

/// ファイルをパースする。
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<SysctlConfig, std::io::Error> {
    Parser::new().parse_file(path)
}
