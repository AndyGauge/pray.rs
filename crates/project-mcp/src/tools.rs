use serde_json::{json, Value};
use time::OffsetDateTime;

use crate::store::{FeaturePriority, FeatureRequest, FeatureStatus, ProjectStore};

pub fn list_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "add_feature_request",
                "description": "Add a feature request to the backlog.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title":       { "type": "string" },
                        "description": { "type": "string" },
                        "priority":    { "type": "string", "enum": ["low", "medium", "high"] }
                    },
                    "required": ["title", "description"]
                }
            },
            {
                "name": "list_features",
                "description": "List all feature requests. Optionally filter by status and/or priority.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "status":   { "type": "string", "enum": ["open", "in_progress", "done", "declined"] },
                        "priority": { "type": "string", "enum": ["low", "medium", "high"] }
                    }
                }
            },
            {
                "name": "update_feature_status",
                "description": "Update the status of a feature request.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id":     { "type": "string" },
                        "status": { "type": "string", "enum": ["open", "in_progress", "done", "declined"] }
                    },
                    "required": ["id", "status"]
                }
            },
            {
                "name": "add_changelog_entry",
                "description": "Append an entry to the project changelog.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entry": { "type": "string" }
                    },
                    "required": ["entry"]
                }
            },
            {
                "name": "get_changelog",
                "description": "Retrieve the full project changelog.",
                "inputSchema": { "type": "object", "properties": {} }
            }
        ]
    })
}

pub fn call_tool(store: &ProjectStore, name: &str, args: &Value) -> Value {
    match name {
        "add_feature_request" => {
            let title       = args["title"].as_str().unwrap_or("").to_string();
            let description = args["description"].as_str().unwrap_or("").to_string();
            let priority    = match args["priority"].as_str().unwrap_or("medium") {
                "low"  => FeaturePriority::Low,
                "high" => FeaturePriority::High,
                _      => FeaturePriority::Medium,
            };
            let now = OffsetDateTime::now_utc();
            let id  = format!("{}-{:02}-{:02}-{}", now.year(), now.month() as u8, now.day(), uuid_slug());
            let req = FeatureRequest { id: id.clone(), title, description, status: FeatureStatus::Open, priority, created_at: now.to_string() };
            match store.add_feature(req) {
                Ok(id) => tool_text(format!("Feature request created: {id}")),
                Err(e) => tool_error(e.to_string()),
            }
        }
        "list_features" => {
            match store.list_features() {
                Ok(features) => {
                    let status_filter   = args["status"].as_str();
                    let priority_filter = args["priority"].as_str();
                    let filtered: Vec<_> = features.iter().filter(|f| {
                        status_filter.map_or(true, |s| f.status.to_string() == s)
                            && priority_filter.map_or(true, |p| f.priority.to_string() == p)
                    }).collect();
                    let text = if filtered.is_empty() {
                        "No features found.".to_string()
                    } else {
                        filtered.iter().map(|f| format!("[{}] [{}] {} — {} ({})", f.status, f.priority, f.id, f.title, f.description)).collect::<Vec<_>>().join("\n")
                    };
                    tool_text(text)
                }
                Err(e) => tool_error(e.to_string()),
            }
        }
        "update_feature_status" => {
            let id = args["id"].as_str().unwrap_or("");
            let status = match args["status"].as_str().unwrap_or("") {
                "open"        => FeatureStatus::Open,
                "in_progress" => FeatureStatus::InProgress,
                "done"        => FeatureStatus::Done,
                "declined"    => FeatureStatus::Declined,
                other         => return tool_error(format!("unknown status: {other}")),
            };
            match store.update_feature_status(id, status) {
                Ok(_)  => tool_text(format!("Updated {id}")),
                Err(e) => tool_error(e.to_string()),
            }
        }
        "add_changelog_entry" => {
            let entry = args["entry"].as_str().unwrap_or("");
            match store.append_changelog(entry) {
                Ok(_)  => tool_text("Changelog updated."),
                Err(e) => tool_error(e.to_string()),
            }
        }
        "get_changelog" => {
            match store.get_changelog() {
                Ok(text) => tool_text(text),
                Err(e)   => tool_error(e.to_string()),
            }
        }
        _ => tool_error(format!("unknown tool: {name}")),
    }
}

fn tool_text(text: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": text.into() }] })
}

fn tool_error(msg: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": msg.into() }], "isError": true })
}

fn uuid_slug() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
    format!("{n:08x}")
}
