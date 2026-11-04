use serde_json::{Map, Value};

use drawcraft_document::{PageId, ShapeId};

use crate::{EditError, Result};

/// Typed, allocation-free access to JSON command parameters.
#[derive(Clone, Copy, Debug)]
pub struct Params<'a> {
    map: &'a Map<String, Value>,
}

impl<'a> Params<'a> {
    pub fn new(value: &'a Value) -> Result<Self> {
        match value {
            Value::Object(map) => Ok(Self { map }),
            Value::Null => Ok(Self { map: empty_map() }),
            _ => Err(EditError::InvalidParam {
                name: "params",
                reason: "expected a JSON object".into(),
            }),
        }
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&'a Value> {
        self.map.get(key)
    }

    pub fn required(&self, key: &'static str) -> Result<&'a Value> {
        self.get(key).ok_or(EditError::MissingParam(key))
    }

    pub fn string(&self, key: &'static str) -> Result<&'a str> {
        self.required(key)?
            .as_str()
            .ok_or_else(|| EditError::InvalidParam {
                name: key,
                reason: "expected a string".into(),
            })
    }

    pub fn string_or(&self, key: &str, default: &'a str) -> &'a str {
        self.get(key).and_then(Value::as_str).unwrap_or(default)
    }

    pub fn optional_string(&self, key: &str) -> Option<&'a str> {
        self.get(key).and_then(Value::as_str)
    }

    pub fn number(&self, key: &'static str) -> Result<f64> {
        self.required(key)?
            .as_f64()
            .ok_or_else(|| EditError::InvalidParam {
                name: key,
                reason: "expected a number".into(),
            })
    }

    pub fn number_or(&self, key: &str, default: f64) -> f64 {
        self.get(key).and_then(Value::as_f64).unwrap_or(default)
    }

    pub fn integer(&self, key: &'static str) -> Result<i64> {
        self.required(key)?
            .as_i64()
            .ok_or_else(|| EditError::InvalidParam {
                name: key,
                reason: "expected an integer".into(),
            })
    }

    pub fn integer_or(&self, key: &str, default: i64) -> i64 {
        self.get(key).and_then(Value::as_i64).unwrap_or(default)
    }

    pub fn boolean_or(&self, key: &str, default: bool) -> bool {
        self.get(key).and_then(Value::as_bool).unwrap_or(default)
    }

    pub fn shape_id(&self, key: &'static str) -> Result<ShapeId> {
        Ok(ShapeId::from(self.string(key)?))
    }

    pub fn optional_shape_id(&self, key: &str) -> Option<ShapeId> {
        self.optional_string(key)
            .filter(|value| !value.is_empty())
            .map(ShapeId::from)
    }

    pub fn page_id(&self, key: &'static str) -> Result<PageId> {
        Ok(PageId::from(self.string(key)?))
    }

    pub fn optional_page_id(&self, key: &str) -> Option<PageId> {
        self.optional_string(key)
            .filter(|value| !value.is_empty())
            .map(PageId::from)
    }

    pub fn color(&self, key: &'static str) -> Result<String> {
        Ok(self.string(key)?.to_owned())
    }
}

// A stable empty map for the `Null` case. `serde_json` does not expose one.
fn empty_map() -> &'static Map<String, Value> {
    use std::sync::OnceLock;
    static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
    EMPTY.get_or_init(Map::new)
}
