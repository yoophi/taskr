//! ratatui 인바운드 어댑터. 터미널 이벤트를 받아 [`App`] 상태를 갱신하고 [`ui`]로 그린다.

mod app;
mod ui;

pub use app::App;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::adapters::tui::app::Mode;
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

/// 한 번의 입력 이벤트를 모드별로 처리한다.
fn handle_event(app: &mut App) -> Result<()> {
    // 윈도우에서는 press/release 두 번 들어오므로 press만 처리한다.
    let Event::Key(key) = event::read()? else {
        return Ok(());
    };
    if key.kind != KeyEventKind::Press {
        return Ok(());
    }

    match &app.mode {
        Mode::Normal => handle_normal(app, key.code),
        Mode::Form(_) => handle_form(app, key.code),
        Mode::ConfirmDelete { .. } => handle_confirm(app, key.code),
        Mode::Search(_) => handle_search(app, key.code),
        Mode::Help => app.cancel_modal(), // 아무 키나 닫기
    }
    Ok(())
}

fn handle_normal(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
        KeyCode::Char('h') | KeyCode::Left => app.move_left(),
        KeyCode::Char('l') | KeyCode::Right => app.move_right(),
        KeyCode::Char('g') | KeyCode::Home => app.first(),
        KeyCode::Char('G') | KeyCode::End => app.last(),
        KeyCode::Char('r') => app.refresh(),
        KeyCode::Char('n') => app.start_create(),
        KeyCode::Char('e') => app.start_edit(),
        KeyCode::Char('d') => app.start_delete(),
        KeyCode::Char(' ') => app.toggle_status(),
        KeyCode::Tab => app.toggle_view(),
        KeyCode::Char('/') => app.start_search(),
        KeyCode::Char('f') => app.cycle_status_filter(),
        KeyCode::Char('?') => app.show_help(),
        KeyCode::Esc => app.clear_filter(),
        _ => {}
    }
}

fn handle_search(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.cancel_modal(),
        KeyCode::Enter => app.submit_search(),
        KeyCode::Backspace => app.search_backspace(),
        KeyCode::Char(c) => app.search_input(c),
        _ => {}
    }
}

fn handle_form(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.cancel_modal(),
        KeyCode::Enter => app.submit_form(),
        KeyCode::Tab | KeyCode::Down => app.form_next_field(),
        KeyCode::BackTab | KeyCode::Up => app.form_prev_field(),
        // 우선순위 필드에서는 좌우로 값을 바꾼다.
        KeyCode::Left if app.form_on_priority() => app.form_cycle_priority(-1),
        KeyCode::Right if app.form_on_priority() => app.form_cycle_priority(1),
        KeyCode::Backspace => app.form_backspace(),
        KeyCode::Char(c) => app.form_input(c),
        _ => {}
    }
}

fn handle_confirm(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => app.confirm_delete(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_modal(),
        _ => {}
    }
}
