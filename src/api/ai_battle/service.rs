//! AI対戦サービス

use std::sync::Arc;
use tokio::time::{sleep, Duration};
use chrono::Utc;

use crate::game::{Position, Player, ReversiRules};
use crate::ai::service::{AIService, AIServiceFactory};
use crate::session::AiBattleSessionManager;

use super::dto::{
    AiBattleSession, AiBattleError, AiBattleResult, AiDifficulty, 
    MoveRecord, GameStatus, AiBattleResponse, MoveResponse
};

pub struct AiBattleService {
    session_manager: Arc<AiBattleSessionManager>,
    ai_service: Arc<dyn AIService>,
}

impl std::fmt::Debug for AiBattleService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AiBattleService")
            .field("session_manager", &self.session_manager)
            .field("ai_service", &format!("{}", self.ai_service.get_name()))
            .finish()
    }
}

impl AiBattleService {
    pub fn new(session_manager: Arc<AiBattleSessionManager>) -> Self {
        let ai_service = AIServiceFactory::create_default_local()
            .expect("Failed to create default local AI service");
        
        Self {
            session_manager,
            ai_service: ai_service.into(),
        }
    }
    
    pub fn new_with_ai_service(
        session_manager: Arc<AiBattleSessionManager>,
        ai_service: Arc<dyn AIService>
    ) -> Self {
        Self {
            session_manager,
            ai_service,
        }
    }
    
    pub fn get_ai_service(&self) -> &Arc<dyn AIService> {
        &self.ai_service
    }
    
    pub fn set_ai_service(&mut self, ai_service: Arc<dyn AIService>) {
        self.ai_service = ai_service;
    }
    
    pub async fn create_ai_battle(&self, difficulty: AiDifficulty) -> AiBattleResult<AiBattleResponse> {
        let session_id = self.session_manager.create_session(difficulty).await?;
        let session = self.session_manager.get_session(&session_id)?;
        
        Ok(AiBattleResponse::from_session(&session))
    }
    
    pub fn get_game_state(&self, session_id: uuid::Uuid) -> AiBattleResult<AiBattleResponse> {
        let session = self.session_manager.get_session(&session_id)?;
        Ok(AiBattleResponse::from_session(&session))
    }
    
    pub async fn make_player_move(
        &self, 
        session_id: uuid::Uuid, 
        position: Position
    ) -> AiBattleResult<MoveResponse> {
        let mut session = self.session_manager.get_session(&session_id)?;
        
        if session.is_finished() {
            return Err(AiBattleError::GameAlreadyFinished);
        }
        
        if !session.is_player_turn() {
            return Err(AiBattleError::NotPlayerTurn);
        }
        
        if session.ai_thinking {
            return Err(AiBattleError::AiThinkingError { 
                details: "AI is currently thinking".to_string() 
            });
        }
        
        if !ReversiRules::is_valid_move(&session.game_state.board, position, session.current_player) {
            return Err(AiBattleError::InvalidMove { 
                reason: format!("Invalid move at position {:?}", position) 
            });
        }
        
        let _flipped_positions = ReversiRules::apply_move(&mut session.game_state, position)
            .map_err(|e| AiBattleError::GameError(e))?;
        
        session.game_state.switch_player();
        
        // ゲーム終了チェック（両プレイヤーが手を打てない場合）
        if ReversiRules::is_game_over(&session.game_state.board) {
            let winner = ReversiRules::determine_winner(&session.game_state.board);
            session.game_state.finish(winner);
        }
        
        if session.game_state.is_finished() {
            let winner = if let crate::game::GameStatus::Finished { winner, .. } = &session.game_state.game_status {
                *winner
            } else {
                None
            };
            session.status = GameStatus::Finished { winner };
            session.current_player = session.game_state.current_player;
            self.session_manager.update_session(session.clone())?;
            
            return Ok(MoveResponse {
                success: true,
                game_state: AiBattleResponse::from_session(&session),
                player_move: position,
                ai_move: None,
                message: Some("Game finished".to_string()),
            });
        }
        
        session.current_player = session.game_state.current_player;
        
        if !session.is_ai_turn() {
            self.session_manager.update_session(session.clone())?;
            
            return Ok(MoveResponse {
                success: true,
                game_state: AiBattleResponse::from_session(&session),
                player_move: position,
                ai_move: None,
                message: Some(format!("Player continues, current_player: {:?}", session.current_player)),
            });
        }
        
        session.ai_thinking = true;
        self.session_manager.update_session(session.clone())?;
        
        match self.process_ai_move(&mut session).await {
            Ok(ai_position) => {
                session.ai_thinking = false;
                self.session_manager.update_session(session.clone())?;
                
                Ok(MoveResponse {
                    success: true,
                    game_state: AiBattleResponse::from_session(&session),
                    player_move: position,
                    ai_move: Some(ai_position),
                    message: None,
                })
            }
            Err(ai_error) => {
                session.ai_thinking = false;
                self.session_manager.update_session(session)?;
                Err(ai_error)
            }
        }
    }
    
    async fn process_ai_move(&self, session: &mut AiBattleSession) -> AiBattleResult<Position> {
        let ai_result = self.ai_service.calculate_move(&session.game_state, session.ai_difficulty).await
            .map_err(|e| AiBattleError::AiThinkingError { 
                details: format!("AI service error: {}", e) 
            })?;
        
        let ai_position = ai_result.position;
        
        let move_record = MoveRecord::new(
            Player::White,
            ai_position,
            Some(ai_result.thinking_time_ms),
        );
        session.add_move_record(move_record);
        
        let _flipped_positions = ReversiRules::apply_move(&mut session.game_state, ai_position)
            .map_err(|e| AiBattleError::GameError(e))?;
        
        session.game_state.switch_player();
        
        // ゲーム終了チェック（両プレイヤーが手を打てない場合）
        if ReversiRules::is_game_over(&session.game_state.board) {
            let winner = ReversiRules::determine_winner(&session.game_state.board);
            session.game_state.finish(winner);
        }
        
        if session.game_state.is_finished() {
            let winner = if let crate::game::GameStatus::Finished { winner, .. } = &session.game_state.game_status {
                *winner
            } else {
                None
            };
            session.status = GameStatus::Finished { winner };
        }
        
        session.current_player = session.game_state.current_player;
        
        Ok(ai_position)
    }
    
    pub fn get_move_history(&self, session_id: uuid::Uuid) -> AiBattleResult<Vec<MoveRecord>> {
        let session = self.session_manager.get_session(&session_id)?;
        
        let move_records: Vec<MoveRecord> = session.game_state.move_history
            .iter()
            .map(|game_move| MoveRecord::from_move(game_move, None))
            .collect();
        
        Ok(move_records)
    }
    
    pub fn list_sessions(&self) -> Vec<AiBattleSession> {
        self.session_manager.list_sessions()
    }
    
    pub fn delete_session(&self, session_id: uuid::Uuid) -> AiBattleResult<()> {
        self.session_manager.remove_session(&session_id)?;
        Ok(())
    }
    
    pub fn change_difficulty(&self, session_id: uuid::Uuid, new_difficulty: AiDifficulty) -> AiBattleResult<AiBattleResponse> {
        let mut session = self.session_manager.get_session(&session_id)?;
        
        if session.ai_thinking {
            return Err(AiBattleError::AiThinkingError { 
                details: "Cannot change difficulty while AI is thinking".to_string() 
            });
        }
        
        session.ai_difficulty = new_difficulty;
        self.session_manager.update_session(session.clone())?;
        
        Ok(AiBattleResponse::from_session(&session))
    }
    
    pub fn is_ai_thinking(&self, session_id: uuid::Uuid) -> AiBattleResult<bool> {
        self.session_manager.is_ai_thinking(&session_id)
    }
    
    pub async fn cleanup_inactive_sessions(&self) -> usize {
        self.session_manager.cleanup_inactive_sessions().await
    }
    
    pub fn get_service_stats(&self) -> ServiceStats {
        let session_stats = self.session_manager.get_stats();
        
        ServiceStats {
            total_sessions: session_stats.total_sessions,
            max_sessions: session_stats.max_sessions,
            ai_thinking_count: session_stats.ai_thinking_count,
            difficulty_distribution: session_stats.difficulty_counts,
        }
    }
}

#[derive(Debug)]
pub struct ServiceStats {
    pub total_sessions: usize,
    pub max_sessions: usize,
    pub ai_thinking_count: usize,
    pub difficulty_distribution: std::collections::HashMap<AiDifficulty, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    
    fn create_test_service() -> AiBattleService {
        let session_manager = Arc::new(AiBattleSessionManager::new(10));
        AiBattleService::new(session_manager)
    }
    
    #[tokio::test]
    async fn test_create_ai_battle() {
        let service = create_test_service();
        
        let result = service.create_ai_battle(AiDifficulty::Easy).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.ai_difficulty, AiDifficulty::Easy);
        assert_eq!(response.current_player, Player::Black);
        assert!(!response.ai_thinking);
    }
    
    #[tokio::test]
    async fn test_get_game_state() {
        let service = create_test_service();
        
        let create_result = service.create_ai_battle(AiDifficulty::Medium).await.unwrap();
        let session_id = create_result.game_id;
        
        let result = service.get_game_state(session_id);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.game_id, session_id);
        assert_eq!(response.ai_difficulty, AiDifficulty::Medium);
    }
    
    #[tokio::test]
    async fn test_get_nonexistent_game_state() {
        let service = create_test_service();
        let nonexistent_id = Uuid::new_v4();
        
        let result = service.get_game_state(nonexistent_id);
        assert!(matches!(result, Err(AiBattleError::GameNotFound { .. })));
    }
    
    #[tokio::test]
    async fn test_make_player_move_valid() {
        let service = create_test_service();
        
        let create_result = service.create_ai_battle(AiDifficulty::Easy).await.unwrap();
        let session_id = create_result.game_id;
        
        // 有効な着手位置を取得
        let valid_moves = create_result.valid_moves;
        assert!(!valid_moves.is_empty());
        
        let first_valid_move = valid_moves[0];
        let result = service.make_player_move(session_id, first_valid_move).await;
        
        assert!(result.is_ok());
        let move_response = result.unwrap();
        println!("Move response: success={}, ai_move={:?}, message={:?}", 
                 move_response.success, move_response.ai_move, move_response.message);
        assert!(move_response.success);
        assert_eq!(move_response.player_move, first_valid_move);
        assert!(move_response.ai_move.is_some());
    }
    
    #[tokio::test]
    async fn test_make_player_move_invalid_position() {
        let service = create_test_service();
        
        let create_result = service.create_ai_battle(AiDifficulty::Easy).await.unwrap();
        let session_id = create_result.game_id;
        
        // 無効な位置で着手を試行
        let invalid_position = Position::new(0, 0).unwrap(); // 初期状態では通常無効
        let result = service.make_player_move(session_id, invalid_position).await;
        
        assert!(matches!(result, Err(AiBattleError::InvalidMove { .. })));
    }
    
    #[tokio::test]
    async fn test_make_player_move_nonexistent_session() {
        let service = create_test_service();
        let nonexistent_id = Uuid::new_v4();
        let position = Position::new(2, 3).unwrap();
        
        let result = service.make_player_move(nonexistent_id, position).await;
        assert!(matches!(result, Err(AiBattleError::GameNotFound { .. })));
    }
    
    #[tokio::test]
    async fn test_get_move_history() {
        let service = create_test_service();
        
        let create_result = service.create_ai_battle(AiDifficulty::Easy).await.unwrap();
        let session_id = create_result.game_id;
        
        // 初期状態では履歴は空
        let history = service.get_move_history(session_id).unwrap();
        assert_eq!(history.len(), 0);
        
        // プレイヤー着手後
        let valid_moves = create_result.valid_moves;
        let first_valid_move = valid_moves[0];
        let _move_result = service.make_player_move(session_id, first_valid_move).await.unwrap();
        
        let history = service.get_move_history(session_id).unwrap();
        assert_eq!(history.len(), 2); // プレイヤー + AI
    }
    
    #[tokio::test]
    async fn test_list_sessions() {
        let service = create_test_service();
        
        // 初期状態では空
        let sessions = service.list_sessions();
        assert_eq!(sessions.len(), 0);
        
        // セッション作成後
        let _result1 = service.create_ai_battle(AiDifficulty::Easy).await.unwrap();
        let _result2 = service.create_ai_battle(AiDifficulty::Hard).await.unwrap();
        
        let sessions = service.list_sessions();
        assert_eq!(sessions.len(), 2);
    }
    
    #[tokio::test]
    async fn test_delete_session() {
        let service = create_test_service();
        
        let create_result = service.create_ai_battle(AiDifficulty::Medium).await.unwrap();
        let session_id = create_result.game_id;
        
        // セッションが存在することを確認
        assert!(service.get_game_state(session_id).is_ok());
        
        // セッション削除
        let delete_result = service.delete_session(session_id);
        assert!(delete_result.is_ok());
        
        // セッションが削除されたことを確認
        assert!(matches!(
            service.get_game_state(session_id), 
            Err(AiBattleError::GameNotFound { .. })
        ));
    }
    
    #[tokio::test]
    async fn test_change_difficulty() {
        let service = create_test_service();
        
        let create_result = service.create_ai_battle(AiDifficulty::Easy).await.unwrap();
        let session_id = create_result.game_id;
        
        let result = service.change_difficulty(session_id, AiDifficulty::Hard);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.ai_difficulty, AiDifficulty::Hard);
    }
    
    #[tokio::test]
    async fn test_is_ai_thinking() {
        let service = create_test_service();
        
        let create_result = service.create_ai_battle(AiDifficulty::Easy).await.unwrap();
        let session_id = create_result.game_id;
        
        let result = service.is_ai_thinking(session_id);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // 初期状態では思考中ではない
    }
    
    #[test]
    fn test_get_service_stats() {
        let service = create_test_service();
        
        let stats = service.get_service_stats();
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.max_sessions, 10);
        assert_eq!(stats.ai_thinking_count, 0);
    }
    
    #[tokio::test]
    async fn test_cleanup_inactive_sessions() {
        let service = create_test_service();
        
        let removed_count = service.cleanup_inactive_sessions().await;
        assert_eq!(removed_count, 0); // 初期状態では削除されるセッションはない
    }
}