# 課題プロジェクト（Rust言語）

# 課題内容
## linuxのsysctl.confと同じ文法の任意のファイルをロードし、辞書型・Map等に格納するプログラムを作成してください。

- sysctl.confの文法についてはこちらをご確認ください

https://man7.org/linux/man-pages/man5/sysctl.conf.5.html

- プログラムはライブラリとしてファイルをParseするのに利用する想定と考えてください。任意のファイルをロードし、変数に格納できるようにしたいです。
- 担当者から指定がなければ、Go / Scala / Rust / C# のいずれかで実装してください

## 入力例１

```
endpoint = localhost:3000
debug = true
log.file = /var/log/console.log
```

## 入力例２

```
endpoint = localhost:3000
# debug = true
log.file = /var/log/console.log
log.name = default.log
```

## 格納した変数の中身の期待値

(※) わかりやすさのために出力例をJson形式で書いていますが、Json形式で出力する必要はありません。

## 入力例１の出力例

```json
{
  "endpoint": "localhost:3000",
  "debug": "true",
  "log": {
    "file": "/var/log/console.log"
  }
}
```

## 入力例2の出力例

```json
{
  "endpoint": "localhost:3000",
  "log": {
    "file": "/var/log/console.log",
    "name": "default.log",
  }
}
```

# 実装

パース処理は**ライブラリ**（`sysctl_conf` クレート）として提供し、
任意のファイルをロードして `Map` 相当のデータ構造に格納します。
CLI はそのライブラリの利用例で、格納した内容を JSON で標準出力に書き出します。

外部クレート: `serde` / `serde_json`（JSON 出力）

## 構成

| パス | 内容 |
| --- | --- |
| `assignment1/src/lib.rs` | ライブラリのエントリポイント。`parse_str` / `parse_file` |
| `assignment1/src/parser.rs` | sysctl.conf 文法の解釈 |
| `assignment1/src/config.rs` | ロード結果 `SysctlConfig` |
| `assignment1/src/value.rs` | 値の表現 `Value` / `Table` |
| `assignment1/src/main.rs` | `config/example2.conf` をロードして JSON を標準出力へ書き出す |
| `assignment1/config/example1.conf`, `assignment1/config/example2.conf` | 入力例ファイル |

## 実行方法

```console
$ cd assignment1
$ cargo run
{
  "endpoint": "localhost:3000",
  "log": {
    "file": "/var/log/console.log",
    "name": "default.log"
  }
}
```

ロードするファイルは `assignment1/src/main.rs` の `CONFIG_PATH`（既定: `config/example2.conf`）にハードコードしています。
パスは `assignment1` ディレクトリ基準です。

## テスト

```console
$ cd assignment1
$ cargo test
```
