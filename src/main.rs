//! taskr — 외부 작업관리 인프라(beads, backlog.md) 위에서 동작하는 TUI 클라이언트.
//!
//! 현재는 스캐폴드 단계: 터미널을 안전하게 초기화/복구하고 빈 화면을 그린 뒤
//! `q` 입력 시 종료한다. 이후 단계에서 도메인/포트/어댑터/UI가 채워진다.

use color_eyre::Result;
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::widgets::{Block, Paragraph};

fn main() -> Result<()> {
    // 패닉 시에도 보기 좋은 리포트를 출력한다.
    color_eyre::install()?;

    // 대체 화면 진입 + raw 모드. 종료 시 반드시 restore 한다.
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

/// 메인 렌더/이벤트 루프.
fn run(mut terminal: DefaultTerminal) -> Result<()> {
    loop {
        terminal.draw(draw)?;

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
fn draw(frame: &mut Frame) {
    let block = Block::bordered().title(" taskr ");
    let body = Paragraph::new("taskr — TUI todo client\n\n(스캐폴드 단계)\n\nq: 종료").block(block);
    frame.render_widget(body, frame.area());
}
