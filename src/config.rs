//! 앱 설정 모델. JSON으로 직렬화되며 `~/.config/taskr/config.json`에 저장된다.
//! 모든 필드는 `#[serde(default)]`로 부분 설정 + 기본값 병합을 지원한다(일부 키만 있어도 동작).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 사용할 백엔드. `--backend`로 오버라이드 가능(clap `ValueEnum`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    /// beads(`bd` CLI).
    #[default]
    Beads,
    /// backlog.md(`backlog` CLI). (추후 지원)
    Backlog,
    /// 인메모리(백엔드 없이 데모/테스트).
    Memory,
}

impl Backend {
    pub fn as_str(self) -> &'static str {
        match self {
            Backend::Beads => "beads",
            Backend::Backlog => "backlog",
            Backend::Memory => "memory",
        }
    }
}

/// 시작 시 표시할 기본 뷰.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    #[default]
    List,
    Board,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BeadsConfig {
    /// beads DB 디렉터리. `None`이면 현재 작업 디렉터리/`BEADS_DIR`를 따른다.
    pub dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BacklogConfig {
    /// backlog.md 프로젝트 경로. `None`이면 현재 작업 디렉터리.
    pub project_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub default_view: View,
}

/// 최상위 설정.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub backend: Backend,
    pub beads: BeadsConfig,
    pub backlog: BacklogConfig,
    pub ui: UiConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_json_merges_defaults() {
        // backend만 지정 — 나머지는 기본값으로 채워져야 한다.
        let cfg: Config = serde_json::from_str(r#"{ "backend": "backlog" }"#).unwrap();
        assert_eq!(cfg.backend, Backend::Backlog);
        assert_eq!(cfg.ui.default_view, View::List);
        assert!(cfg.beads.dir.is_none());
    }

    #[test]
    fn default_backend_is_beads() {
        assert_eq!(Config::default().backend, Backend::Beads);
    }
}
