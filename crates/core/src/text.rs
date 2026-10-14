//! Reader-visible words, from `data/strings.json`. The file is nested JSON;
//! keys are read as dotted paths (`banner.rift_open`). A `{name}` in a
//! string is filled by the core, so a number in the text is always the
//! number the rule uses.

use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct Text {
    map: BTreeMap<String, String>,
    /// Keys asked for that the file lacks. Tests require this to stay empty.
    pub missing: RefCell<Vec<String>>,
}

fn flatten(prefix: &str, v: &serde_json::Value, out: &mut BTreeMap<String, String>) {
    match v {
        serde_json::Value::Object(m) => {
            for (k, v) in m {
                if k.starts_with('_') {
                    continue;
                }
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&key, v, out);
            }
        }
        serde_json::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        _ => {}
    }
}

impl Text {
    pub fn parse(json: &str) -> Result<Text, String> {
        let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("strings.json: {e}"))?;
        let mut map = BTreeMap::new();
        flatten("", &v, &mut map);
        if map.is_empty() {
            return Err("strings.json holds no strings".into());
        }
        Ok(Text {
            map,
            missing: RefCell::new(Vec::new()),
        })
    }

    pub fn has(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// The string for `key`; the key itself, recorded as missing, when the
    /// file has none.
    pub fn get(&self, key: &str) -> String {
        match self.map.get(key) {
            Some(s) => s.clone(),
            None => {
                let mut m = self.missing.borrow_mut();
                if !m.iter().any(|k| k == key) {
                    m.push(key.to_string());
                }
                key.to_string()
            }
        }
    }

    /// The string for `key` with each `{name}` replaced.
    pub fn fill(&self, key: &str, values: &[(&str, String)]) -> String {
        let mut s = self.get(key);
        for (name, value) in values {
            s = s.replace(&format!("{{{name}}}"), value);
        }
        s
    }

    /// A name from the content: `name.<id>`.
    pub fn name(&self, id: &str) -> String {
        self.get(&format!("name.{id}"))
    }
}

/// Seconds as m:ss.
pub fn clock(seconds: i32) -> String {
    let s = seconds.max(0);
    format!("{}:{:02}", s / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_keys_read_as_dotted_paths_and_placeholders_fill() {
        let t = Text::parse(r#"{"a": {"b": "Hello {who}"}, "_note": "skip"}"#).unwrap();
        assert_eq!(t.fill("a.b", &[("who", "Sam".into())]), "Hello Sam");
        assert!(!t.has("_note"));
        assert_eq!(t.get("a.c"), "a.c");
        assert_eq!(*t.missing.borrow(), vec!["a.c".to_string()]);
    }

    #[test]
    fn the_clock_pads_seconds() {
        assert_eq!(clock(65), "1:05");
        assert_eq!(clock(720), "12:00");
    }
}
