//! 렌더링. App 상태를 ratatui 위젯으로 그린다(부수효과 없음).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, List, ListItem, Paragraph, Wrap};

use crate::adapters::tui::app::App;
use crate::domain::model::{Priority, Status, Task};

/// 상태별 표시 색.
fn status_color(s: Status) -> Color {
    match s {
        Status::Open => Color::White,
        Status::InProgress => Color::Cyan,
        Status::Blocked => Color::Red,
        Status::Deferred => Color::DarkGray,
        Status::Done => Color::Green,
    }
}

/// 우선순위별 표시 색(P0 가장 강조).
fn priority_color(p: Priority) -> Color {
    match p {
        Priority::P0 => Color::Red,
        Priority::P1 => Color::Yellow,
        Priority::P2 => Color::White,
        Priority::P3 => Color::Blue,
        Priority::P4 => Color::DarkGray,
    }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let [body, statusbar] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());
    let [list_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(body);

    draw_list(frame, app, list_area);
    draw_detail(frame, app.selected(), detail_area);
    draw_statusbar(frame, app, statusbar);
}

fn draw_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .map(|t| {
            let line = Line::from(vec![
                Span::styled(t.status.symbol(), Style::new().fg(status_color(t.status))),
                Span::raw(" "),
                Span::styled(
                    format!("{:<2}", t.priority.label()),
                    Style::new().fg(priority_color(t.priority)),
                ),
                Span::raw(" "),
                Span::raw(t.title.clone()),
            ]);
            ListItem::new(line)
        })
        .collect();

    let title = format!(" 작업 ({}) ", app.tasks.len());
    let list = List::new(items)
        .block(Block::bordered().title(title))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_detail(frame: &mut Frame, task: Option<&Task>, area: Rect) {
    let block = Block::bordered().title(" 상세 ");
    let text: Text = match task {
        Some(t) => {
            let mut lines = vec![
                kv("ID", &t.id),
                kv("제목", &t.title),
                Line::from(vec![
                    Span::styled("상태  ", Style::new().add_modifier(Modifier::BOLD)),
                    Span::styled(t.status.label(), Style::new().fg(status_color(t.status))),
                ]),
                Line::from(vec![
                    Span::styled("우선  ", Style::new().add_modifier(Modifier::BOLD)),
                    Span::styled(t.priority.label(), Style::new().fg(priority_color(t.priority))),
                ]),
            ];
            if let Some(a) = &t.assignee {
                lines.push(kv("담당", a));
            }
            if !t.labels.is_empty() {
                lines.push(kv("라벨", &t.labels.join(", ")));
            }
            if let Some(p) = &t.parent {
                lines.push(kv("상위", p));
            }
            lines.push(Line::raw(""));
            lines.push(Line::styled("설명", Style::new().add_modifier(Modifier::BOLD)));
            lines.push(Line::raw(t.description.clone().unwrap_or_else(|| "(없음)".into())));
            Text::from(lines)
        }
        None => Text::from("선택된 작업이 없습니다."),
    };
    frame.render_widget(Paragraph::new(text).block(block).wrap(Wrap { trim: false }), area);
}

/// "키  값" 형태의 한 줄. 입력을 소유 문자열로 복사하므로 반환 수명은 `'static`이다.
fn kv(key: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:<4}"), Style::new().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::raw(value.to_string()),
    ])
}

fn draw_statusbar(frame: &mut Frame, app: &App, area: Rect) {
    let left = format!(" [{}] {} ", app.backend_name(), app.status);
    let hints = "j/k 이동  r 새로고침  q 종료 ";
    let [l, r] =
        Layout::horizontal([Constraint::Min(1), Constraint::Length(hints.len() as u16)]).areas(area);
    frame.render_widget(Paragraph::new(left).style(Style::new().bg(Color::DarkGray).fg(Color::White)), l);
    frame.render_widget(
        Paragraph::new(hints).style(Style::new().bg(Color::DarkGray).fg(Color::Gray)),
        r,
    );
}
