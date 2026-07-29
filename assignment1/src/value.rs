//! ロードした設定を保持する値の表現。

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

/// 設定値。
///
/// sysctl.conf は型を持たないため、葉の値は常に文字列として保持する。
/// `log.file` のように `.` を含むキーは [`Value::Table`] による入れ子になる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// `token = value` の値。
    String(String),
    /// 入れ子になった設定。
    Table(Table),
}

impl Value {
    /// 文字列であればその中身を返す。
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            Self::Table(_) => None,
        }
    }

    /// 入れ子であればそのテーブルを返す。
    pub fn as_table(&self) -> Option<&Table> {
        match self {
            Self::Table(t) => Some(t),
            Self::String(_) => None,
        }
    }

    /// 入れ子かどうか。
    pub fn is_table(&self) -> bool {
        matches!(self, Self::Table(_))
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::String(s) => serializer.serialize_str(s),
            Self::Table(table) => table.serialize(serializer),
        }
    }
}

/// キーと値の対応。
///
/// 設定ファイルに現れた順序を保つため、挿入順のベクタで保持する。
/// 既存キーへの再代入では順序は変わらず値だけが上書きされる。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Table {
    entries: Vec<(String, Value)>,
}

impl Table {
    /// 空のテーブルを作る。
    pub fn new() -> Self {
        Self::default()
    }

    /// 直下のキーを引く。
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// 挿入順に要素を走査する。
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// 挿入順にキーを走査する。
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(k, _)| k.as_str())
    }

    /// JSON 表現 (1 行) を返す。
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Table is always serializable")
    }

    /// 整形した JSON 表現を返す。
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("Table is always serializable")
    }

    /// キーに値を設定する。
    pub(crate) fn insert(&mut self, key: &str, value: Value) {
        if let Some((_, existing)) = self.entries.iter_mut().find(|(k, _)| k == key) {
            *existing = value;
        } else {
            self.entries.push((key.to_string(), value));
        }
    }

    /// 子テーブルを取得し、存在しなければ作る。
    ///
    /// キーが文字列を保持している場合は入れ子にできないため `None` を返す。
    pub(crate) fn child_table(&mut self, key: &str) -> Option<&mut Table> {
        if let Some(i) = self.entries.iter().position(|(k, _)| k == key) {
            match &mut self.entries[i].1 {
                Value::Table(table) => Some(table),
                Value::String(_) => None,
            }
        } else {
            self.entries
                .push((key.to_string(), Value::Table(Table::new())));
            match &mut self.entries.last_mut().expect("just pushed").1 {
                Value::Table(table) => Some(table),
                Value::String(_) => unreachable!(),
            }
        }
    }
}

impl Serialize for Table {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.entries.len()))?;
        for (key, value) in &self.entries {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_keeps_original_position_when_overwriting() {
        let mut table = Table::new();
        table.insert("a", Value::String("1".into()));
        table.insert("b", Value::String("2".into()));
        table.insert("a", Value::String("3".into()));

        assert_eq!(table.keys().collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(table.get("a").and_then(Value::as_str), Some("3"));
    }

    #[test]
    fn child_table_creates_missing_entry() {
        let mut table = Table::new();
        table
            .child_table("log")
            .expect("newly created entry is a table")
            .insert("file", Value::String("/var/log/console.log".into()));

        let log = table.get("log").and_then(Value::as_table).expect("nested");
        assert_eq!(
            log.get("file").and_then(Value::as_str),
            Some("/var/log/console.log")
        );
    }

    #[test]
    fn child_table_rejects_scalar_entry() {
        let mut table = Table::new();
        table.insert("log", Value::String("true".into()));
        assert!(table.child_table("log").is_none());
    }

    #[test]
    fn serde_json_preserves_insertion_order() {
        let mut root = Table::new();
        root.insert("endpoint", Value::String("localhost:3000".into()));
        root.child_table("log")
            .expect("log is a table")
            .insert("file", Value::String("/var/log/console.log".into()));

        assert_eq!(
            root.to_json(),
            r#"{"endpoint":"localhost:3000","log":{"file":"/var/log/console.log"}}"#
        );
    }
}
