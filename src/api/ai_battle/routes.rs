//! AI対戦APIルート

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

use super::handlers;
use super::service::AiBattleService;

pub fn create_ai_battle_routes(service: Arc<AiBattleService>) -> Router {
    Router::new()
        .route("/api/ai-battle", post(handlers::create_ai_battle))
        .route("/api/ai-battle/difficulties", get(handlers::get_difficulties))
        .route("/api/ai-battle/sessions", get(handlers::get_sessions))
        
        .route("/api/ai-battle/:game_id", get(handlers::get_game_state))
        .route("/api/ai-battle/:game_id", delete(handlers::delete_game))
        .route("/api/ai-battle/:game_id/move", post(handlers::execute_move))
        .route("/api/ai-battle/:game_id/difficulty", put(handlers::change_difficulty))
        .route("/api/ai-battle/:game_id/history", get(handlers::get_history))
        
        .with_state(service)
}