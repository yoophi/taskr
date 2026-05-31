//! 렌더링. App 상태를 ratatui 위젯으로 그린다(부수효과 없음).

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Clear, List, ListItem, Paragraph, Wrap};

use crate::adapters::tui::app::{App, FormField, Mode};
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

    // 모달은 본문 위에 덧그린다.
    match &app.mode {
        Mode::Normal => {}
        Mode::Form(_) => draw_form(frame, app),
        Mode::ConfirmDelete { title, .. } => draw_confirm(frame, title),
    }
}

/// 화면 중앙에 가로/세로 비율로 영역을 잡는다.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let [v] = Layout::vertical([Constraint::Length(height)]).flex(Flex::Center).areas(area);
    let [h] = Layout::horizontal([Constraint::Length(width)]).flex(Flex::Center).areas(v);
    h
}

fn draw_form(frame: &mut Frame, app: &App) {
    let Mode::Form(form) = &app.mode else { return };
    let area = centered_rect(60, 11, frame.area());
    frame.render_widget(Clear, area);

    let title = if form.is_edit() { " 작업 수정 " } else { " 새 작업 " };
    let field_line = |label: &str, value: &str, focused: bool| -> Line<'static> {
        let marker = if focused { "▶ " } else { "  " };
        let style = if focused {
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::new()
        };
        Line::from(vec![
            Span::styled(format!("{marker}{label:<5} "), style),
            Span::raw(value.to_string()),
            if focused { Span::styled("▏", Style::new().fg(Color::Yellow)) } else { Span::raw("") },
        ])
    };

    let lines = vec![
        field_line("제목", &form.title, form.field == FormField::Title),
        field_line("설명", &form.description, form.field == FormField::Description),
        field_line(
            "우선",
            &format!("◀ {} ▶", form.priority.label()),
            form.field == FormField::Priority,
        ),
        Line::raw(""),
        Line::styled(
            "Tab/↑↓ 필드  ←→ 우선순위  Enter 저장  Esc 취소",
            Style::new().fg(Color::DarkGray),
        ),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(Block::bordered().title(title)).wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_confirm(frame: &mut Frame, title: &str) {
    let area = centered_rect(50, 6, frame.area());
    frame.render_widget(Clear, area);
    let lines = vec![
        Line::raw(""),
        Line::from(vec![Span::raw("  삭제할까요? "), Span::styled(title.to_string(), Style::new().add_modifier(Modifier::BOLD))]),
        Line::raw(""),
        Line::styled("  y/Enter 삭제    n/Esc 취소", Style::new().fg(Color::DarkGray)),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::bordered().title(" 삭제 확인 ").border_style(Style::new().fg(Color::Red))),
        area,
    );
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
    let hints = "n 새작업  e 수정  d 삭제  space 완료  r 새로고침  q 종료 ";
    let [l, r] = Layout::horizontal([Constraint::Min(1), Constraint::Length(display_width(hints))])
        .areas(area);
    frame.render_widget(Paragraph::new(left).style(Style::new().bg(Color::DarkGray).fg(Color::White)), l);
    frame.render_widget(
        Paragraph::new(hints).style(Style::new().bg(Color::DarkGray).fg(Color::Gray)),
        r,
    );
}

/// 터미널 표시 폭 근사(ASCII는 1칸, 그 외 한글 등은 2칸).
fn display_width(s: &str) -> u16 {
    s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum::<usize>() as u16
}
