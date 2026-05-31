//! taskr — 외부 작업관리 인프라(beads, backlog.md) 위에서 동작하는 TUI 클라이언트.
//!
//! `main`은 Composition Root다: 설정을 읽어 백엔드 어댑터를 선택·주입하고 TUI를 띄운다.
//! (현재는 설정 로드까지 배선되어 있고, 백엔드 어댑터/실 UI는 이후 단계에서 채워진다.)

// 코어/어댑터를 점진적으로 조립하는 동안에는 아직 UI에서 호출되지 않는 코드가 있다.
// 전체 배선이 끝나는 마감 단계에서 제거한다.
#![allow(dead_code)]

mod adapters;
mod config;
mod domain;
mod error;
mod ports;

use std::path::PathBuf;

use clap::Parser;
use color_eyre::Result;
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::widgets::{Block, Paragraph};

use crate::adapters::config_file::FileConfigStore;
use crate::config::{Backend, Config};
use crate::ports::ConfigStore;

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

    // 대체 화면 진입 + raw 모드. 종료 시 반드시 restore 한다.
    let terminal = ratatui::init();
    let result = run(terminal, &config, &config_path);
    ratatui::restore();
    result
}

/// 메인 렌더/이벤트 루프.
fn run(mut terminal: DefaultTerminal, config: &Config, config_path: &str) -> Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, config, config_path))?;

        // 윈도우에서는 press/release 두 번 들어오므로 press만 처리한다.
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && key.code == KeyCode::Char('q')
        {
            break;
        }
    }
    Ok(())
}

/// 한 프레임을 그린다.
fn draw(frame: &mut Frame, config: &Config, config_path: &str) {
    let block = Block::bordered().title(" taskr ");
    let text = format!(
        "taskr — TUI todo client\n\nbackend: {}\nconfig:  {}\n\n(설정 계층 완료 — 백엔드/UI는 다음 단계)\n\nq: 종료",
        config.backend.as_str(),
        config_path,
    );
    frame.render_widget(Paragraph::new(text).block(block), frame.area());
}
