//! 인메모리 [`TaskRepository`] 구현. 외부 백엔드 없이 동작하며,
//! 단위 테스트와 `--backend memory` 데모에 쓰인다. 종료 시 데이터는 사라진다.

use std::cell::{Cell, RefCell};

use crate::domain::model::{Filter, NewTask, Status, Task, TaskPatch};
use crate::error::{Error, Result};
use crate::ports::TaskRepository;

/// 단일 스레드(TUI) 가정의 인메모리 저장소. 내부 가변성에 `RefCell`을 쓴다.
#[derive(Default)]
pub struct MemoryRepository {
    tasks: RefCell<Vec<Task>>,
    next_id: Cell<u64>,
}

impl MemoryRepository {
    pub fn new() -> Self {
        Self { tasks: RefCell::new(Vec::new()), next_id: Cell::new(1) }
    }

    fn find_index(&self, id: &str) -> Result<usize> {
        self.tasks
            .borrow()
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| Error::NotFound(id.to_string()))
    }
}

impl TaskRepository for MemoryRepository {
    fn name(&self) -> &str {
        "memory"
    }

    fn list(&self, filter: &Filter) -> Result<Vec<Task>> {
        Ok(self.tasks.borrow().iter().filter(|t| filter.matches(t)).cloned().collect())
    }

    fn get(&self, id: &str) -> Result<Task> {
        let idx = self.find_index(id)?;
        Ok(self.tasks.borrow()[idx].clone())
    }

    fn create(&self, task: &NewTask) -> Result<String> {
        let n = self.next_id.get();
        self.next_id.set(n + 1);
        let id = format!("mem-{n}");
        self.tasks.borrow_mut().push(Task {
            id: id.clone(),
            title: task.title.clone(),
            description: task.description.clone(),
            status: Status::Open,
            priority: task.priority,
            assignee: None,
            labels: task.labels.clone(),
            parent: task.parent.clone(),
        });
        Ok(id)
    }

    fn update(&self, id: &str, patch: &TaskPatch) -> Result<()> {
        let idx = self.find_index(id)?;
        let mut tasks = self.tasks.borrow_mut();
        let t = &mut tasks[idx];
        if let Some(v) = &patch.title {
            t.title = v.clone();
        }
        if let Some(v) = &patch.description {
            t.description = Some(v.clone());
        }
        if let Some(v) = patch.priority {
            t.priority = v;
        }
        if let Some(v) = patch.status {
            t.status = v;
        }
        if let Some(v) = &patch.assignee {
            t.assignee = Some(v.clone());
        }
        if let Some(v) = &patch.labels {
            t.labels = v.clone();
        }
        Ok(())
    }

    fn set_status(&self, id: &str, status: Status) -> Result<()> {
        let idx = self.find_index(id)?;
        self.tasks.borrow_mut()[idx].status = status;
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<()> {
        let idx = self.find_index(id)?;
        self.tasks.borrow_mut().remove(idx);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_assigns_open_status_and_id() {
        let repo = MemoryRepository::new();
        let id = repo.create(&NewTask { title: "a".into(), ..Default::default() }).unwrap();
        assert_eq!(id, "mem-1");
        assert_eq!(repo.get(&id).unwrap().status, Status::Open);
    }

    #[test]
    fn update_changes_fields() {
        let repo = MemoryRepository::new();
        let id = repo.create(&NewTask { title: "a".into(), ..Default::default() }).unwrap();
        repo.update(&id, &TaskPatch { title: Some("b".into()), ..Default::default() }).unwrap();
        assert_eq!(repo.get(&id).unwrap().title, "b");
    }

    #[test]
    fn missing_id_is_not_found() {
        let repo = MemoryRepository::new();
        assert!(matches!(repo.get("nope"), Err(Error::NotFound(_))));
    }
}
