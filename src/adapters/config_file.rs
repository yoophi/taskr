//! 파일 기반 [`ConfigStore`] 어댑터. XDG 규칙으로 설정 경로를 해석하고
//! JSON으로 읽고 쓴다. macOS에서도 `~/Library/...` 대신 `~/.config/taskr/`를 쓰도록
//! `etcetera`의 Xdg 전략을 사용한다.

use std::path::{Path, PathBuf};

use etcetera::base_strategy::{BaseStrategy, Xdg};

use crate::config::Config;
use crate::error::{Error, Result};
use crate::ports::ConfigStore;

/// 설정 파일 경로를 해석한다(우선순위: `$TASKR_CONFIG` → XDG 기본 경로).
///
/// `--config` 플래그는 우선순위가 가장 높으며, 그 경우 [`FileConfigStore::new`]에
/// 명시 경로를 넘겨 이 함수를 건너뛴다.
pub fn default_config_path() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("TASKR_CONFIG")
        && !p.is_empty()
    {
        return Ok(PathBuf::from(p));
    }
    let xdg = Xdg::new().map_err(|e| Error::Config(format!("홈 디렉터리 해석 실패: {e}")))?;
    Ok(xdg.config_dir().join("taskr").join("config.json"))
}

pub struct FileConfigStore {
    path: PathBuf,
}

impl FileConfigStore {
    /// `explicit`이 있으면 그 경로를, 없으면 [`default_config_path`]를 사용한다.
    pub fn new(explicit: Option<PathBuf>) -> Result<Self> {
        let path = match explicit {
            Some(p) => p,
            None => default_config_path()?,
        };
        Ok(Self { path })
    }
}

impl ConfigStore for FileConfigStore {
    fn path(&self) -> &Path {
        &self.path
    }

    fn load(&self) -> Result<Config> {
        match std::fs::read_to_string(&self.path) {
            Ok(s) => Ok(serde_json::from_str(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // 첫 실행 — 기본 설정을 만들어 저장하고 반환.
                let cfg = Config::default();
                self.save(&cfg)?;
                Ok(cfg)
            }
            Err(e) => Err(e.into()),
        }
    }

    fn save(&self, config: &Config) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(config)?;
        // 원자적 쓰기: 같은 디렉터리에 temp 파일을 쓰고 rename 한다.
        let mut tmp = self.path.clone();
        let name = self
            .path
            .file_name()
            .map(|n| format!("{}.tmp", n.to_string_lossy()))
            .unwrap_or_else(|| "config.json.tmp".to_string());
        tmp.set_file_name(name);
        std::fs::write(&tmp, json.as_bytes())?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Backend;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn scratch_path() -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir()
            .join(format!("taskr-test-{}-{n}", std::process::id()))
            .join("config.json")
    }

    #[test]
    fn load_missing_creates_default_file() {
        let path = scratch_path();
        let store = FileConfigStore::new(Some(path.clone())).unwrap();
        let cfg = store.load().unwrap();
        assert_eq!(cfg.backend, Backend::Beads);
        assert!(path.exists(), "기본 설정 파일이 생성되어야 한다");
        // 정리
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let path = scratch_path();
        let store = FileConfigStore::new(Some(path.clone())).unwrap();
        let cfg = Config { backend: Backend::Memory, ..Default::default() };
        store.save(&cfg).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.backend, Backend::Memory);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
