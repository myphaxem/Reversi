use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use tower::util::ServiceExt;

use super::{
    handlers::{create_game, delete_game, get_game, make_move, AppState},
    middleware::{cors, logging},
    ai_battle::routes::create_ai_battle_routes,
};

pub fn create_router() -> Router<AppState> {
    let base_routes = Router::new()
        .route("/api/games", post(create_game))
        .route("/api/games/:id", get(get_game))
        .route("/api/games/:id/move", put(make_move))
        .route("/api/games/:id", delete(delete_game))
        
        .route("/health", get(health_check));
    
    base_routes
        .layer(middleware::from_fn(cors))
        .layer(middleware::from_fn(logging))
}

pub fn create_ai_battle_router(app_state: AppState) -> Router {
    create_ai_battle_routes(app_state.ai_battle_service)
        .layer(middleware::from_fn(cors))
        .layer(middleware::from_fn(logging))
}

async fn health_check() -> &'static str {
    "Reversi API Server is running"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let router = create_router();
        assert!(true);
    }
}