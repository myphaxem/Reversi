//! AI対戦API データ転送オブジェクト (DTO)

use axum::{http::StatusCode, response::Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::game::{GameState, Position, Player, Move};
use crate::ai::Difficulty as LegacyDifficulty;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AiDifficulty {
    Easy,
    Medium,
    Hard,
}

impl AiDifficulty {
    pub fn all() -> Vec<AiDifficulty> {
        vec![AiDifficulty::Easy, AiDifficulty::Medium, AiDifficulty::Hard]
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            AiDifficulty::Easy => "初級 - ランダムな手を選択",
            AiDifficulty::Medium => "中級 - 基本的な戦略を使用", 
            AiDifficulty::Hard => "上級 - 高度な先読みを実行",
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            AiDifficulty::Easy => "Easy",
            AiDifficulty::Medium => "Medium", 
            AiDifficulty::Hard => "Hard",
        }
    }
}

impl FromStr for AiDifficulty {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "easy" => Ok(AiDifficulty::Easy),
            "medium" => Ok(AiDifficulty::Medium),
            "hard" => Ok(AiDifficulty::Hard),
            _ => Err(format!("Invalid difficulty: {}. Valid options: easy, medium, hard", s)),
        }
    }
}

impl From<AiDifficulty> for LegacyDifficulty {
    fn from(difficulty: AiDifficulty) -> Self {
        match difficulty {
            AiDifficulty::Easy => LegacyDifficulty::Beginner,
            AiDifficulty::Medium => LegacyDifficulty::Intermediate,
            AiDifficulty::Hard => LegacyDifficulty::Advanced,
        }
    }
}

impl From<LegacyDifficulty> for AiDifficulty {
    fn from(difficulty: LegacyDifficulty) -> Self {
        match difficulty {
            LegacyDifficulty::Beginner => AiDifficulty::Easy,
            LegacyDifficulty::Intermediate => AiDifficulty::Medium,
            LegacyDifficulty::Advanced => AiDifficulty::Hard,
        }
    }
}

pub fn validate_position(row: u8, col: u8) -> Result<Position, String> {
    if row >= 8 || col >= 8 {
        return Err(format!("座標が範囲外です: ({}, {}). 有効範囲: 0-7", row, col));
    }
    
    Position::new(row as usize, col as usize)
        .ok_or_else(|| format!("無効な座標です: ({}, {})", row, col))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveRecord {
    pub player: Player,
    pub position: Position,
    pub timestamp: DateTime<Utc>,
    pub thinking_time_ms: Option<u64>,
}

impl MoveRecord {
    pub fn new(player: Player, position: Position, thinking_time_ms: Option<u64>) -> Self {
        Self {
            player,
            position,
            timestamp: Utc::now(),
            thinking_time_ms,
        }
    }
    
    pub fn from_move(game_move: &Move, thinking_time_ms: Option<u64>) -> Self {
        Self {
            player: game_move.player,
            position: game_move.position,
            timestamp: game_move.timestamp,
            thinking_time_ms,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameStatus {
    InProgress,
    Finished { winner: Option<Player> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiBattleSession {
    pub id: Uuid,
    pub game_state: GameState,
    pub ai_difficulty: AiDifficulty,
    pub current_player: Player,
    pub ai_thinking: bool,
    pub created_at: DateTime<Utc>,
    pub last_move_at: DateTime<Utc>,
    pub move_history: Vec<MoveRecord>,
    pub status: GameStatus,
}

impl AiBattleSession {
    pub fn new(ai_difficulty: AiDifficulty) -> Self {
        let now = Utc::now();
        let game_state = GameState::new();
        
        Self {
            id: Uuid::new_v4(),
            game_state: game_state.clone(),
            ai_difficulty,
            current_player: game_state.current_player,
            ai_thinking: false,
            created_at: now,
            last_move_at: now,
            move_history: Vec::new(),
            status: GameStatus::InProgress,
        }
    }
    
    pub fn is_ai_turn(&self) -> bool {
        self.current_player == Player::White && !self.ai_thinking
    }
    
    pub fn is_player_turn(&self) -> bool {
        self.current_player == Player::Black
    }
    
    pub fn update_last_move(&mut self) {
        self.last_move_at = Utc::now();
    }
    
    pub fn add_move_record(&mut self, move_record: MoveRecord) {
        self.move_history.push(move_record);
        self.update_last_move();
    }
    
    pub fn is_finished(&self) -> bool {
        matches!(self.status, GameStatus::Finished { .. })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateAiBattleRequest {
    pub difficulty: AiDifficulty,
}

#[derive(Debug, Deserialize)]
pub struct PlayerMoveRequest {
    pub row: u8,
    pub col: u8,
}

#[derive(Debug, Deserialize)]
pub struct ChangeDifficultyRequest {
    pub difficulty: AiDifficulty,
}

#[derive(Debug, Serialize)]
pub struct AiBattleResponse {
    pub game_id: Uuid,
    pub board: Vec<Vec<Option<Player>>>,
    pub current_player: Player,
    pub black_count: u8,
    pub white_count: u8,
    pub ai_difficulty: AiDifficulty,
    pub ai_thinking: bool,
    pub status: GameStatus,
    pub valid_moves: Vec<Position>,
    pub move_count: u32,
}

impl AiBattleResponse {
    pub fn from_session(session: &AiBattleSession) -> Self {
        let mut board = vec![vec![None; 8]; 8];
        for row in 0..8 {
            for col in 0..8 {
                if let Some(position) = Position::new(row, col) {
                    if let Some(cell) = session.game_state.board.get_cell(position) {
                        board[row][col] = match cell {
                            crate::game::Cell::Empty => None,
                            crate::game::Cell::Black => Some(Player::Black),
                            crate::game::Cell::White => Some(Player::White),
                        };
                    }
                }
            }
        }
        
        let valid_moves = if session.is_finished() {
            Vec::new()
        } else {
            crate::game::ReversiRules::get_valid_moves(&session.game_state.board, session.current_player)
        };
        
        let (black_count, white_count) = session.game_state.get_score();
        
        Self {
            game_id: session.id,
            board,
            current_player: session.current_player,
            black_count,
            white_count,
            ai_difficulty: session.ai_difficulty,
            ai_thinking: session.ai_thinking,
            status: session.status,
            valid_moves,
            move_count: session.game_state.move_history.len() as u32,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MoveResponse {
    pub success: bool,
    pub game_state: AiBattleResponse,
    pub player_move: Position,
    pub ai_move: Option<Position>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionListResponse {
    pub sessions: Vec<SessionSummary>,
    pub total_count: usize,
}

#[derive(Debug, Serialize)]
pub struct SessionSummary {
    pub game_id: Uuid,
    pub ai_difficulty: AiDifficulty,
    pub status: GameStatus,
    pub created_at: DateTime<Utc>,
    pub last_move_at: DateTime<Utc>,
    pub move_count: u32,
}

impl SessionSummary {
    pub fn from_session(session: &AiBattleSession) -> Self {
        Self {
            game_id: session.id,
            ai_difficulty: session.ai_difficulty,
            status: session.status,
            created_at: session.created_at,
            last_move_at: session.last_move_at,
            move_count: session.game_state.move_history.len() as u32,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MoveHistoryResponse {
    pub game_id: Uuid,
    pub moves: Vec<MoveRecord>,
    pub total_moves: usize,
}

#[derive(Debug, Serialize)]
pub struct DifficultyInfo {
    pub id: AiDifficulty,
    pub name: &'static str,
    pub description: &'static str,
}

impl From<AiDifficulty> for DifficultyInfo {
    fn from(difficulty: AiDifficulty) -> Self {
        Self {
            id: difficulty,
            name: difficulty.name(),
            description: difficulty.description(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DifficultiesResponse {
    pub difficulties: Vec<DifficultyInfo>,
}

impl DifficultiesResponse {
    pub fn new() -> Self {
        Self {
            difficulties: AiDifficulty::all()
                .into_iter()
                .map(DifficultyInfo::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub error_code: Option<String>,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            timestamp: Utc::now(),
            error_code: None,
        }
    }
    
    pub fn with_code(error: impl Into<String>, message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            timestamp: Utc::now(),
            error_code: Some(code.into()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AiBattleError {
    #[error("ゲームセッションが見つかりません: {game_id}")]
    GameNotFound { game_id: Uuid },
    
    #[error("無効な着手です: {reason}")]
    InvalidMove { reason: String },
    
    #[error("プレイヤーの手番ではありません")]
    NotPlayerTurn,
    
    #[error("無効なAI難易度です: {difficulty}")]
    InvalidDifficulty { difficulty: String },
    
    #[error("セッション制限に達しています (最大: {max})")]
    MaxSessionsReached { max: usize },
    
    #[error("AI思考エラー: {details}")]
    AiThinkingError { details: String },
    
    #[error("ゲームは既に終了しています")]
    GameAlreadyFinished,
    
    #[error("無効なリクエストです: {details}")]
    BadRequest { details: String },
    
    #[error("サーバー内部エラー: {details}")]
    InternalError { details: String },
    
    #[error("ゲームエラー: {0}")]
    GameError(#[from] crate::error::GameError),
    
    #[error("AIエラー: {0}")]
    AIError(#[from] crate::error::AIError),
}

impl AiBattleError {
    pub fn error_code(&self) -> &'static str {
        match self {
            AiBattleError::GameNotFound { .. } => "GAME_NOT_FOUND",
            AiBattleError::InvalidMove { .. } => "INVALID_MOVE",
            AiBattleError::NotPlayerTurn => "NOT_PLAYER_TURN",
            AiBattleError::InvalidDifficulty { .. } => "INVALID_DIFFICULTY",
            AiBattleError::MaxSessionsReached { .. } => "MAX_SESSIONS_REACHED",
            AiBattleError::AiThinkingError { .. } => "AI_THINKING_ERROR",
            AiBattleError::GameAlreadyFinished => "GAME_ALREADY_FINISHED",
            AiBattleError::BadRequest { .. } => "BAD_REQUEST",
            AiBattleError::InternalError { .. } => "INTERNAL_ERROR",
            AiBattleError::GameError(_) => "GAME_ERROR",
            AiBattleError::AIError(_) => "AI_ERROR",
        }
    }
    
    pub fn status_code(&self) -> StatusCode {
        match self {
            AiBattleError::GameNotFound { .. } => StatusCode::NOT_FOUND,
            AiBattleError::InvalidMove { .. } => StatusCode::BAD_REQUEST,
            AiBattleError::NotPlayerTurn => StatusCode::FORBIDDEN,
            AiBattleError::InvalidDifficulty { .. } => StatusCode::BAD_REQUEST,
            AiBattleError::MaxSessionsReached { .. } => StatusCode::TOO_MANY_REQUESTS,
            AiBattleError::AiThinkingError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AiBattleError::GameAlreadyFinished => StatusCode::BAD_REQUEST,
            AiBattleError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            AiBattleError::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AiBattleError::GameError(_) => StatusCode::BAD_REQUEST,
            AiBattleError::AIError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<AiBattleError> for (StatusCode, Json<ErrorResponse>) {
    fn from(err: AiBattleError) -> Self {
        let status_code = err.status_code();
        let error_response = ErrorResponse::with_code(
            err.error_code(),
            err.to_string(),
            err.error_code(),
        );
        
        (status_code, Json(error_response))
    }
}

pub type AiBattleResult<T> = Result<T, AiBattleError>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ai_difficulty_all() {
        let all_difficulties = AiDifficulty::all();
        assert_eq!(all_difficulties.len(), 3);
        assert!(all_difficulties.contains(&AiDifficulty::Easy));
        assert!(all_difficulties.contains(&AiDifficulty::Medium));
        assert!(all_difficulties.contains(&AiDifficulty::Hard));
    }
    
    #[test]
    fn test_ai_difficulty_description() {
        assert!(AiDifficulty::Easy.description().contains("初級"));
        assert!(AiDifficulty::Medium.description().contains("中級"));
        assert!(AiDifficulty::Hard.description().contains("上級"));
    }
    
    #[test]
    fn test_ai_difficulty_name() {
        assert_eq!(AiDifficulty::Easy.name(), "Easy");
        assert_eq!(AiDifficulty::Medium.name(), "Medium");
        assert_eq!(AiDifficulty::Hard.name(), "Hard");
    }
    
    #[test]
    fn test_ai_difficulty_from_str() {
        assert_eq!("easy".parse::<AiDifficulty>().unwrap(), AiDifficulty::Easy);
        assert_eq!("MEDIUM".parse::<AiDifficulty>().unwrap(), AiDifficulty::Medium);
        assert_eq!("Hard".parse::<AiDifficulty>().unwrap(), AiDifficulty::Hard);
        assert!("invalid".parse::<AiDifficulty>().is_err());
    }
    
    #[test]
    fn test_ai_difficulty_conversion_to_legacy() {
        assert_eq!(LegacyDifficulty::from(AiDifficulty::Easy), LegacyDifficulty::Beginner);
        assert_eq!(LegacyDifficulty::from(AiDifficulty::Medium), LegacyDifficulty::Intermediate);
        assert_eq!(LegacyDifficulty::from(AiDifficulty::Hard), LegacyDifficulty::Advanced);
    }
    
    #[test]
    fn test_ai_difficulty_conversion_from_legacy() {
        assert_eq!(AiDifficulty::from(LegacyDifficulty::Beginner), AiDifficulty::Easy);
        assert_eq!(AiDifficulty::from(LegacyDifficulty::Intermediate), AiDifficulty::Medium);
        assert_eq!(AiDifficulty::from(LegacyDifficulty::Advanced), AiDifficulty::Hard);
    }
    
    #[test]
    fn test_validate_position_valid() {
        assert!(validate_position(0, 0).is_ok());
        assert!(validate_position(7, 7).is_ok());
        assert!(validate_position(3, 4).is_ok());
    }
    
    #[test]
    fn test_validate_position_invalid() {
        assert!(validate_position(8, 0).is_err());
        assert!(validate_position(0, 8).is_err());
        assert!(validate_position(10, 10).is_err());
    }
    
    #[test]
    fn test_move_record_creation() {
        let position = Position::new(3, 4).unwrap();
        let move_record = MoveRecord::new(Player::Black, position, Some(1500));
        
        assert_eq!(move_record.player, Player::Black);
        assert_eq!(move_record.position, position);
        assert_eq!(move_record.thinking_time_ms, Some(1500));
    }
    
    #[test]
    fn test_ai_battle_session_creation() {
        let session = AiBattleSession::new(AiDifficulty::Easy);
        
        assert_eq!(session.ai_difficulty, AiDifficulty::Easy);
        assert_eq!(session.current_player, Player::Black);
        assert!(!session.ai_thinking);
        assert!(!session.is_finished());
        assert!(session.is_player_turn());
        assert!(!session.is_ai_turn());
        assert_eq!(session.move_history.len(), 0);
    }
    
    #[test]
    fn test_game_status() {
        let in_progress = GameStatus::InProgress;
        let finished_black = GameStatus::Finished { winner: Some(Player::Black) };
        let finished_tie = GameStatus::Finished { winner: None };
        
        assert_ne!(in_progress, finished_black);
        assert_ne!(finished_black, finished_tie);
    }
    
    #[test]
    fn test_ai_battle_response_from_session() {
        let session = AiBattleSession::new(AiDifficulty::Medium);
        let response = AiBattleResponse::from_session(&session);
        
        assert_eq!(response.game_id, session.id);
        assert_eq!(response.ai_difficulty, AiDifficulty::Medium);
        assert_eq!(response.current_player, Player::Black);
        assert!(!response.ai_thinking);
        assert_eq!(response.board.len(), 8);
        assert_eq!(response.board[0].len(), 8);
        assert!(response.valid_moves.len() > 0);
    }
    
    #[test]
    fn test_session_summary_from_session() {
        let session = AiBattleSession::new(AiDifficulty::Hard);
        let summary = SessionSummary::from_session(&session);
        
        assert_eq!(summary.game_id, session.id);
        assert_eq!(summary.ai_difficulty, AiDifficulty::Hard);
        assert_eq!(summary.move_count, 0);
        assert_eq!(summary.status, GameStatus::InProgress);
    }
    
    #[test]
    fn test_difficulties_response() {
        let response = DifficultiesResponse::new();
        
        assert_eq!(response.difficulties.len(), 3);
        assert!(response.difficulties.iter().any(|d| matches!(d.id, AiDifficulty::Easy)));
        assert!(response.difficulties.iter().any(|d| matches!(d.id, AiDifficulty::Medium)));
        assert!(response.difficulties.iter().any(|d| matches!(d.id, AiDifficulty::Hard)));
    }
    
    #[test]
    fn test_difficulty_info_conversion() {
        let info = DifficultyInfo::from(AiDifficulty::Easy);
        
        assert_eq!(info.id, AiDifficulty::Easy);
        assert_eq!(info.name, "Easy");
        assert!(info.description.contains("初級"));
    }
    
    #[test]
    fn test_error_response_creation() {
        let error = ErrorResponse::new("TestError", "Test message");
        
        assert_eq!(error.error, "TestError");
        assert_eq!(error.message, "Test message");
        assert!(error.error_code.is_none());
    }
    
    #[test]
    fn test_error_response_with_code() {
        let error = ErrorResponse::with_code("TestError", "Test message", "TEST_CODE");
        
        assert_eq!(error.error, "TestError");
        assert_eq!(error.message, "Test message");
        assert_eq!(error.error_code, Some("TEST_CODE".to_string()));
    }
    
    #[test]
    fn test_ai_battle_error_codes() {
        let game_id = Uuid::new_v4();
        let error = AiBattleError::GameNotFound { game_id };
        assert_eq!(error.error_code(), "GAME_NOT_FOUND");
        
        let error = AiBattleError::InvalidMove { reason: "test".to_string() };
        assert_eq!(error.error_code(), "INVALID_MOVE");
        
        let error = AiBattleError::NotPlayerTurn;
        assert_eq!(error.error_code(), "NOT_PLAYER_TURN");
        
        let error = AiBattleError::MaxSessionsReached { max: 10 };
        assert_eq!(error.error_code(), "MAX_SESSIONS_REACHED");
    }
    
    #[test]
    fn test_ai_battle_error_status_codes() {
        let game_id = Uuid::new_v4();
        let error = AiBattleError::GameNotFound { game_id };
        assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
        
        let error = AiBattleError::InvalidMove { reason: "test".to_string() };
        assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        
        let error = AiBattleError::NotPlayerTurn;
        assert_eq!(error.status_code(), StatusCode::FORBIDDEN);
        
        let error = AiBattleError::MaxSessionsReached { max: 10 };
        assert_eq!(error.status_code(), StatusCode::TOO_MANY_REQUESTS);
        
        let error = AiBattleError::AiThinkingError { details: "test".to_string() };
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }
    
    #[test]
    fn test_ai_battle_error_http_conversion() {
        let game_id = Uuid::new_v4();
        let error = AiBattleError::GameNotFound { game_id };
        let (status, json_response): (StatusCode, Json<ErrorResponse>) = error.into();
        
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json_response.error, "GAME_NOT_FOUND");
        assert_eq!(json_response.error_code, Some("GAME_NOT_FOUND".to_string()));
    }
    
    #[test]
    fn test_ai_battle_result_type() {
        let success_result: AiBattleResult<i32> = Ok(42);
        assert!(success_result.is_ok());
        assert_eq!(success_result.unwrap(), 42);
        
        let error_result: AiBattleResult<i32> = Err(AiBattleError::NotPlayerTurn);
        assert!(error_result.is_err());
    }
}