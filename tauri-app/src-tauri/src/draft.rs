use crate::storage::atomic_write_string;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

const DRAFT_FILE_NAME: &str = "draft.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DraftSnapshot {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub target_id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

pub fn load_draft<R: Runtime>(app: &AppHandle<R>) -> Result<DraftSnapshot, String> {
    let path = draft_file_path(app)?;
    if !path.exists() {
        return Ok(DraftSnapshot::default());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read draft file {}: {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse draft file {}: {error}", path.display()))
}

pub fn save_draft<R: Runtime>(app: &AppHandle<R>, draft: &DraftSnapshot) -> Result<(), String> {
    let path = draft_file_path(app)?;
    let raw = serde_json::to_string_pretty(draft)
        .map_err(|error| format!("failed to serialize draft: {error}"))?;
    atomic_write_string(&path, &raw)
}

fn draft_file_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|path| path.join(DRAFT_FILE_NAME))
        .map_err(|error| format!("failed to resolve draft directory: {error}"))
}

#[cfg(test)]
mod tests {
    use super::DraftSnapshot;

    #[test]
    fn draft_snapshot_round_trips() {
        let draft = DraftSnapshot {
            text: "line one\nline two".into(),
            target_id: "daily".into(),
            created_at: "2026-09-03 10:00".into(),
            updated_at: "2026-09-03T10:01:00+08:00".into(),
        };

        let raw = serde_json::to_string(&draft).unwrap();
        assert_eq!(serde_json::from_str::<DraftSnapshot>(&raw).unwrap(), draft);
    }
}
