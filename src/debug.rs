use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::Value;

static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_enabled(enabled: bool) {
    DEBUG_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn is_enabled() -> bool {
    DEBUG_ENABLED.load(Ordering::Relaxed)
}

pub fn log(message: impl AsRef<str>) {
    if is_enabled() {
        eprintln!("[debug] {}", message.as_ref());
    }
}

pub fn summarize_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let keys = map.keys().take(8).cloned().collect::<Vec<_>>().join(", ");
            let mut parts = vec![format!("keys=[{keys}]")];

            for key in ["items", "members", "value"] {
                if let Some(items) = map.get(key).and_then(Value::as_array) {
                    parts.push(format!("{key}.len={}", items.len()));
                }
            }

            for key in ["count", "totalCount"] {
                if let Some(count) = map.get(key).and_then(Value::as_u64) {
                    parts.push(format!("{key}={count}"));
                }
            }

            if let Some(token) = map.get("continuationToken") {
                parts.push(format!(
                    "continuationToken={}",
                    summarize_scalar(token).unwrap_or_else(|| "<complex>".into())
                ));
            }

            parts.join(", ")
        }
        Value::Array(items) => format!("array(len={})", items.len()),
        _ => summarize_scalar(value).unwrap_or_else(|| "<complex>".into()),
    }
}

pub fn preview_text(text: &str, limit: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview = normalized.chars().take(limit).collect::<String>();
    if normalized.chars().count() > limit {
        preview.push('…');
    }
    preview
}

fn summarize_scalar(value: &Value) -> Option<String> {
    match value {
        Value::Null => Some("null".into()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(preview_text(value, 80)),
        _ => None,
    }
}
