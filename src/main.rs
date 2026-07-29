//! `sysctl.conf` 形式のファイルをロードし、結果を JSON として標準出力に書き出す。

use std::process::ExitCode;

use sysctl_conf::parse_file;

/// ロードする設定ファイル。
const CONFIG_PATH: &str = "config/example2.conf";

fn main() -> ExitCode {
    match parse_file(CONFIG_PATH) {
        Ok(config) => {
            println!("{}", config.to_json_pretty());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
