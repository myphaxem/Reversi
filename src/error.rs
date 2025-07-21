//! アプリケーション全体のエラー定義モジュール
//! ゲームロジック、AIサービス、永続化などのエラーを統一管理。

use thiserror::Error;
use uuid::Uuid;

/// ゲームロジックに関連するエラー
#[derive(Debug, Error)]
pub enum GameError {
    #[error("Invalid move: {reason}")]
    InvalidMove { reason: String },
    
    #[error("Game not found: {game_id}")]
    GameNotFound { game_id: Uuid },
    
    #[error("Game already finished")]
    GameFinished,
    
    #[error("AI calculation failed: {source}")]
    AIError { 
        #[from]
        source: AIError 
    },
    
    #[error("Persistence error: {source}")]
    PersistenceError { 
        #[from]
        source: PersistenceError 
    },
    
    #[error("Session limit exceeded")]
    SessionLimitExceeded,
}

/// AIサービスに関連するエラー
#[derive(Debug, Error)]
pub enum AIError {
    #[error("AI calculation timeout")]
    Timeout,
    
    #[error("No valid moves available")]
    NoValidMoves,
    
    #[error("AI strategy error: {message}")]
    StrategyError { message: String },
    
    #[error("AI service unavailable: {service_name} - {reason}")]
    ServiceUnavailable { 
        service_name: String, 
        reason: String 
    },
    
    #[error("AI service configuration error: {message}")]
    ConfigurationError { message: String },
}

/// データ永続化に関連するエラー
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("Database error: {message}")]
    DatabaseError { message: String },
    
    #[error("File I/O error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
    
    #[error("Serialization error: {message}")]
    SerializationError { message: String },
}

/// ゲームエラーをベースとした結果型
pub type Result<T> = std::result::Result<T, GameError>;