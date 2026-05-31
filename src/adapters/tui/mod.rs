//! ratatui 인바운드 어댑터. 터미널 이벤트를 받아 [`App`] 상태를 갱신하고 [`ui`]로 그린다.

mod app;
mod ui;

pub use app::App;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::config::View;
use crate::error::Result;
use crate::ports::TaskApi;

/// TUI를 실행한다. 터미널 init/restore는 호출측(main)이 책임진다.
pub fn run(
    terminal: &mut DefaultTerminal,
    api: Box<dyn TaskApi>,
    config_path: String,
    view: View,
) -> Result<()> {
    let mut app = App::new(api, config_path, view);
    app.refresh();

    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;
        handle_event(&mut app)?;
    }
    Ok(())
}

/// 한 번의 입력 이벤트를 처리한다.
fn handle_event(app: &mut App) -> Result<()> {
    // 윈도우에서는 press/release 두 번 들어오므로 press만 처리한다.
    if let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
    {
        match key.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => app.next(),
            KeyCode::Char('k') | KeyCode::Up => app.prev(),
            KeyCode::Char('g') | KeyCode::Home => app.first(),
            KeyCode::Char('G') | KeyCode::End => app.last(),
            KeyCode::Char('r') => app.refresh(),
            _ => {}
        }
    }
    Ok(())
}
