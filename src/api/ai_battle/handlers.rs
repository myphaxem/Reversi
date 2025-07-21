//! AI対戦APIハンドラー

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use uuid::Uuid;

use super::dto::{
    AiBattleError, AiBattleResponse, CreateAiBattleRequest, 
    DifficultiesResponse, ErrorResponse, PlayerMoveRequest,
    MoveResponse, ChangeDifficultyRequest, validate_position,
    MoveHistoryResponse, SessionListResponse, SessionSummary
};
use super::service::AiBattleService;

pub async fn create_ai_battle(
    State(service): State<Arc<AiBattleService>>,
    Json(request): Json<CreateAiBattleRequest>,
) -> Result<(StatusCode, Json<AiBattleResponse>), (StatusCode, Json<ErrorResponse>)> {
    match service.create_ai_battle(request.difficulty).await {
        Ok(response) => Ok((StatusCode::CREATED, Json(response))),
        Err(err) => Err(err.into()),
    }
}

pub async fn get_game_state(
    State(service): State<Arc<AiBattleService>>,
    Path(game_id): Path<Uuid>,
) -> Result<Json<AiBattleResponse>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_game_state(game_id) {
        Ok(response) => Ok(Json(response)),
        Err(err) => Err(err.into()),
    }
}

pub async fn get_difficulties() -> Json<DifficultiesResponse> {
    Json(DifficultiesResponse::new())
}

pub async fn execute_move(
    State(service): State<Arc<AiBattleService>>,
    Path(game_id): Path<Uuid>,
    Json(request): Json<PlayerMoveRequest>,
) -> Result<Json<MoveResponse>, (StatusCode, Json<ErrorResponse>)> {
    let position = match validate_position(request.row, request.col) {
        Ok(pos) => pos,
        Err(error_msg) => {
            let error = ErrorResponse::with_code(
                "INVALID_POSITION",
                error_msg,
                "INVALID_POSITION"
            );
            return Err((StatusCode::BAD_REQUEST, Json(error)));
        }
    };
    
    match service.make_player_move(game_id, position).await {
        Ok(response) => Ok(Json(response)),
        Err(err) => Err(err.into()),
    }
}

pub async fn change_difficulty(
    State(service): State<Arc<AiBattleService>>,
    Path(game_id): Path<Uuid>,
    Json(request): Json<ChangeDifficultyRequest>,
) -> Result<Json<AiBattleResponse>, (StatusCode, Json<ErrorResponse>)> {
    match service.change_difficulty(game_id, request.difficulty) {
        Ok(response) => Ok(Json(response)),
        Err(err) => Err(err.into()),
    }
}

pub async fn delete_game(
    State(service): State<Arc<AiBattleService>>,
    Path(game_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match service.delete_session(game_id) {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(err) => Err(err.into()),
    }
}

pub async fn get_history(
    State(service): State<Arc<AiBattleService>>,
    Path(game_id): Path<Uuid>,
) -> Result<Json<MoveHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_move_history(game_id) {
        Ok(moves) => {
            let response = MoveHistoryResponse {
                game_id,
                moves: moves.clone(),
                total_moves: moves.len(),
            };
            Ok(Json(response))
        },
        Err(err) => Err(err.into()),
    }
}

pub async fn get_sessions(
    State(service): State<Arc<AiBattleService>>,
) -> Json<SessionListResponse> {
    let sessions = service.list_sessions();
    let session_summaries: Vec<SessionSummary> = sessions
        .iter()
        .map(SessionSummary::from_session)
        .collect();
    
    let response = SessionListResponse {
        sessions: session_summaries,
        total_count: sessions.len(),
    };
    
    Json(response)
}