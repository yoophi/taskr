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
}

pub struct App {
    api: Box<dyn TaskApi>,
    pub config_path: String,
    pub tasks: Vec<Task>,
    pub list_state: ListState,
    pub view: View,
    pub mode: Mode,
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
            status: String::new(),
            should_quit: false,
        }
    }

    pub fn backend_name(&self) -> &str {
        self.api.backend_name()
    }

    // ── 목록/선택 ────────────────────────────────────────────────

    pub fn refresh(&mut self) {
        match self.api.list(&Filter::default()) {
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
}

/// 공백뿐이거나 빈 문자열이면 `None`.
fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}
