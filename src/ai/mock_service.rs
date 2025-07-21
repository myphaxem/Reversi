
use async_trait::async_trait;
use std::time::Instant;
use tokio::time::{sleep, Duration};

use crate::api::ai_battle::dto::AiDifficulty;
use crate::error::AIError;
use crate::game::{GameState, ReversiRules, Position};

use super::service::{AIService, AIMoveResult, AIServiceType};

#[derive(Debug, Clone)]
pub struct MockAIConfig {
    pub available: bool,
    pub response_time_ms: u64,
    pub should_error: bool,
    pub error_message: String,
    pub fixed_move: Option<Position>,
    pub supported_difficulties: Vec<AiDifficulty>,
}

impl Default for MockAIConfig {
    fn default() -> Self {
        Self {
            available: true,
            response_time_ms: 100,
            should_error: false,
            error_message: "Mock AI error".to_string(),
            fixed_move: None,
            supported_difficulties: vec![AiDifficulty::Easy, AiDifficulty::Medium, AiDifficulty::Hard],
        }
    }
}

#[derive(Debug, Clone)]
pub struct MockAIService {
    config: MockAIConfig,
}

impl MockAIService {
    pub fn new(config: MockAIConfig) -> Self {
        Self { config }
    }
    
    pub fn new_default() -> Self {
        Self::new(MockAIConfig::default())
    }
    
    pub fn new_unavailable() -> Self {
        Self::new(MockAIConfig {
            available: false,
            ..MockAIConfig::default()
        })
    }
    
    pub fn new_error(error_message: impl Into<String>) -> Self {
        Self::new(MockAIConfig {
            should_error: true,
            error_message: error_message.into(),
            ..MockAIConfig::default()
        })
    }
    
    pub fn new_with_fixed_move(position: Position) -> Self {
        Self::new(MockAIConfig {
            fixed_move: Some(position),
            response_time_ms: 0,
            ..MockAIConfig::default()
        })
    }
    
    pub fn new_fast() -> Self {
        Self::new(MockAIConfig {
            response_time_ms: 0,
            ..MockAIConfig::default()
        })
    }
    
    pub fn update_config(&mut self, config: MockAIConfig) {
        self.config = config;
    }
    
    pub fn get_config(&self) -> &MockAIConfig {
        &self.config
    }
}

#[async_trait]
impl AIService for MockAIService {
    async fn calculate_move(
        &self, 
        game_state: &GameState, 
        difficulty: AiDifficulty
    ) -> Result<AIMoveResult, AIError> {
        let start_time = Instant::now();
        
        if !self.config.available {
            return Err(AIError::ServiceUnavailable {
                service_name: self.get_name().to_string(),
                reason: "Mock AI service is configured as unavailable".to_string(),
            });
        }
        
        if self.config.should_error {
            return Err(AIError::StrategyError {
                message: self.config.error_message.clone(),
            });
        }
        
        if !self.config.supported_difficulties.contains(&difficulty) {
            return Err(AIError::StrategyError {
                message: format!("Difficulty {:?} is not supported by mock AI", difficulty),
            });
        }
        
        if game_state.is_finished() {
            return Err(AIError::StrategyError {
                message: "Cannot calculate move for finished game".to_string(),
            });
        }
        
        if self.config.response_time_ms > 0 {
            sleep(Duration::from_millis(self.config.response_time_ms)).await;
        }
        
        let position = if let Some(fixed_move) = self.config.fixed_move {
            let valid_moves = ReversiRules::get_valid_moves(&game_state.board, game_state.current_player);
            if valid_moves.contains(&fixed_move) {
                fixed_move
            } else {
                valid_moves.first().copied()
                    .ok_or(AIError::NoValidMoves)?
            }
        } else {
            let valid_moves = ReversiRules::get_valid_moves(&game_state.board, game_state.current_player);
            if valid_moves.is_empty() {
                return Err(AIError::NoValidMoves);
            }
            
            valid_moves[0]
        };
        
        let actual_thinking_time = start_time.elapsed().as_millis() as u64;
        
        let evaluation_score = match difficulty {
            AiDifficulty::Easy => Some(0.1),
            AiDifficulty::Medium => Some(0.5),
            AiDifficulty::Hard => Some(0.9),
        };
        
        let depth_reached = match difficulty {
            AiDifficulty::Easy => Some(1),
            AiDifficulty::Medium => Some(3),
            AiDifficulty::Hard => Some(6),
        };
        
        let nodes_evaluated = match difficulty {
            AiDifficulty::Easy => Some(10),
            AiDifficulty::Medium => Some(100),
            AiDifficulty::Hard => Some(1000),
        };
        
        Ok(AIMoveResult {
            position,
            thinking_time_ms: actual_thinking_time,
            evaluation_score,
            depth_reached,
            nodes_evaluated,
        })
    }
    
    async fn is_available(&self) -> bool {
        self.config.available
    }
    
    fn get_supported_difficulties(&self) -> Vec<AiDifficulty> {
        self.config.supported_difficulties.clone()
    }
    
    fn get_name(&self) -> &'static str {
        "MockAIService"
    }
    
    fn get_service_type(&self) -> AIServiceType {
        AIServiceType::Mock
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{GameState, Position};
    
    #[tokio::test]
    async fn test_mock_ai_service_default() {
        let service = MockAIService::new_default();
        assert_eq!(service.get_name(), "MockAIService");
        assert_eq!(service.get_service_type(), AIServiceType::Mock);
        assert!(service.is_available().await);
    }
    
    #[tokio::test]
    async fn test_mock_ai_service_unavailable() {
        let service = MockAIService::new_unavailable();
        assert!(!service.is_available().await);
        
        let game_state = GameState::new();
        let result = service.calculate_move(&game_state, AiDifficulty::Easy).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AIError::ServiceUnavailable { .. }));
    }
    
    #[tokio::test]
    async fn test_mock_ai_service_error() {
        let service = MockAIService::new_error("Test error");
        
        let game_state = GameState::new();
        let result = service.calculate_move(&game_state, AiDifficulty::Easy).await;
        assert!(result.is_err());
        
        if let Err(AIError::StrategyError { message }) = result {
            assert_eq!(message, "Test error");
        } else {
            panic!("Expected StrategyError");
        }
    }
    
    #[tokio::test]
    async fn test_mock_ai_service_fixed_move() {
        let fixed_position = Position::new(2, 3).unwrap();
        let service = MockAIService::new_with_fixed_move(fixed_position);
        
        let game_state = GameState::new();
        let result = service.calculate_move(&game_state, AiDifficulty::Easy).await;
        assert!(result.is_ok());
        
        let move_result = result.unwrap();
        
        let valid_moves = ReversiRules::get_valid_moves(&game_state.board, game_state.current_player);
        if valid_moves.contains(&fixed_position) {
            assert_eq!(move_result.position, fixed_position);
        }
    }
    
    #[tokio::test]
    async fn test_mock_ai_service_fast() {
        let service = MockAIService::new_fast();
        
        let game_state = GameState::new();
        let start = Instant::now();
        let result = service.calculate_move(&game_state, AiDifficulty::Easy).await;
        let elapsed = start.elapsed();
        
        assert!(result.is_ok());
        assert!(elapsed.as_millis() < 50);
    }
    
    #[tokio::test]
    async fn test_supported_difficulties() {
        let service = MockAIService::new_default();
        let difficulties = service.get_supported_difficulties();
        
        assert!(difficulties.contains(&AiDifficulty::Easy));
        assert!(difficulties.contains(&AiDifficulty::Medium));
        assert!(difficulties.contains(&AiDifficulty::Hard));
    }
    
    #[tokio::test]
    async fn test_evaluation_scores_by_difficulty() {
        let service = MockAIService::new_fast();
        let game_state = GameState::new();
        
        for difficulty in [AiDifficulty::Easy, AiDifficulty::Medium, AiDifficulty::Hard] {
            let result = service.calculate_move(&game_state, difficulty).await;
            assert!(result.is_ok());
            
            let move_result = result.unwrap();
            assert!(move_result.evaluation_score.is_some());
            assert!(move_result.depth_reached.is_some());
            assert!(move_result.nodes_evaluated.is_some());
        }
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let service = MockAIService::new_default();
        let status = service.health_check().await;
        
        assert!(status.is_ok());
        let status = status.unwrap();
        assert_eq!(status.name, "MockAIService");
        assert!(status.available);
        assert!(status.average_response_time_ms.is_some());
    }
    
    #[tokio::test]
    async fn test_config_update() {
        let mut service = MockAIService::new_default();
        assert!(service.is_available().await);
        
        service.update_config(MockAIConfig {
            available: false,
            ..MockAIConfig::default()
        });
        
        assert!(!service.is_available().await);
    }
}