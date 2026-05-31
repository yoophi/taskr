//! taskr — 외부 작업관리 인프라(beads, backlog.md) 위에서 동작하는 TUI 클라이언트.
//!
//! `main`은 Composition Root다: 설정을 읽어 백엔드 어댑터를 선택·주입하고 TUI를 띄운다.

mod adapters;
mod config;
mod domain;
mod error;
mod ports;

use std::path::PathBuf;

use clap::Parser;
use color_eyre::Result;

use crate::adapters::backlog::BacklogMdRepository;
use crate::adapters::beads::BeadsRepository;
use crate::adapters::config_file::FileConfigStore;
use crate::adapters::memory::MemoryRepository;
use crate::config::{Backend, Config};
use crate::domain::service::TaskService;
use crate::ports::{ConfigStore, TaskApi, TaskRepository};

/// 명령행 인자. 설정 파일 값을 덮어쓴다.
#[derive(Parser, Debug)]
#[command(name = "taskr", version, about = "외부 작업관리 인프라 위의 TUI 할 일 클라이언트")]
struct Cli {
    /// 설정 파일 경로 (기본: $TASKR_CONFIG 또는 ~/.config/taskr/config.json)
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// 사용할 백엔드 (설정 파일의 backend 값을 덮어씀)
    #[arg(long, value_enum)]
    backend: Option<Backend>,
}

fn main() -> Result<()> {
    // 패닉 시에도 보기 좋은 리포트를 출력한다.
    color_eyre::install()?;

    let cli = Cli::parse();

    // 설정 로드(없으면 기본값 생성) + CLI 오버라이드 적용.
    let store = FileConfigStore::new(cli.config)?;
    let mut config = store.load()?;
    if let Some(backend) = cli.backend {
        config.backend = backend;
    }
    let config_path = store.path().display().to_string();

    // 설정에 따라 아웃바운드 어댑터를 선택해 서비스에 주입한다(의존성 역전).
    let repo = build_repository(&config);
    let api: Box<dyn TaskApi> = Box::new(TaskService::new(repo));

    // 대체 화면 진입 + raw 모드. 종료 시 반드시 restore 한다.
    let mut terminal = ratatui::init();
    let result = adapters::tui::run(&mut terminal, api, config_path, config.ui.default_view);
    ratatui::restore();
    result.map_err(Into::into)
}

/// 설정의 backend 값에 맞는 [`TaskRepository`]를 만든다.
fn build_repository(config: &Config) -> Box<dyn TaskRepository> {
    match config.backend {
        Backend::Beads => Box::new(BeadsRepository::new(&config.beads)),
        Backend::Backlog => Box::new(BacklogMdRepository::new(&config.backlog)),
        Backend::Memory => Box::new(MemoryRepository::new()),
    }
}
