//! 아웃바운드/인바운드 어댑터. 포트 트레잇을 구현해 코어를 외부 세계에 연결한다.
//!
//! - [`memory`]: 인메모리 [`crate::ports::TaskRepository`]. 테스트 및 백엔드 없는 데모용.
//! - (추후) `beads`: `bd` CLI 어댑터.
//! - (추후) `backlog`: backlog.md 어댑터.
//! - (추후) `tui`: ratatui 인바운드 어댑터.

pub mod beads;
pub mod config_file;
pub mod memory;
