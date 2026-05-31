//! TUI 애플리케이션 상태. 인바운드 포트 [`TaskApi`]를 통해서만 백엔드와 대화한다.

use ratatui::widgets::ListState;

use crate::config::View;
use crate::domain::model::{Filter, Task};
use crate::ports::TaskApi;

pub struct App {
    api: Box<dyn TaskApi>,
    /// 상태바에 표시할 설정 파일 경로.
    pub config_path: String,
    /// 현재 표시 중인 작업 목록.
    pub tasks: Vec<Task>,
    /// 리스트 선택 상태.
    pub list_state: ListState,
    /// 현재 뷰(리스트/보드). 보드는 이후 단계에서 사용.
    pub view: View,
    /// 하단 상태바 메시지(개수 또는 에러).
    pub status: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(api: Box<dyn TaskApi>, config_path: String, view: View) -> Self {
        Self {
            api,
            config_path,
            tasks: Vec::new(),
            list_state: ListState::default(),
            view,
            status: String::new(),
            should_quit: false,
        }
    }

    pub fn backend_name(&self) -> &str {
        self.api.backend_name()
    }

    /// 백엔드에서 목록을 다시 불러온다. 실패하면 상태바에 에러를 표시한다.
    pub fn refresh(&mut self) {
        match self.api.list(&Filter::default()) {
            Ok(tasks) => {
                self.tasks = tasks;
                self.clamp_selection();
                self.status = format!("{}개 작업", self.tasks.len());
            }
            Err(e) => {
                self.status = format!("오류: {e}");
            }
        }
    }

    /// 목록 길이에 맞춰 선택 인덱스를 보정한다.
    fn clamp_selection(&mut self) {
        if self.tasks.is_empty() {
            self.list_state.select(None);
        } else {
            let i = self.list_state.selected().unwrap_or(0).min(self.tasks.len() - 1);
            self.list_state.select(Some(i));
        }
    }

    pub fn selected(&self) -> Option<&Task> {
        self.list_state.selected().and_then(|i| self.tasks.get(i))
    }

    pub fn next(&mut self) {
        if self.tasks.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1).min(self.tasks.len() - 1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn prev(&mut self) {
        if self.tasks.is_empty() {
            return;
        }
        let i = self.list_state.selected().map(|i| i.saturating_sub(1)).unwrap_or(0);
        self.list_state.select(Some(i));
    }

    pub fn first(&mut self) {
        if !self.tasks.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn last(&mut self) {
        if !self.tasks.is_empty() {
            self.list_state.select(Some(self.tasks.len() - 1));
        }
    }
}
