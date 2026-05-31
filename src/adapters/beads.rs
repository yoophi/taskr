//! beads(`bd` CLI) 아웃바운드 어댑터. `bd ... --json`을 셸아웃해 결과를 파싱한다.
//!
//! 검증된 사실(스크래치 DB 기준):
//! - `bd list --json` / `bd show --json` 모두 **이슈 객체 배열**을 반환한다(단건도 배열).
//! - `--silent` 생성 시 경고는 stderr로, 순수 ID는 stdout으로 나간다.
//! - 상태값: `open` / `in_progress` / `blocked` / `deferred` / `closed`(완료).
//! - 완료 = `bd close <id>`, 그 외/재오픈 = `bd update <id> --status <s>`.
//! - 비대화형 삭제 = `bd delete <id> --force`.

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

use crate::config::BeadsConfig;
use crate::domain::model::{Filter, NewTask, Priority, Status, Task, TaskPatch};
use crate::error::{Error, Result};
use crate::ports::TaskRepository;

/// beads 이슈 JSON DTO. 필요한 필드만 취하고, 없을 수 있는 필드는 기본값으로 둔다.
#[derive(Debug, Deserialize)]
struct BeadIssue {
    id: String,
    title: String,
    #[serde(default)]
    description: Option<String>,
    status: String,
    priority: u8,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    parent: Option<String>,
}

fn status_from_beads(s: &str) -> Status {
    match s {
        "in_progress" => Status::InProgress,
        "blocked" => Status::Blocked,
        "deferred" => Status::Deferred,
        "closed" => Status::Done,
        _ => Status::Open,
    }
}

fn status_to_beads(s: Status) -> &'static str {
    match s {
        Status::Open => "open",
        Status::InProgress => "in_progress",
        Status::Blocked => "blocked",
        Status::Deferred => "deferred",
        Status::Done => "closed",
    }
}

impl From<BeadIssue> for Task {
    fn from(i: BeadIssue) -> Self {
        Task {
            id: i.id,
            title: i.title,
            description: i.description.filter(|d| !d.is_empty()),
            status: status_from_beads(&i.status),
            priority: Priority::from_num(i.priority),
            assignee: i.assignee.filter(|a| !a.is_empty()),
            labels: i.labels,
            parent: i.parent,
        }
    }
}

pub struct BeadsRepository {
    /// `BEADS_DIR` 오버라이드. `None`이면 `bd`의 기본(cwd/환경변수)을 따른다.
    dir: Option<PathBuf>,
}

impl BeadsRepository {
    pub fn new(cfg: &BeadsConfig) -> Self {
        Self { dir: cfg.dir.clone() }
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new("bd");
        if let Some(dir) = &self.dir {
            cmd.env("BEADS_DIR", dir);
        }
        cmd
    }

    /// `bd`를 실행하고 stdout을 돌려준다. 종료 코드가 0이 아니면 stderr를 담아 에러로 반환.
    fn run<I, S>(&self, args: I) -> Result<String>
    where
        I: IntoIterator<Item = S> + Clone,
        S: AsRef<OsStr>,
    {
        let output = self
            .command()
            .args(args.clone())
            .output()
            .map_err(|e| Error::Backend(format!("bd 실행 실패(설치/경로 확인): {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let argline: Vec<String> =
                args.into_iter().map(|s| s.as_ref().to_string_lossy().into_owned()).collect();
            return Err(Error::Backend(format!("`bd {}` 실패: {}", argline.join(" "), stderr.trim())));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn parse_issues(json: &str) -> Result<Vec<Task>> {
        let issues: Vec<BeadIssue> = serde_json::from_str(json)?;
        Ok(issues.into_iter().map(Task::from).collect())
    }
}

impl TaskRepository for BeadsRepository {
    fn name(&self) -> &str {
        "beads"
    }

    fn list(&self, filter: &Filter) -> Result<Vec<Task>> {
        let mut args: Vec<String> =
            vec!["list".into(), "--all".into(), "--json".into(), "--no-pager".into()];
        if let Some(s) = filter.status {
            args.push("--status".into());
            args.push(status_to_beads(s).into());
        }
        if let Some(p) = filter.priority {
            args.push("--priority".into());
            args.push(p.as_num().to_string());
        }
        for label in &filter.labels {
            args.push("--label".into());
            args.push(label.clone());
        }
        // 텍스트 검색은 백엔드에 위임하지 않고 서비스 계층(Filter::matches)에서 거른다.
        let out = self.run(&args)?;
        Self::parse_issues(&out)
    }

    fn get(&self, id: &str) -> Result<Task> {
        let out = self.run(["show", id, "--json"])?;
        Self::parse_issues(&out)?
            .into_iter()
            .next()
            .ok_or_else(|| Error::NotFound(id.to_string()))
    }

    fn create(&self, task: &NewTask) -> Result<String> {
        let mut args: Vec<String> = vec!["create".into(), task.title.clone(), "--silent".into()];
        if let Some(d) = &task.description
            && !d.is_empty()
        {
            args.push("-d".into());
            args.push(d.clone());
        }
        args.push("-p".into());
        args.push(task.priority.as_num().to_string());
        if !task.labels.is_empty() {
            args.push("-l".into());
            args.push(task.labels.join(","));
        }
        if let Some(parent) = &task.parent {
            args.push("--parent".into());
            args.push(parent.clone());
        }
        let out = self.run(&args)?;
        // --silent는 stdout에 ID만 출력한다. 안전하게 마지막 비어 있지 않은 줄을 취한다.
        let id = out.lines().map(str::trim).filter(|l| !l.is_empty()).next_back().unwrap_or("");
        if id.is_empty() {
            return Err(Error::Backend("생성된 이슈 ID를 파싱하지 못했습니다".into()));
        }
        Ok(id.to_string())
    }

    fn update(&self, id: &str, patch: &TaskPatch) -> Result<()> {
        // 비-상태 필드는 한 번의 `bd update`로 적용한다.
        let mut args: Vec<String> = vec!["update".into(), id.to_string()];
        let mut has_field = false;
        if let Some(t) = &patch.title {
            args.push("--title".into());
            args.push(t.clone());
            has_field = true;
        }
        if let Some(d) = &patch.description {
            args.push("-d".into());
            args.push(d.clone());
            has_field = true;
        }
        if let Some(p) = patch.priority {
            args.push("-p".into());
            args.push(p.as_num().to_string());
            has_field = true;
        }
        if let Some(a) = &patch.assignee {
            args.push("-a".into());
            args.push(a.clone());
            has_field = true;
        }
        if let Some(labels) = &patch.labels {
            args.push("--set-labels".into());
            args.push(labels.join(","));
            has_field = true;
        }
        if has_field {
            self.run(&args)?;
        }
        // 상태 변경은 close/update 분기가 다르므로 set_status에 위임한다.
        if let Some(s) = patch.status {
            self.set_status(id, s)?;
        }
        Ok(())
    }

    fn set_status(&self, id: &str, status: Status) -> Result<()> {
        match status {
            Status::Done => {
                self.run(["close", id])?;
            }
            other => {
                self.run(["update", id, "--status", status_to_beads(other)])?;
            }
        }
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<()> {
        self.run(["delete", id, "--force"])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 실제 `bd list --json` 출력에서 가져온 픽스처(환경 비의존 파싱 검증).
    const FIXTURE: &str = r#"[
      { "id": "p-xec", "title": "API 인증 버그 수정", "description": "토큰 만료 처리",
        "status": "open", "priority": 1, "issue_type": "task",
        "owner": "me@example.com", "labels": ["backend", "bug"] },
      { "id": "p-v2a.1", "title": "자식작업", "status": "closed", "priority": 2,
        "issue_type": "task", "parent": "p-v2a" }
    ]"#;

    #[test]
    fn parses_fixture_into_tasks() {
        let tasks = BeadsRepository::parse_issues(FIXTURE).unwrap();
        assert_eq!(tasks.len(), 2);

        let t0 = &tasks[0];
        assert_eq!(t0.id, "p-xec");
        assert_eq!(t0.status, Status::Open);
        assert_eq!(t0.priority, Priority::P1);
        assert_eq!(t0.labels, vec!["backend", "bug"]);
        assert_eq!(t0.description.as_deref(), Some("토큰 만료 처리"));

        let t1 = &tasks[1];
        assert_eq!(t1.status, Status::Done); // closed → Done
        assert_eq!(t1.parent.as_deref(), Some("p-v2a"));
        assert!(t1.description.is_none());
    }

    #[test]
    fn status_mapping_roundtrip() {
        for s in Status::ALL {
            assert_eq!(status_from_beads(status_to_beads(s)), s);
        }
        assert_eq!(status_from_beads("unknown"), Status::Open); // 미지 상태 안전 폴백
    }

    // bd가 설치되어 있을 때만 도는 엔드투엔드 통합 테스트.
    // 임시 BEADS_DIR에서 create→list→get→update→set_status→delete 전 과정을 검증한다.
    // beads 데몬/init 오버헤드로 수십 초 걸리므로 기본 실행에서 제외한다.
    // 수동 실행: `cargo test -- --ignored`
    #[test]
    #[ignore = "bd 설치 필요 + 수십 초 소요 — 수동 실행"]
    fn end_to_end_with_bd() {
        // bd 미설치/미동작 환경에서는 조용히 건너뛴다.
        if Command::new("bd").arg("version").output().map(|o| !o.status.success()).unwrap_or(true) {
            eprintln!("bd 미설치 — end_to_end_with_bd 건너뜀");
            return;
        }

        let dir = std::env::temp_dir().join(format!("taskr-it-{}", std::process::id()));
        let beads_dir = dir.join(".beads");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&beads_dir).unwrap();
        // 임시 DB 초기화.
        let init_ok = Command::new("bd")
            .arg("init")
            .env("BEADS_DIR", &beads_dir)
            .current_dir(&dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        assert!(init_ok, "bd init 실패");

        let repo = BeadsRepository::new(&BeadsConfig { dir: Some(beads_dir.clone()) });

        let id = repo
            .create(&NewTask {
                title: "통합 테스트 작업".into(),
                description: Some("설명".into()),
                priority: Priority::P1,
                labels: vec!["it".into()],
                ..Default::default()
            })
            .unwrap();

        let got = repo.get(&id).unwrap();
        assert_eq!(got.title, "통합 테스트 작업");
        assert_eq!(got.priority, Priority::P1);
        assert_eq!(got.status, Status::Open);

        repo.set_status(&id, Status::InProgress).unwrap();
        assert_eq!(repo.get(&id).unwrap().status, Status::InProgress);

        repo.update(&id, &TaskPatch { title: Some("수정됨".into()), ..Default::default() }).unwrap();
        assert_eq!(repo.get(&id).unwrap().title, "수정됨");

        repo.set_status(&id, Status::Done).unwrap();
        assert_eq!(repo.get(&id).unwrap().status, Status::Done);

        let all = repo.list(&Filter::default()).unwrap();
        assert!(all.iter().any(|t| t.id == id));

        repo.delete(&id).unwrap();

        let _ = std::fs::remove_dir_all(&dir);
    }
}
