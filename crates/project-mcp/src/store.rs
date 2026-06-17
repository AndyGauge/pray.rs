use std::{
    fs,
    path::PathBuf,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io at {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

fn io_err(path: impl Into<PathBuf>, source: std::io::Error) -> StoreError {
    StoreError::Io { path: path.into(), source }
}

pub struct ProjectStore {
    root: PathBuf,
}

impl ProjectStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self { root }
    }

    fn features_dir(&self) -> PathBuf { self.root.join("features") }
    fn changelog_path(&self) -> PathBuf { self.root.join("changelog.md") }

    fn ensure_dirs(&self) -> Result<(), StoreError> {
        fs::create_dir_all(self.features_dir())
            .map_err(|e| io_err(self.features_dir(), e))
    }

    // ── Feature requests ──────────────────────────────────────────────────

    pub fn add_feature(&self, req: FeatureRequest) -> Result<String, StoreError> {
        self.ensure_dirs()?;
        let id = req.id.clone();
        let path = self.features_dir().join(format!("{id}.json"));
        let json = serde_json::to_string_pretty(&req)?;
        fs::write(&path, json).map_err(|e| io_err(&path, e))?;
        Ok(id)
    }

    pub fn list_features(&self) -> Result<Vec<FeatureRequest>, StoreError> {
        self.ensure_dirs()?;
        let mut features = vec![];
        for entry in fs::read_dir(self.features_dir())
            .map_err(|e| io_err(self.features_dir(), e))?
        {
            let entry = entry.map_err(|e| io_err(self.features_dir(), e))?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                let raw = fs::read_to_string(entry.path())
                    .map_err(|e| io_err(entry.path(), e))?;
                features.push(serde_json::from_str(&raw)?);
            }
        }
        features.sort_by(|a: &FeatureRequest, b: &FeatureRequest| b.created_at.cmp(&a.created_at));
        Ok(features)
    }

    pub fn update_feature_status(&self, id: &str, status: FeatureStatus) -> Result<(), StoreError> {
        let path = self.features_dir().join(format!("{id}.json"));
        let raw = fs::read_to_string(&path).map_err(|e| io_err(&path, e))?;
        let mut req: FeatureRequest = serde_json::from_str(&raw)?;
        req.status = status;
        let json = serde_json::to_string_pretty(&req)?;
        fs::write(&path, json).map_err(|e| io_err(&path, e))?;
        Ok(())
    }

    // ── Changelog ─────────────────────────────────────────────────────────

    pub fn append_changelog(&self, entry: &str) -> Result<(), StoreError> {
        let path = self.changelog_path();
        let existing = if path.exists() {
            fs::read_to_string(&path).map_err(|e| io_err(&path, e))?
        } else {
            "# Thanksgivings Changelog\n\n".to_string()
        };

        let now = OffsetDateTime::now_utc();
        let date = format!("{}-{:02}-{:02}", now.year(), now.month() as u8, now.day());
        let new_entry = format!("\n## {date}\n\n{entry}\n");

        fs::write(&path, existing + &new_entry)
            .map_err(|e| io_err(&path, e))?;
        Ok(())
    }

    pub fn get_changelog(&self) -> Result<String, StoreError> {
        let path = self.changelog_path();
        if !path.exists() {
            return Ok("# Thanksgivings Changelog\n\n_No entries yet._".to_string());
        }
        fs::read_to_string(&path).map_err(|e| io_err(path, e))
    }
}

// ── Domain types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureRequest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: FeatureStatus,
    pub priority: FeaturePriority,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeatureStatus {
    Open,
    InProgress,
    Done,
    Declined,
}

impl std::fmt::Display for FeatureStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self { Self::Open => "open", Self::InProgress => "in_progress", Self::Done => "done", Self::Declined => "declined" };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeaturePriority { Low, Medium, High }

impl std::fmt::Display for FeaturePriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self { Self::Low => "low", Self::Medium => "medium", Self::High => "high" };
        write!(f, "{s}")
    }
}
