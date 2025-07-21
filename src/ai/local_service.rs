
use async_trait::async_trait;
use std::time::Instant;
use tokio::time::{sleep, Duration};

use crate::api::ai_battle::dto::AiDifficulty;
use crate::error::AIError;
use crate::game::{GameState, ReversiRules};

use super::service::{AIService, AIMoveResult, AIServiceType};
use super::strategies::{AIStrategy, create_ai_strategy, Difficulty as LegacyDifficulty};

#[derive(Debug, Clone)]
pub struct LocalAIService {
    pub simulate_thinking_time: bool,
    pub min_thinking_time_ms: u64,
    pub max_thinking_time_ms: u64,
}

impl LocalAIService {
    pub fn new() -> Self {
        Self {
            simulate_thinking_time: true,
            min_thinking_time_ms: 300,
            max_thinking_time_ms: 3000,
        }
    }
    
    pub fn new_fast() -> Self {
        Self {
            simulate_thinking_time: false,
            min_thinking_time_ms: 0,
            max_thinking_time_ms: 0,
        }
    }
    
    fn get_thinking_time(&self, difficulty: AiDifficulty) -> u64 {
        if !self.simulate_thinking_time {
            return 0;
        }
        
        match difficulty {
            AiDifficulty::Easy => 500,
            AiDifficulty::Medium => 1500,
            AiDifficulty::Hard => 3000,
        }
    }
    
    fn convert_difficulty(difficulty: AiDifficulty) -> LegacyDifficulty {
        match difficulty {
            AiDifficulty::Easy => LegacyDifficulty::Beginner,
            AiDifficulty::Medium => LegacyDifficulty::Intermediate,
            AiDifficulty::Hard => LegacyDifficulty::Advanced,
        }
    }
}

impl Default for LocalAIService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AIService for LocalAIService {
    async fn calculate_move(
        &self, 
        game_state: &GameState, 
        difficulty: AiDifficulty
    ) -> Result<AIMoveResult, AIError> {
        let start_time = Instant::now();
        
        if game_state.is_finished() {
            return Err(AIError::StrategyError {
                message: "Cannot calculate move for finished game".to_string(),
            });
        }
        
        let valid_moves = ReversiRules::get_valid_moves(&game_state.board, game_state.current_player);
        if valid_moves.is_empty() {
            return Err(AIError::NoValidMoves);
        }
        
        let thinking_time_ms = self.get_thinking_time(difficulty);
        if thinking_time_ms > 0 {
            sleep(Duration::from_millis(thinking_time_ms)).await;
        }
        
        let legacy_difficulty = Self::convert_difficulty(difficulty);
        let ai_strategy = create_ai_strategy(legacy_difficulty);
        
        let position = ai_strategy.calculate_move(game_state)?;
        
        let actual_thinking_time = start_time.elapsed().as_millis() as u64;
        
        Ok(AIMoveResult {
            position,
            thinking_time_ms: actual_thinking_time,
            evaluation_score: None,
            depth_reached: None,
            nodes_evaluated: None,
        })
    }
    
    async fn is_available(&self) -> bool {
        true
    }
    
    fn get_supported_difficulties(&self) -> Vec<AiDifficulty> {
        vec![AiDifficulty::Easy]
    }
    
    fn get_name(&self) -> &'static str {
        "LocalAIService"
    }
    
    fn get_service_type(&self) -> AIServiceType {
        AIServiceType::Local
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::GameState;
    
    #[tokio::test]
    async fn test_local_ai_service_creation() {
        let service = LocalAIService::new();
        assert_eq!(service.get_name(), "LocalAIService");
        assert_eq!(service.get_service_type(), AIServiceType::Local);
        assert!(service.is_available().await);
    }
    
    #[tokio::test]
    async fn test_local_ai_service_fast() {
        let service = LocalAIService::new_fast();
        assert!(!service.simulate_thinking_time);
        assert_eq!(service.min_thinking_time_ms, 0);
        assert_eq!(service.max_thinking_time_ms, 0);
    }
    
    #[tokio::test]
    async fn test_supported_difficulties() {
        let service = LocalAIService::new();
        let difficulties = service.get_supported_difficulties();
        assert!(difficulties.contains(&AiDifficulty::Easy));
    }
    
    #[tokio::test]
    async fn test_calculate_move() {
        let service = LocalAIService::new_fast();
        let game_state = GameState::new();
        
        let result = service.calculate_move(&game_state, AiDifficulty::Easy).await;
        assert!(result.is_ok());
        
        let move_result = result.unwrap();
        assert!(move_result.thinking_time_ms >= 0);
        
        let valid_moves = ReversiRules::get_valid_moves(&game_state.board, game_state.current_player);
        assert!(valid_moves.contains(&move_result.position));
    }
    
    #[tokio::test]
    async fn test_calculate_move_finished_game() {
        let service = LocalAIService::new_fast();
        let mut game_state = GameState::new();
        
        game_state.game_status = crate::game::GameStatus::Finished { 
            winner: None, 
            final_score: (32, 32) 
        };
        
        let result = service.calculate_move(&game_state, AiDifficulty::Easy).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AIError::StrategyError { .. }));
    }
    
    #[test]
    fn test_difficulty_conversion() {
        assert_eq!(LocalAIService::convert_difficulty(AiDifficulty::Easy), LegacyDifficulty::Beginner);
        assert_eq!(LocalAIService::convert_difficulty(AiDifficulty::Medium), LegacyDifficulty::Intermediate);
        assert_eq!(LocalAIService::convert_difficulty(AiDifficulty::Hard), LegacyDifficulty::Advanced);
    }
    
    #[test]
    fn test_thinking_time() {
        let service = LocalAIService::new();
        
        assert_eq!(service.get_thinking_time(AiDifficulty::Easy), 500);
        assert_eq!(service.get_thinking_time(AiDifficulty::Medium), 1500);
        assert_eq!(service.get_thinking_time(AiDifficulty::Hard), 3000);
        
        let fast_service = LocalAIService::new_fast();
        assert_eq!(fast_service.get_thinking_time(AiDifficulty::Easy), 0);
        assert_eq!(fast_service.get_thinking_time(AiDifficulty::Medium), 0);
        assert_eq!(fast_service.get_thinking_time(AiDifficulty::Hard), 0);
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let service = LocalAIService::new();
        let status = service.health_check().await;
        
        assert!(status.is_ok());
        let status = status.unwrap();
        assert_eq!(status.name, "LocalAIService");
        assert!(status.available);
        assert!(status.average_response_time_ms.is_some());
    }
}