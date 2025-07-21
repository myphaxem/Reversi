use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    game::{GameState, Position, Player, ReversiRules},
    ai::{Difficulty},
    error::GameError,
    api::ai_battle::service::AiBattleService,
    session::AiBattleSessionManager,
};

#[derive(Debug, Serialize)]
pub struct GameResponse {
    pub id: Uuid,
    pub board: [[u8; 8]; 8],        // 0: Empty, 1: Black, 2: White
    pub current_player: u8,          // 1: Black, 2: White
    pub valid_moves: Vec<[usize; 2]>,
    pub game_status: String,
    pub score: (u8, u8),
    pub move_count: u32,
}

#[derive(Debug, Serialize)]
pub struct MoveResponse {
    pub success: bool,
    pub game_state: GameResponse,
    pub flipped_positions: Vec<[usize; 2]>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGameRequest {
    pub player1_type: PlayerTypeRequest,
    pub player2_type: PlayerTypeRequest,
}

#[derive(Debug, Deserialize)]
pub enum PlayerTypeRequest {
    Human { name: String },
    AI { difficulty: Difficulty },
}

#[derive(Debug, Deserialize)]
pub struct MakeMoveRequest {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug)]
pub struct AppState {
    pub games: Arc<RwLock<std::collections::HashMap<Uuid, GameState>>>,
    pub ai_battle_service: Arc<AiBattleService>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            games: Arc::clone(&self.games),
            ai_battle_service: Arc::clone(&self.ai_battle_service),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        let session_manager = Arc::new(AiBattleSessionManager::new(100));
        let ai_battle_service = Arc::new(AiBattleService::new(session_manager));
        
        Self {
            games: Arc::new(RwLock::new(std::collections::HashMap::new())),
            ai_battle_service,
        }
    }
    
    pub fn new_with_configurable_service(configurable_service: Arc<crate::api::ai_battle::ConfigurableAiBattleService>) -> Self {
        Self {
            games: Arc::new(RwLock::new(std::collections::HashMap::new())),
            ai_battle_service: Arc::clone(configurable_service.get_service()),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameResponse {
    pub fn from_game_state(game_state: &GameState) -> Self {
        let mut board = [[0u8; 8]; 8];
        for row in 0..8 {
            for col in 0..8 {
                if let Some(position) = Position::new(row, col) {
                    if let Some(cell) = game_state.board.get_cell(position) {
                        board[row][col] = match cell {
                            crate::game::Cell::Empty => 0,
                            crate::game::Cell::Black => 1,
                            crate::game::Cell::White => 2,
                        };
                    }
                }
            }
        }

        let valid_moves = ReversiRules::get_valid_moves(&game_state.board, game_state.current_player)
            .into_iter()
            .map(|pos| [pos.row, pos.col])
            .collect();

        let game_status = match &game_state.game_status {
            crate::game::GameStatus::InProgress => "in_progress".to_string(),
            crate::game::GameStatus::Paused => "paused".to_string(),
            crate::game::GameStatus::Finished { winner, .. } => {
                match winner {
                    Some(Player::Black) => "finished_black_wins",
                    Some(Player::White) => "finished_white_wins",
                    None => "finished_tie",
                }.to_string()
            }
        };

        let score = game_state.get_score();

        Self {
            id: game_state.id,
            board,
            current_player: match game_state.current_player {
                Player::Black => 1,
                Player::White => 2,
            },
            valid_moves,
            game_status,
            score,
            move_count: game_state.get_move_count() as u32,
        }
    }
}

pub async fn create_game(
    State(state): State<AppState>,
    Json(_payload): Json<CreateGameRequest>,
) -> std::result::Result<Json<GameResponse>, (StatusCode, Json<ErrorResponse>)> {
    let game_state = GameState::new();
    let game_id = game_state.id;
    
    {
        let mut games = state.games.write().await;
        games.insert(game_id, game_state.clone());
    }

    let response = GameResponse::from_game_state(&game_state);
    Ok(Json(response))
}

pub async fn get_game(
    State(state): State<AppState>,
    Path(game_id): Path<Uuid>,
) -> std::result::Result<Json<GameResponse>, (StatusCode, Json<ErrorResponse>)> {
    let games = state.games.read().await;
    
    match games.get(&game_id) {
        Some(game_state) => {
            let response = GameResponse::from_game_state(game_state);
            Ok(Json(response))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Game not found".to_string(),
                details: Some(format!("No game with ID {}", game_id)),
            }),
        )),
    }
}

pub async fn make_move(
    State(state): State<AppState>,
    Path(game_id): Path<Uuid>,
    Json(payload): Json<MakeMoveRequest>,
) -> std::result::Result<Json<MoveResponse>, (StatusCode, Json<ErrorResponse>)> {
    let position = match Position::new(payload.row, payload.col) {
        Some(pos) => pos,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invalid position".to_string(),
                    details: Some(format!("Position ({}, {}) is out of bounds", payload.row, payload.col)),
                }),
            ));
        }
    };

    let mut games = state.games.write().await;
    
    match games.get_mut(&game_id) {
        Some(game_state) => {
            match ReversiRules::apply_move(game_state, position) {
                Ok(flipped_positions) => {
                    game_state.switch_player();
                    
                    ReversiRules::handle_turn(game_state);

                    let flipped: Vec<[usize; 2]> = flipped_positions
                        .into_iter()
                        .map(|pos| [pos.row, pos.col])
                        .collect();

                    let response = MoveResponse {
                        success: true,
                        game_state: GameResponse::from_game_state(game_state),
                        flipped_positions: flipped,
                        message: None,
                    };
                    
                    Ok(Json(response))
                }
                Err(e) => {
                    let error_msg = match e {
                        GameError::InvalidMove { reason } => reason,
                        GameError::GameFinished => "Game is already finished".to_string(),
                        _ => "Move failed".to_string(),
                    };
                    
                    Err((
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: error_msg,
                            details: None,
                        }),
                    ))
                }
            }
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Game not found".to_string(),
                details: Some(format!("No game with ID {}", game_id)),
            }),
        )),
    }
}

pub async fn delete_game(
    State(state): State<AppState>,
    Path(game_id): Path<Uuid>,
) -> std::result::Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let mut games = state.games.write().await;
    
    match games.remove(&game_id) {
        Some(_) => Ok(StatusCode::NO_CONTENT),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Game not found".to_string(),
                details: Some(format!("No game with ID {}", game_id)),
            }),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_response_conversion() {
        let game_state = GameState::new();
        let response = GameResponse::from_game_state(&game_state);

        assert_eq!(response.id, game_state.id);
        assert_eq!(response.current_player, 1); // Black
        assert_eq!(response.score, (2, 2));
        assert_eq!(response.game_status, "in_progress");
        assert_eq!(response.valid_moves.len(), 4); // Initial valid moves
    }

    #[test]
    fn test_app_state_creation() {
        let state = AppState::new();
        assert!(true);
    }
}