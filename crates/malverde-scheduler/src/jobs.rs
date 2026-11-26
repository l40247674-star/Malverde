use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use malverde_core::{MalverdeError, Result};

use crate::cron::Cron;

/// Un trabajo programado: un prompt que se ejecuta en un cron.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub cron: String,
    pub prompt: String,
    pub enabled: bool,
    #[serde(default)]
    pub last_run: Option<DateTime<Utc>>,
    #[serde(default = "default_platform")]
    pub platform: String,
}

fn default_platform() -> String {
    "cli".to_string()
}

impl CronJob {
    pub fn next_run(&self) -> Result<Option<DateTime<Utc>>> {
        if !self.enabled {
            return Ok(None);
        }
        let cron = Cron::parse(&self.cron)?;
        Ok(cron.next_after(self.last_run.unwrap_or_else(Utc::now)))
    }
}

/// Almacén de trabajos cron persistido en JSON.
#[derive(Debug, Default)]
pub struct CronStore {
    path: std::path::PathBuf,
}

impl CronStore {
    pub fn open(path: impl AsRef<Path>) -> Self {
        Self { path: path.as_ref().to_path_buf() }
    }

    pub fn list(&self) -> Result<Vec<CronJob>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let raw = std::fs::read_to_string(&self.path).map_err(MalverdeError::io)?;
        serde_json::from_str(&raw).map_err(|e| MalverdeError::serialization(e.to_string()))
    }

    pub fn save(&self, jobs: &[CronJob]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(MalverdeError::io)?;
        }
        let json = serde_json::to_string_pretty(jobs).map_err(|e| MalverdeError::serialization(e.to_string()))?;
        std::fs::write(&self.path, json).map_err(MalverdeError::io)
    }

    pub fn add(&self, name: &str, cron: &str, prompt: &str, platform: &str) -> Result<CronJob> {
        let mut jobs = self.list()?;
        Cron::parse(cron)?;
        let job = CronJob {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            name: name.to_string(),
            cron: cron.to_string(),
            prompt: prompt.to_string(),
            enabled: true,
            last_run: None,
            platform: platform.to_string(),
        };
        jobs.push(job.clone());
        self.save(&jobs)?;
        Ok(job)
    }

    pub fn remove(&self, id: &str) -> Result<bool> {
        let mut jobs = self.list()?;
        let before = jobs.len();
        jobs.retain(|j| j.id != id);
        self.save(&jobs)?;
        Ok(jobs.len() != before)
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<bool> {
        let mut jobs = self.list()?;
        let mut changed = false;
        for j in &mut jobs {
            if j.id == id {
                j.enabled = enabled;
                changed = true;
            }
        }
        if changed {
            self.save(&jobs)?;
        }
        Ok(changed)
    }

    pub fn mark_run(&self, id: &str, at: DateTime<Utc>) -> Result<()> {
        let mut jobs = self.list()?;
        for j in &mut jobs {
            if j.id == id {
                j.last_run = Some(at);
            }
        }
        self.save(&jobs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_store() -> (std::path::PathBuf, CronStore) {
        let dir = std::env::temp_dir().join(format!("malverde-cron-{}-{}", std::process::id(), format!("{:?}", std::thread::current().id()).replace(|c: char| !c.is_ascii_alphanumeric(), "")));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cron.json");
        (path.clone(), CronStore::open(path))
    }

    #[test]
    fn add_list_remove() {
        let (dir, store) = tmp_store();
        let job = store.add("saludo", "0 9 * * *", "dime buenos días", "cli").unwrap();
        assert_eq!(store.list().unwrap().len(), 1);
        assert!(job.next_run().unwrap().is_some());
        assert!(store.set_enabled(&job.id, false).unwrap());
        let disabled = &store.list().unwrap()[0];
        assert!(!disabled.enabled);
        assert!(store.remove(&job.id).unwrap());
        assert!(store.list().unwrap().is_empty());
        std::fs::remove_dir_all(dir.parent().unwrap()).ok();
    }

    #[test]
    fn rejects_bad_cron() {
        let (dir, store) = tmp_store();
        assert!(store.add("malo", "99 * * * *", "x", "cli").is_err());
        std::fs::remove_dir_all(dir.parent().unwrap()).ok();
    }
}
