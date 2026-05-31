//! backlog.md(`backlog` CLI) 아웃바운드 어댑터.
//!
//! backlog.md는 `--json`을 지원하지 않으므로 `--plain` 텍스트 출력을 파싱한다.
//! 검증된 사실(스크래치 프로젝트 기준):
//! - `backlog task list --plain`: 상태 그룹 헤더(`To Do:` 등) + `  [PRIORITY] TASK-N - 제목`.
//! - `backlog task <id> --plain`: `Status:` / `Priority:` / `Description:` 섹션.
//! - 상태는 기본 3종: `To Do` / `In Progress` / `Done`. 우선순위는 `high/medium/low`.
//! - 생성=`task create`, 수정=`task edit`, 상태=`task edit -s`, 삭제는 없으므로 `task archive`.
//!
//! 제약: backlog 기본 상태가 3종이라 Blocked/Deferred는 To Do로 매핑한다(역매핑 불가).

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

use crate::config::BacklogConfig;
use crate::domain::model::{Filter, NewTask, Priority, Status, Task, TaskPatch};
use crate::error::{Error, Result};
use crate::ports::TaskRepository;

fn status_from_backlog(s: &str) -> Status {
    // "○ To Do"처럼 심볼이 붙을 수 있어 포함 여부로 판별한다.
    if s.contains("In Progress") {
        Status::InProgress
    } else if s.contains("Done") {
        Status::Done
    } else {
        Status::Open
    }
}

fn status_to_backlog(s: Status) -> &'static str {
    match s {
        Status::InProgress => "In Progress",
        Status::Done => "Done",
        // backlog 기본 상태에 없는 값은 To Do로 둔다.
        Status::Open | Status::Blocked | Status::Deferred => "To Do",
    }
}

fn priority_from_backlog(s: &str) -> Priority {
    match s.trim().to_ascii_lowercase().as_str() {
        "high" => Priority::P1,
        "low" => Priority::P3,
        _ => Priority::P2,
    }
}

fn priority_to_backlog(p: Priority) -> &'static str {
    match p {
        Priority::P0 | Priority::P1 => "high",
        Priority::P2 => "medium",
        Priority::P3 | Priority::P4 => "low",
    }
}

pub struct BacklogMdRepository {
    /// backlog 프로젝트 경로(`backlog/` 디렉터리가 있는 곳). `None`이면 현재 작업 디렉터리.
    project_path: Option<PathBuf>,
}

impl BacklogMdRepository {
    pub fn new(cfg: &BacklogConfig) -> Self {
        Self { project_path: cfg.project_path.clone() }
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new("backlog");
        if let Some(dir) = &self.project_path {
            cmd.current_dir(dir);
        }
        cmd
    }

    fn run<I, S>(&self, args: I) -> Result<String>
    where
        I: IntoIterator<Item = S> + Clone,
        S: AsRef<OsStr>,
    {
        let output = self
            .command()
            .args(args.clone())
            .output()
            .map_err(|e| Error::Backend(format!("backlog 실행 실패(설치/경로 확인): {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let argline: Vec<String> =
                args.into_iter().map(|s| s.as_ref().to_string_lossy().into_owned()).collect();
            return Err(Error::Backend(format!(
                "`backlog {}` 실패: {}",
                argline.join(" "),
                stderr.trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// `backlog task list --plain` 출력을 파싱한다.
    fn parse_list(text: &str) -> Vec<Task> {
        let mut tasks = Vec::new();
        let mut current = Status::Open;
        for raw in text.lines() {
            let line = raw.trim_end();
            if line.trim().is_empty() {
                continue;
            }
            // 들여쓰기 없는 "그룹명:" → 상태 헤더.
            if !raw.starts_with(' ') && line.ends_with(':') {
                current = status_from_backlog(line.trim_end_matches(':'));
                continue;
            }
            if let Some(task) = parse_list_item(line.trim(), current) {
                tasks.push(task);
            }
        }
        tasks
    }
}

/// "  [HIGH] TASK-1 - 제목" 한 줄을 파싱한다.
fn parse_list_item(line: &str, status: Status) -> Option<Task> {
    let mut rest = line;
    let mut priority = Priority::default();
    if let Some(end) = rest.strip_prefix('[').and_then(|r| r.find(']').map(|i| (r, i))) {
        let (after_bracket, idx) = end;
        priority = priority_from_backlog(&after_bracket[..idx]);
        rest = after_bracket[idx + 1..].trim_start();
    }
    // "TASK-1 - 제목"
    let (id, title) = rest.split_once(" - ")?;
    let id = id.trim();
    if id.is_empty() {
        return None;
    }
    Some(Task {
        id: id.to_string(),
        title: title.trim().to_string(),
        description: None, // 목록에는 설명이 없다. 상세는 get()에서 채운다.
        status,
        priority,
        assignee: None,
        labels: Vec::new(),
        parent: None,
    })
}

/// `backlog task <id> --plain` 출력에서 상태/우선순위/설명을 파싱한다.
fn parse_view(id: &str, text: &str) -> Task {
    let mut status = Status::Open;
    let mut priority = Priority::default();
    let mut title = id.to_string();
    let mut description = String::new();
    let mut in_desc = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(v) = trimmed.strip_prefix("Task ") {
            // "Task TASK-1 - 제목"
            if let Some((_, t)) = v.split_once(" - ") {
                title = t.trim().to_string();
            }
        } else if let Some(v) = trimmed.strip_prefix("Status:") {
            status = status_from_backlog(v);
            in_desc = false;
        } else if let Some(v) = trimmed.strip_prefix("Priority:") {
            priority = priority_from_backlog(v);
            in_desc = false;
        } else if trimmed == "Description:" {
            in_desc = true;
        } else if trimmed.ends_with(':') && trimmed.chars().next().is_some_and(|c| c.is_uppercase())
        {
            // 다음 섹션 헤더(Acceptance Criteria: 등) → 설명 끝.
            in_desc = false;
        } else if in_desc && !trimmed.starts_with("---") {
            if !description.is_empty() {
                description.push('\n');
            }
            description.push_str(line.trim_end());
        }
    }

    let description = description.trim();
    Task {
        id: id.to_string(),
        title,
        description: if description.is_empty() || description.starts_with("No ") {
            None
        } else {
            Some(description.to_string())
        },
        status,
        priority,
        assignee: None,
        labels: Vec::new(),
        parent: None,
    }
}

impl TaskRepository for BacklogMdRepository {
    fn name(&self) -> &str {
        "backlog.md"
    }

    fn list(&self, filter: &Filter) -> Result<Vec<Task>> {
        let out = self.run(["task", "list", "--plain"])?;
        let mut tasks = Self::parse_list(&out);
        // backlog 목록 필터는 제한적이므로 공통 필터를 클라이언트에서 적용한다.
        if !filter.is_empty() {
            tasks.retain(|t| filter.matches(t));
        }
        Ok(tasks)
    }

    fn get(&self, id: &str) -> Result<Task> {
        let out = self.run(["task", id, "--plain"])?;
        Ok(parse_view(id, &out))
    }

    fn create(&self, task: &NewTask) -> Result<String> {
        let mut args: Vec<String> = vec!["task".into(), "create".into(), task.title.clone()];
        if let Some(d) = &task.description
            && !d.is_empty()
        {
            args.push("-d".into());
            args.push(d.clone());
        }
        args.push("--priority".into());
        args.push(priority_to_backlog(task.priority).into());
        if !task.labels.is_empty() {
            args.push("-l".into());
            args.push(task.labels.join(","));
        }
        let out = self.run(&args)?;
        // "Created task TASK-1" 에서 ID 추출.
        let id = out
            .lines()
            .find_map(|l| l.trim().strip_prefix("Created task "))
            .map(|s| s.trim().to_string())
            .ok_or_else(|| Error::Backend("생성된 작업 ID를 파싱하지 못했습니다".into()))?;
        Ok(id)
    }

    fn update(&self, id: &str, patch: &TaskPatch) -> Result<()> {
        let mut args: Vec<String> = vec!["task".into(), "edit".into(), id.to_string()];
        if let Some(t) = &patch.title {
            args.push("-t".into());
            args.push(t.clone());
        }
        if let Some(d) = &patch.description {
            args.push("-d".into());
            args.push(d.clone());
        }
        if let Some(p) = patch.priority {
            args.push("--priority".into());
            args.push(priority_to_backlog(p).into());
        }
        if let Some(s) = patch.status {
            args.push("-s".into());
            args.push(status_to_backlog(s).into());
        }
        if let Some(labels) = &patch.labels {
            args.push("-l".into());
            args.push(labels.join(","));
        }
        // edit 인자가 id뿐이면 변경 사항이 없으므로 호출하지 않는다.
        if args.len() > 3 {
            self.run(&args)?;
        }
        Ok(())
    }

    fn set_status(&self, id: &str, status: Status) -> Result<()> {
        self.run(["task", "edit", id, "-s", status_to_backlog(status)])?;
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<()> {
        // backlog에는 삭제가 없어 아카이브로 대체한다.
        self.run(["task", "archive", id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST_FIXTURE: &str = "To Do:\n  [HIGH] TASK-1 - 첫 작업\n  [LOW] TASK-2 - 둘째 작업\nIn Progress:\n  TASK-3 - 진행 중 작업\nDone:\n  [MEDIUM] TASK-4 - 완료 작업\n";

    const VIEW_FIXTURE: &str = "File: /x/backlog/tasks/task-1.md\n\nTask TASK-1 - 첫 작업\n==================================================\n\nStatus: ○ To Do\nPriority: High\nCreated: 2026-05-31 05:00\n\nDescription:\n--------------------------------------------------\n설명입니다\n둘째 줄\n\nAcceptance Criteria:\n--------------------------------------------------\nNo acceptance criteria defined\n";

    #[test]
    fn parses_list_groups_and_items() {
        let tasks = BacklogMdRepository::parse_list(LIST_FIXTURE);
        assert_eq!(tasks.len(), 4);
        assert_eq!(tasks[0].id, "TASK-1");
        assert_eq!(tasks[0].status, Status::Open);
        assert_eq!(tasks[0].priority, Priority::P1); // HIGH → P1
        assert_eq!(tasks[1].priority, Priority::P3); // LOW → P3
        assert_eq!(tasks[2].status, Status::InProgress);
        assert_eq!(tasks[2].priority, Priority::P2); // 우선순위 표기 없음 → 기본 P2
        assert_eq!(tasks[3].status, Status::Done);
        assert_eq!(tasks[3].title, "완료 작업");
    }

    #[test]
    fn parses_view_with_description() {
        let t = parse_view("TASK-1", VIEW_FIXTURE);
        assert_eq!(t.title, "첫 작업");
        assert_eq!(t.status, Status::Open);
        assert_eq!(t.priority, Priority::P1);
        assert_eq!(t.description.as_deref(), Some("설명입니다\n둘째 줄"));
    }

    #[test]
    fn status_priority_mapping() {
        assert_eq!(status_to_backlog(Status::InProgress), "In Progress");
        assert_eq!(status_to_backlog(Status::Blocked), "To Do"); // 폴백
        assert_eq!(priority_to_backlog(Priority::P0), "high");
        assert_eq!(priority_from_backlog("LOW"), Priority::P3);
    }
}
