//! 앱 전역 에러 타입. 도메인 코어와 어댑터가 공유한다.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    /// 주어진 id의 작업이 없음.
    #[error("작업을 찾을 수 없습니다: {0}")]
    NotFound(String),

    /// 외부 백엔드(CLI 등) 호출 실패.
    #[error("백엔드 오류: {0}")]
    Backend(String),

    /// 설정 로드/저장 실패.
    #[error("설정 오류: {0}")]
    Config(String),

    /// 입력 검증 실패(빈 제목 등).
    #[error("입력 오류: {0}")]
    Invalid(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("JSON 처리 오류: {0}")]
    Json(#[from] serde_json::Error),
}
