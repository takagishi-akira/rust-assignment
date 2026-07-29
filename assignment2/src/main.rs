//! `sysctl.conf` 形式のファイルをスキーマで検証し、結果を JSON として標準出力に書き出す。

use std::process::ExitCode;

use sysctl_conf::{parse_file, Schema};

/// 検証に使うスキーマファイル。
const SCHEMA_PATH: &str = "config/schema.conf";

/// ロードする設定ファイル。`config/example2.conf` に変えるとエラー出力になる。
const CONFIG_PATH: &str = "config/example2.conf";

fn main() -> ExitCode {
    match run() {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let schema = Schema::parse_file(SCHEMA_PATH).map_err(|err| err.to_string())?;
    let config = parse_file(CONFIG_PATH).map_err(|err| err.to_string())?;
    schema.validate(&config).map_err(|err| err.to_string())?;
    Ok(config.to_json_pretty())
}
