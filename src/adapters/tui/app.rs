//! TUI 애플리케이션 상태. 인바운드 포트 [`TaskApi`]를 통해서만 백엔드와 대화한다.

use ratatui::widgets::ListState;

use crate::config::View;
use crate::domain::model::{Filter, NewTask, Priority, Status, Task, TaskPatch};
use crate::ports::TaskApi;

/// 생성/수정 폼의 입력 필드.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Title,
    Description,
    Priority,
}

/// 생성/수정 폼 상태. `editing_id`가 `Some`이면 수정, `None`이면 생성.
#[derive(Debug, Clone)]
pub struct Form {
    pub editing_id: Option<String>,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub field: FormField,
}

impl Form {
    fn for_create() -> Self {
        Self {
            editing_id: None,
            title: String::new(),
            description: String::new(),
            priority: Priority::default(),
            field: FormField::Title,
        }
    }

    fn for_edit(task: &Task) -> Self {
        Self {
            editing_id: Some(task.id.clone()),
            title: task.title.clone(),
            description: task.description.clone().unwrap_or_default(),
            priority: task.priority,
            field: FormField::Title,
        }
    }

    pub fn is_edit(&self) -> bool {
        self.editing_id.is_some()
    }

    fn input_char(&mut self, c: char) {
        match self.field {
            FormField::Title => self.title.push(c),
            FormField::Description => self.description.push(c),
            FormField::Priority => {}
        }
    }

    fn backspace(&mut self) {
        match self.field {
            FormField::Title => {
                self.title.pop();
            }
            FormField::Description => {
                self.description.pop();
            }
            FormField::Priority => {}
        }
    }

    fn next_field(&mut self) {
        self.field = match self.field {
            FormField::Title => FormField::Description,
            FormField::Description => FormField::Priority,
            FormField::Priority => FormField::Title,
        };
    }

    fn prev_field(&mut self) {
        self.field = match self.field {
            FormField::Title => FormField::Priority,
            FormField::Description => FormField::Title,
            FormField::Priority => FormField::Description,
        };
    }

    fn cycle_priority(&mut self, delta: i8) {
        let cur = self.priority.as_num() as i8;
        let next = (cur + delta).clamp(0, 4) as u8;
        self.priority = Priority::from_num(next);
    }
}

/// 현재 UI 모드(모달 상태 머신).
pub enum Mode {
    Normal,
    Form(Form),
    ConfirmDelete { id: String, title: String },
    /// 텍스트 검색 입력 중(현재까지 입력된 질의).
    Search(String),
    /// 도움말 오버레이.
    Help,
}

pub struct App {
    api: Box<dyn TaskApi>,
    pub config_path: String,
    pub tasks: Vec<Task>,
    pub list_state: ListState,
    pub view: View,
    pub mode: Mode,
    /// 현재 적용 중인 목록 필터(상태/우선순위/라벨/텍스트).
    pub filter: Filter,
    /// 하단 상태바 메시지(개수/결과/에러).
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
            mode: Mode::Normal,
            filter: Filter::default(),
            status: String::new(),
            should_quit: false,
        }
    }

    pub fn backend_name(&self) -> &str {
        self.api.backend_name()
    }

    // ── 목록/선택 ────────────────────────────────────────────────

    pub fn refresh(&mut self) {
        match self.api.list(&self.filter) {
            Ok(tasks) => {
                self.tasks = tasks;
                self.clamp_selection();
                self.status = format!("{}개 작업", self.tasks.len());
            }
            Err(e) => self.status = format!("오류: {e}"),
        }
    }

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

    // ── 모달 진입 ────────────────────────────────────────────────

    pub fn start_create(&mut self) {
        self.mode = Mode::Form(Form::for_create());
    }

    pub fn start_edit(&mut self) {
        if let Some(task) = self.selected() {
            self.mode = Mode::Form(Form::for_edit(task));
        }
    }

    pub fn start_delete(&mut self) {
        if let Some(task) = self.selected() {
            self.mode = Mode::ConfirmDelete { id: task.id.clone(), title: task.title.clone() };
        }
    }

    pub fn cancel_modal(&mut self) {
        self.mode = Mode::Normal;
    }

    // ── 폼 입력 전달 ─────────────────────────────────────────────

    pub fn form_input(&mut self, c: char) {
        if let Mode::Form(f) = &mut self.mode {
            f.input_char(c);
        }
    }
    pub fn form_backspace(&mut self) {
        if let Mode::Form(f) = &mut self.mode {
            f.backspace();
        }
    }
    pub fn form_next_field(&mut self) {
        if let Mode::Form(f) = &mut self.mode {
            f.next_field();
        }
    }
    pub fn form_prev_field(&mut self) {
        if let Mode::Form(f) = &mut self.mode {
            f.prev_field();
        }
    }
    pub fn form_cycle_priority(&mut self, delta: i8) {
        if let Mode::Form(f) = &mut self.mode {
            f.cycle_priority(delta);
        }
    }
    /// 현재 폼에서 우선순위 필드가 선택돼 있는지(키 처리 분기용).
    pub fn form_on_priority(&self) -> bool {
        matches!(&self.mode, Mode::Form(f) if f.field == FormField::Priority)
    }

    // ── 액션(백엔드 호출) ────────────────────────────────────────

    /// 폼을 제출한다(생성 또는 수정). 성공 시 닫고 새로고침, 실패 시 폼을 유지한 채 에러 표시.
    pub fn submit_form(&mut self) {
        // 폼을 꺼내고 모드를 Normal로 둔다(불변/가변 차용 충돌 방지).
        let Mode::Form(form) = std::mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };

        let result = match &form.editing_id {
            Some(id) => {
                let patch = TaskPatch {
                    title: Some(form.title.clone()),
                    description: Some(form.description.clone()),
                    priority: Some(form.priority),
                    ..Default::default()
                };
                self.api.update(id, &patch).map(|()| id.clone())
            }
            None => {
                let new = NewTask {
                    title: form.title.clone(),
                    description: non_empty(&form.description),
                    priority: form.priority,
                    ..Default::default()
                };
                self.api.create(&new)
            }
        };

        match result {
            Ok(_) => {
                let action = if form.is_edit() { "수정됨" } else { "생성됨" };
                self.refresh();
                self.status = action.into();
            }
            Err(e) => {
                // 폼을 되살려 사용자가 고칠 수 있게 한다.
                self.status = format!("오류: {e}");
                self.mode = Mode::Form(form);
            }
        }
    }

    /// 삭제 확인 후 실제 삭제.
    pub fn confirm_delete(&mut self) {
        let Mode::ConfirmDelete { id, .. } = std::mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };
        match self.api.delete(&id) {
            Ok(()) => {
                self.refresh();
                self.status = "삭제됨".into();
            }
            Err(e) => self.status = format!("오류: {e}"),
        }
    }

    /// 선택 작업의 완료/미완료를 토글한다(Done ↔ Open).
    pub fn toggle_status(&mut self) {
        let Some((id, cur)) = self.selected().map(|t| (t.id.clone(), t.status)) else {
            return;
        };
        let next = if cur == Status::Done { Status::Open } else { Status::Done };
        match self.api.set_status(&id, next) {
            Ok(()) => self.refresh(),
            Err(e) => self.status = format!("오류: {e}"),
        }
    }

    // ── 뷰 전환 / 방향 이동(뷰에 따라 분기) ─────────────────────

    pub fn toggle_view(&mut self) {
        self.view = match self.view {
            View::List => View::Board,
            View::Board => View::List,
        };
    }

    pub fn move_down(&mut self) {
        match self.view {
            View::List => self.next(),
            View::Board => self.board_move(0, 1),
        }
    }
    pub fn move_up(&mut self) {
        match self.view {
            View::List => self.prev(),
            View::Board => self.board_move(0, -1),
        }
    }
    pub fn move_left(&mut self) {
        if matches!(self.view, View::Board) {
            self.board_move(-1, 0);
        }
    }
    pub fn move_right(&mut self) {
        if matches!(self.view, View::Board) {
            self.board_move(1, 0);
        }
    }

    /// 보드 뷰에서 선택을 열(상태) `dcol`, 행 `drow` 만큼 이동한다.
    fn board_move(&mut self, dcol: i32, drow: i32) {
        if self.tasks.is_empty() {
            return;
        }
        let cur = self.list_state.selected().unwrap_or(0);
        let cur_status = self.tasks[cur].status;
        let col = Status::ALL.iter().position(|s| *s == cur_status).unwrap_or(0);
        let col_idx = self.indices_in_status(cur_status);
        let row = col_idx.iter().position(|&i| i == cur).unwrap_or(0);

        if drow != 0 && !col_idx.is_empty() {
            let n = col_idx.len() as i32;
            let r = (row as i32 + drow).clamp(0, n - 1) as usize;
            self.list_state.select(Some(col_idx[r]));
            return;
        }
        if dcol != 0 {
            let ncols = Status::ALL.len() as i32;
            let mut c = col as i32 + dcol;
            while (0..ncols).contains(&c) {
                let idxs = self.indices_in_status(Status::ALL[c as usize]);
                if !idxs.is_empty() {
                    let r = row.min(idxs.len() - 1);
                    self.list_state.select(Some(idxs[r]));
                    return;
                }
                c += dcol;
            }
        }
    }

    /// 주어진 상태에 속한 작업들의 `self.tasks` 인덱스 목록(표시 순서 유지).
    pub fn indices_in_status(&self, status: Status) -> Vec<usize> {
        self.tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.status == status)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.list_state.selected()
    }

    // ── 검색 / 필터 ──────────────────────────────────────────────

    pub fn start_search(&mut self) {
        self.mode = Mode::Search(self.filter.text.clone().unwrap_or_default());
    }
    pub fn search_input(&mut self, c: char) {
        if let Mode::Search(s) = &mut self.mode {
            s.push(c);
        }
    }
    pub fn search_backspace(&mut self) {
        if let Mode::Search(s) = &mut self.mode {
            s.pop();
        }
    }
    /// 검색을 확정한다(텍스트 필터 적용 후 새로고침).
    pub fn submit_search(&mut self) {
        let Mode::Search(query) = std::mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };
        let q = query.trim();
        self.filter.text = if q.is_empty() { None } else { Some(q.to_string()) };
        self.refresh();
    }

    /// 상태 필터를 순환한다(전체→Open→…→Done→전체).
    pub fn cycle_status_filter(&mut self) {
        self.filter.status = match self.filter.status {
            None => Some(Status::Open),
            Some(Status::Open) => Some(Status::InProgress),
            Some(Status::InProgress) => Some(Status::Blocked),
            Some(Status::Blocked) => Some(Status::Deferred),
            Some(Status::Deferred) => Some(Status::Done),
            Some(Status::Done) => None,
        };
        self.refresh();
    }

    /// 모든 필터를 해제한다.
    pub fn clear_filter(&mut self) {
        if !self.filter.is_empty() {
            self.filter = Filter::default();
            self.refresh();
            self.status = "필터 해제".into();
        }
    }

    pub fn show_help(&mut self) {
        self.mode = Mode::Help;
    }

    /// 상태바에 표시할 필터 요약(없으면 빈 문자열).
    pub fn filter_summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(s) = self.filter.status {
            parts.push(format!("상태={}", s.label()));
        }
        if let Some(t) = &self.filter.text {
            parts.push(format!("검색='{t}'"));
        }
        if parts.is_empty() { String::new() } else { format!("[{}] ", parts.join(" ")) }
    }
}

/// 공백뿐이거나 빈 문자열이면 `None`.
fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}
