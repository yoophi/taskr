//! 헥사고날 포트(트레잇) 정의.
//!
//! - 인바운드 포트 [`TaskApi`]: 인바운드 어댑터(TUI)가 호출하는 유스케이스 표면.
//! - 아웃바운드 포트 [`TaskRepository`]: 코어가 외부 인프라(beads, backlog.md 등)에 요구하는 능력.
//!
//! 코어는 이 트레잇에만 의존하고 구체 어댑터는 모른다(의존성 역전).

use std::path::Path;

use crate::config::Config;
use crate::domain::model::{Filter, NewTask, Status, Task, TaskPatch};
use crate::error::Result;

/// 아웃바운드 포트 — 외부 작업관리 인프라 어댑터가 구현한다.
pub trait TaskRepository {
    /// 백엔드 식별용 이름(상태바 표시 등).
    fn name(&self) -> &str;

    fn list(&self, filter: &Filter) -> Result<Vec<Task>>;
    fn get(&self, id: &str) -> Result<Task>;
    fn create(&self, task: &NewTask) -> Result<String>;
    fn update(&self, id: &str, patch: &TaskPatch) -> Result<()>;
    fn set_status(&self, id: &str, status: Status) -> Result<()>;
    fn delete(&self, id: &str) -> Result<()>;
}

/// 아웃바운드 포트 — 설정 영속화. [`crate::adapters::config_file::FileConfigStore`]가 구현한다.
pub trait ConfigStore {
    /// 설정 파일 경로.
    fn path(&self) -> &Path;
    /// 설정을 읽는다. 파일이 없으면 기본값을 생성·저장하고 반환한다.
    fn load(&self) -> Result<Config>;
    /// 설정을 저장한다(원자적 쓰기).
    fn save(&self, config: &Config) -> Result<()>;
}

/// 인바운드 포트 — TUI가 호출하는 유스케이스. [`crate::domain::service::TaskService`]가 구현한다.
pub trait TaskApi {
    fn backend_name(&self) -> &str;

    fn list(&self, filter: &Filter) -> Result<Vec<Task>>;
    fn get(&self, id: &str) -> Result<Task>;
    fn create(&self, task: &NewTask) -> Result<String>;
    fn update(&self, id: &str, patch: &TaskPatch) -> Result<()>;
    fn set_status(&self, id: &str, status: Status) -> Result<()>;
    fn delete(&self, id: &str) -> Result<()>;
}
