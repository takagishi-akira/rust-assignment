# 課題プロジェクト（Rust言語）

# 課題内容
課題1 のプログラム実行時に、入力値に不備がないか検証できるようにしてください。
入力値に不備がないかは、スキーマファイルによって検証することとします。

```
endpoint -> string
debug -> bool
log.file -> string
retry -> integer
```

上述のようなスキーマファイルをユーザーが作成し、parseする対象のファイルとともにロードする事で、スキーマに従っていない場合にはエラーを出力するようにしたいです。
上記のスキーマは一例なので、自由に形式を考えてください。

# 実装

課題1 のパースライブラリに、スキーマによる検証を追加しました。
スキーマファイルと設定ファイルをそれぞれロードし、検証に通った場合のみ JSON を標準出力へ書き出します。
スキーマに従っていない場合は不備を標準エラー出力へ列挙し、終了コード 1 で終了します。

外部クレート: `serde` / `serde_json`（JSON 出力）

## スキーマの形式

1 行に 1 つ、`キー -> 型` の形式で書きます。空行と `#` / `;` で始まる行は無視します。
キーは検証対象と同じドット記法（`log.file`）で指定します。

| 型 | 受け付ける値 |
| --- | --- |
| `string` | `bool` / `integer` として読めない値 |
| `bool` | `true` または `false` |
| `integer` | 整数として読める値 |

sysctl.conf の値はすべて文字列なので、値の見た目から型を判定して型名と厳密に一致するかを見ます。
そのため `endpoint = 123` は `integer` とみなされ、`endpoint -> string` には従っていないと判定されます。

## 検証する内容

- 型が一致しないキー
- スキーマにあるが設定ファイルに無いキー（スキーマのキーはすべて必須）
- 設定ファイルにあるがスキーマに無いキー

不備は 1 件目で打ち切らず、すべて集めてから出力します。

## 構成

| パス | 内容 |
| --- | --- |
| `assignment2/src/lib.rs` | ライブラリのエントリポイント。`parse_str` / `parse_file` |
| `assignment2/src/parser.rs` | sysctl.conf 文法の解釈 |
| `assignment2/src/schema.rs` | スキーマ `Schema` の解釈と検証 |
| `assignment2/src/config.rs` | ロード結果 `SysctlConfig` |
| `assignment2/src/value.rs` | 値の表現 `Value` / `Table` |
| `assignment2/src/main.rs` | 設定ファイルをスキーマで検証し JSON を標準出力へ書き出す |
| `assignment2/config/schema.conf` | スキーマファイル |
| `assignment2/config/example1.conf` | スキーマに従っている入力例 |
| `assignment2/config/example2.conf` | スキーマに従っていない入力例 |

## 実行方法

スキーマに従っている場合は JSON を出力します。

```console
$ cd assignment2
$ cargo run
{
  "endpoint": "localhost:3000",
  "debug": "true",
  "log": {
    "file": "/var/log/console.log",
    "name": "default.log"
  },
  "retry": "3"
}
```

`assignment2/src/main.rs` の `CONFIG_PATH` を `config/example2.conf` に変えると、スキーマ違反の出力を確認できます。

```console
$ cargo run
error: schema validation failed:
  - key `endpoint`: expected string, but got integer (`123`)
  - key `debug`: expected bool, but got string (`dummy`)
  - key `log.file`: expected string, but got integer (`123`)
  - key `log.name`: required by the schema but missing
  - key `retry`: expected integer, but got string (`dummy`)
```

ロードするファイルは `assignment2/src/main.rs` の `SCHEMA_PATH`（既定: `config/schema.conf`）と
`CONFIG_PATH`（既定: `config/example1.conf`）にハードコードしています。
パスは `assignment2` ディレクトリ基準です。

## テスト

```console
$ cd assignment2
$ cargo test
```