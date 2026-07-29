//! ロード結果を保持する [`SysctlConfig`]。

use crate::value::{Table, Value};

/// ファイルまたは文字列をロードした結果。
///
/// 値は `.` 区切りのキーに沿った入れ子 ([`Table`]) として保持され、
/// [`SysctlConfig::get`] のようにドット記法でも引ける。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SysctlConfig {
    root: Table,
}

impl SysctlConfig {
    /// 空の設定を作る。
    pub fn new() -> Self {
        Self::default()
    }

    /// 最上位のテーブルを借用する。
    pub fn root(&self) -> &Table {
        &self.root
    }

    /// `log.file` のようなドット記法で値を引く。
    ///
    /// `log` のように中間のキーを渡した場合は [`Value::Table`] が返る。
    pub fn get(&self, key: &str) -> Option<&Value> {
        let mut segments = key.split('.');
        let mut value = self.root.get(segments.next()?)?;
        for segment in segments {
            value = value.as_table()?.get(segment)?;
        }
        Some(value)
    }

    /// ドット記法で文字列の値を引く。中間のキーを指定した場合は `None`。
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Value::as_str)
    }

    /// 葉の値を `log.file` のようなドット記法のキーと対にして、出現順に並べる。
    pub fn flatten(&self) -> Vec<(String, &str)> {
        let mut flat = Vec::new();
        push_flattened(&self.root, "", &mut flat);
        flat
    }

    /// 入れ子構造を 1 行の JSON にする。
    pub fn to_json(&self) -> String {
        self.root.to_json()
    }

    /// 入れ子構造を整形した JSON にする。
    pub fn to_json_pretty(&self) -> String {
        self.root.to_json_pretty()
    }

    /// キーを `.` で分割しながら値を格納する。
    pub(crate) fn insert(&mut self, key: &str, value: &str) -> Result<(), String> {
        let mut segments = key.split('.').peekable();
        let mut table = &mut self.root;
        let mut visited = String::new();

        while let Some(segment) = segments.next() {
            if segment.is_empty() {
                return Err("key contains an empty segment".to_string());
            }
            if !visited.is_empty() {
                visited.push('.');
            }
            visited.push_str(segment);

            if segments.peek().is_some() {
                table = table.child_table(segment).ok_or_else(|| {
                    format!("key `{visited}` already holds a value and cannot have children")
                })?;
            } else {
                if table.get(segment).is_some_and(Value::is_table) {
                    return Err(format!(
                        "key `{visited}` already holds child keys and cannot hold a value"
                    ));
                }
                table.insert(segment, Value::String(value.to_string()));
            }
        }

        Ok(())
    }
}

fn push_flattened<'a>(table: &'a Table, prefix: &str, flat: &mut Vec<(String, &'a str)>) {
    for (key, value) in table.iter() {
        let path = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            Value::String(text) => flat.push((path, text.as_str())),
            Value::Table(child) => push_flattened(child, &path, flat),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_of(pairs: &[(&str, &str)]) -> SysctlConfig {
        let mut config = SysctlConfig::new();
        for (key, value) in pairs {
            config.insert(key, value).expect("insertable key");
        }
        config
    }

    #[test]
    fn dotted_keys_build_nested_tables() {
        let config = config_of(&[
            ("log.file", "/var/log/console.log"),
            ("log.name", "default.log"),
        ]);

        let log = config
            .get("log")
            .and_then(Value::as_table)
            .expect("nested table");
        assert_eq!(log.keys().collect::<Vec<_>>(), ["file", "name"]);
        assert_eq!(config.get_str("log.name"), Some("default.log"));
    }

    #[test]
    fn get_str_returns_none_for_intermediate_key() {
        let config = config_of(&[("log.file", "/var/log/console.log")]);
        assert_eq!(config.get_str("log"), None);
        assert_eq!(config.get("missing"), None);
    }

    #[test]
    fn scalar_key_cannot_gain_children() {
        let mut config = config_of(&[("log", "true")]);
        assert_eq!(
            config.insert("log.file", "/var/log/console.log"),
            Err("key `log` already holds a value and cannot have children".to_string())
        );
    }

    #[test]
    fn nested_key_cannot_become_scalar() {
        let mut config = config_of(&[("log.file", "/var/log/console.log")]);
        assert_eq!(
            config.insert("log", "true"),
            Err("key `log` already holds child keys and cannot hold a value".to_string())
        );
    }

    #[test]
    fn empty_segments_are_rejected() {
        let mut config = SysctlConfig::new();
        for key in [".log", "log.", "log..file", ""] {
            assert_eq!(
                config.insert(key, "x"),
                Err("key contains an empty segment".to_string()),
                "key `{key}` must be rejected"
            );
        }
    }
}
