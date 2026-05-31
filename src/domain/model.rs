//! 백엔드 무관 도메인 모델. beads/backlog.md 등 인프라의 표현을 이 공통 타입으로 정규화한다.
//! 순수 데이터 + 순수 함수만 둔다(I/O 없음).

use serde::{Deserialize, Serialize};

/// 작업 상태. 각 백엔드의 상태 문자열은 어댑터에서 이 enum으로 매핑한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Done,
}

impl Status {
    /// 보드 컬럼/필터에 쓰는 전체 상태 목록(표시 순서).
    pub const ALL: [Status; 5] = [
        Status::Open,
        Status::InProgress,
        Status::Blocked,
        Status::Deferred,
        Status::Done,
    ];

    /// 사람이 읽는 라벨.
    pub fn label(self) -> &'static str {
        match self {
            Status::Open => "Open",
            Status::InProgress => "In Progress",
            Status::Blocked => "Blocked",
            Status::Deferred => "Deferred",
            Status::Done => "Done",
        }
    }

    /// 리스트 좌측에 붙이는 한 글자 심볼.
    pub fn symbol(self) -> &'static str {
        match self {
            Status::Open => "○",
            Status::InProgress => "◐",
            Status::Blocked => "✗",
            Status::Deferred => "⏸",
            Status::Done => "●",
        }
    }

    /// 리스트 정렬용 가중치(진행 중 → 열림 → … → 완료 순).
    pub fn order(self) -> u8 {
        match self {
            Status::InProgress => 0,
            Status::Open => 1,
            Status::Blocked => 2,
            Status::Deferred => 3,
            Status::Done => 4,
        }
    }
}

/// 우선순위. beads의 P0–P4와 1:1, backlog.md의 high/medium/low는 어댑터에서 매핑한다.
/// 0(P0)이 가장 높다 — `Ord`가 그대로 "높은 우선순위 먼저"를 의미하도록 변형 순서를 맞춘다.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Priority {
    P0,
    P1,
    #[default]
    P2,
    P3,
    P4,
}

impl Priority {
    pub fn label(self) -> &'static str {
        match self {
            Priority::P0 => "P0",
            Priority::P1 => "P1",
            Priority::P2 => "P2",
            Priority::P3 => "P3",
            Priority::P4 => "P4",
        }
    }

    /// 숫자(0–4)로 변환. CLI 인자 전달용.
    pub fn as_num(self) -> u8 {
        match self {
            Priority::P0 => 0,
            Priority::P1 => 1,
            Priority::P2 => 2,
            Priority::P3 => 3,
            Priority::P4 => 4,
        }
    }

    /// 숫자(0–4)에서 변환. 범위를 벗어나면 가장 가까운 값으로 클램프.
    pub fn from_num(n: u8) -> Self {
        match n {
            0 => Priority::P0,
            1 => Priority::P1,
            2 => Priority::P2,
            3 => Priority::P3,
            _ => Priority::P4,
        }
    }
}

/// 정규화된 작업 한 건.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: Status,
    pub priority: Priority,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
    pub parent: Option<String>,
}

/// 새 작업 생성 입력.
#[derive(Debug, Clone, Default)]
pub struct NewTask {
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub labels: Vec<String>,
    pub parent: Option<String>,
}

/// 작업 부분 수정 입력. `None`인 필드는 변경하지 않는다.
#[derive(Debug, Clone, Default)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub status: Option<Status>,
    pub assignee: Option<String>,
    pub labels: Option<Vec<String>>,
}

impl TaskPatch {
    /// 변경할 필드가 하나도 없는지.
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.priority.is_none()
            && self.status.is_none()
            && self.assignee.is_none()
            && self.labels.is_none()
    }
}

/// 목록 조회 필터. 어댑터는 가능한 한 백엔드에 위임하고,
/// 텍스트 검색 등 백엔드가 지원하지 않는 부분은 [`Filter::matches`]로 클라이언트에서 거른다.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub status: Option<Status>,
    pub priority: Option<Priority>,
    pub labels: Vec<String>,
    pub text: Option<String>,
}

impl Filter {
    /// 필터 조건이 하나도 없는지.
    pub fn is_empty(&self) -> bool {
        self.status.is_none() && self.priority.is_none() && self.labels.is_empty() && self.text.is_none()
    }

    /// 작업이 필터를 통과하는지(클라이언트 측 평가).
    pub fn matches(&self, task: &Task) -> bool {
        if let Some(s) = self.status
            && task.status != s
        {
            return false;
        }
        if let Some(p) = self.priority
            && task.priority != p
        {
            return false;
        }
        if !self.labels.iter().all(|l| task.labels.contains(l)) {
            return false;
        }
        if let Some(text) = &self.text {
            let needle = text.to_lowercase();
            let hay_title = task.title.to_lowercase();
            let hay_desc = task.description.as_deref().unwrap_or("").to_lowercase();
            if !hay_title.contains(&needle) && !hay_desc.contains(&needle) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(title: &str, status: Status, priority: Priority) -> Task {
        Task {
            id: "t-1".into(),
            title: title.into(),
            description: None,
            status,
            priority,
            assignee: None,
            labels: vec![],
            parent: None,
        }
    }

    #[test]
    fn priority_num_roundtrip() {
        for p in [Priority::P0, Priority::P1, Priority::P2, Priority::P3, Priority::P4] {
            assert_eq!(Priority::from_num(p.as_num()), p);
        }
        assert_eq!(Priority::from_num(9), Priority::P4); // 범위 초과 클램프
    }

    #[test]
    fn priority_ord_high_first() {
        assert!(Priority::P0 < Priority::P2);
    }

    #[test]
    fn empty_filter_matches_everything() {
        let f = Filter::default();
        assert!(f.is_empty());
        assert!(f.matches(&task("anything", Status::Open, Priority::P2)));
    }

    #[test]
    fn filter_by_status_and_text() {
        let f = Filter {
            status: Some(Status::Open),
            text: Some("API".into()),
            ..Default::default()
        };
        assert!(f.matches(&task("Fix API bug", Status::Open, Priority::P1)));
        assert!(!f.matches(&task("Fix API bug", Status::Done, Priority::P1))); // 상태 불일치
        assert!(!f.matches(&task("unrelated", Status::Open, Priority::P1))); // 텍스트 불일치
    }
}
