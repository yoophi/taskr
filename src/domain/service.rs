//! 애플리케이션 서비스(유스케이스). 인바운드 포트 [`TaskApi`]를 구현하고
//! 아웃바운드 포트 [`TaskRepository`]에 위임한다. 백엔드 무관 도메인 규칙
//! (입력 검증, 목록 정렬 등)이 여기에 모인다.

use crate::domain::model::{Filter, NewTask, Status, Task, TaskPatch};
use crate::error::{Error, Result};
use crate::ports::{TaskApi, TaskRepository};

pub struct TaskService {
    repo: Box<dyn TaskRepository>,
}

impl TaskService {
    pub fn new(repo: Box<dyn TaskRepository>) -> Self {
        Self { repo }
    }
}

impl TaskApi for TaskService {
    fn backend_name(&self) -> &str {
        self.repo.name()
    }

    fn list(&self, filter: &Filter) -> Result<Vec<Task>> {
        let mut tasks = self.repo.list(filter)?;
        // 백엔드가 텍스트 검색을 지원하지 않을 수 있으므로 한 번 더 거른다.
        if filter.text.is_some() {
            tasks.retain(|t| filter.matches(t));
        }
        // 상태(진행 중 먼저) → 우선순위(높은 것 먼저) → 제목 순.
        tasks.sort_by(|a, b| {
            a.status
                .order()
                .cmp(&b.status.order())
                .then(a.priority.cmp(&b.priority))
                .then_with(|| a.title.cmp(&b.title))
        });
        Ok(tasks)
    }

    fn get(&self, id: &str) -> Result<Task> {
        self.repo.get(id)
    }

    fn create(&self, task: &NewTask) -> Result<String> {
        if task.title.trim().is_empty() {
            return Err(Error::Invalid("제목은 비워 둘 수 없습니다".into()));
        }
        self.repo.create(task)
    }

    fn update(&self, id: &str, patch: &TaskPatch) -> Result<()> {
        if patch.is_empty() {
            return Ok(()); // 변경 없음 — 백엔드 호출 생략.
        }
        if let Some(title) = &patch.title
            && title.trim().is_empty()
        {
            return Err(Error::Invalid("제목은 비워 둘 수 없습니다".into()));
        }
        self.repo.update(id, patch)
    }

    fn set_status(&self, id: &str, status: Status) -> Result<()> {
        self.repo.set_status(id, status)
    }

    fn delete(&self, id: &str) -> Result<()> {
        self.repo.delete(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::MemoryRepository;
    use crate::domain::model::Priority;

    fn service() -> TaskService {
        TaskService::new(Box::new(MemoryRepository::new()))
    }

    #[test]
    fn create_rejects_empty_title() {
        let svc = service();
        let err = svc.create(&NewTask { title: "   ".into(), ..Default::default() });
        assert!(matches!(err, Err(Error::Invalid(_))));
    }

    #[test]
    fn create_then_list_and_get() {
        let svc = service();
        let id = svc
            .create(&NewTask { title: "첫 작업".into(), priority: Priority::P1, ..Default::default() })
            .unwrap();

        let all = svc.list(&Filter::default()).unwrap();
        assert_eq!(all.len(), 1);

        let got = svc.get(&id).unwrap();
        assert_eq!(got.title, "첫 작업");
        assert_eq!(got.priority, Priority::P1);
    }

    #[test]
    fn list_is_sorted_in_progress_before_open() {
        let svc = service();
        svc.create(&NewTask { title: "열린 작업".into(), ..Default::default() }).unwrap();
        let id2 = svc.create(&NewTask { title: "진행 작업".into(), ..Default::default() }).unwrap();
        svc.set_status(&id2, Status::InProgress).unwrap();

        let all = svc.list(&Filter::default()).unwrap();
        assert_eq!(all[0].status, Status::InProgress);
    }

    #[test]
    fn update_empty_patch_is_noop() {
        let svc = service();
        let id = svc.create(&NewTask { title: "x".into(), ..Default::default() }).unwrap();
        assert!(svc.update(&id, &TaskPatch::default()).is_ok());
    }

    #[test]
    fn delete_then_get_is_not_found() {
        let svc = service();
        let id = svc.create(&NewTask { title: "지울 작업".into(), ..Default::default() }).unwrap();
        svc.delete(&id).unwrap();
        assert!(matches!(svc.get(&id), Err(Error::NotFound(_))));
    }
}
