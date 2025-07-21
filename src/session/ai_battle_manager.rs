//! AI対戦セッション管理モジュール
//! 同時にAI対戦を行うユーザーのセッションを管理し、
//! セッション数制限、タイムアウト処理、クリーンアップを担当する。

use dashmap::DashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

use crate::api::ai_battle::{AiBattleSession, AiBattleError, AiBattleResult, AiDifficulty};

/// AI対戦セッションの管理を行うメイン構造体
/// スレッドセーフなDashMapで同時アクセスを効率的に処理
#[derive(Debug, Clone)]
pub struct AiBattleSessionManager {
    /// アクティブセッションのコレクション
    sessions: Arc<DashMap<Uuid, AiBattleSession>>,
    /// 同時存在可能な最大セッション数
    max_sessions: usize,
    /// セッションのタイムアウト時間（分）
    session_timeout_minutes: i64,
}

impl AiBattleSessionManager {
    /// デフォルトタイムアウト（30分）でセッションマネージャーを作成
    pub fn new(max_sessions: usize) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            max_sessions,
            session_timeout_minutes: 30,
        }
    }
    
    /// カスタムタイムアウトでセッションマネージャーを作成
    pub fn with_timeout(max_sessions: usize, timeout_minutes: i64) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            max_sessions,
            session_timeout_minutes: timeout_minutes,
        }
    }
    
    /// 新しいAI対戦セッションを作成する
    /// 最大セッション数に達している場合はエラーを返す
    pub async fn create_session(&self, difficulty: AiDifficulty) -> AiBattleResult<Uuid> {
        // セッション数制限をチェック
        if self.sessions.len() >= self.max_sessions {
            return Err(AiBattleError::MaxSessionsReached { max: self.max_sessions });
        }
        
        let session = AiBattleSession::new(difficulty);
        let session_id = session.id;
        
        self.sessions.insert(session_id, session);
        
        Ok(session_id)
    }
    
    /// 指定したIDのセッションを取得する
    pub fn get_session(&self, session_id: &Uuid) -> AiBattleResult<AiBattleSession> {
        match self.sessions.get(session_id) {
            Some(session) => Ok(session.clone()),
            None => Err(AiBattleError::GameNotFound { game_id: *session_id }),
        }
    }
    
    pub fn update_session(&self, session: AiBattleSession) -> AiBattleResult<()> {
        let session_id = session.id;
        
        match self.sessions.get_mut(&session_id) {
            Some(mut existing_session) => {
                *existing_session = session;
                Ok(())
            }
            None => Err(AiBattleError::GameNotFound { game_id: session_id }),
        }
    }
    
    pub fn remove_session(&self, session_id: &Uuid) -> AiBattleResult<AiBattleSession> {
        match self.sessions.remove(session_id) {
            Some((_, session)) => Ok(session),
            None => Err(AiBattleError::GameNotFound { game_id: *session_id }),
        }
    }
    
    pub fn list_sessions(&self) -> Vec<AiBattleSession> {
        self.sessions.iter().map(|entry| entry.value().clone()).collect()
    }
    
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
    
    pub async fn cleanup_inactive_sessions(&self) -> usize {
        let cutoff_time = Utc::now() - Duration::minutes(self.session_timeout_minutes);
        let mut removed_count = 0;
        
        let expired_ids: Vec<Uuid> = self.sessions
            .iter()
            .filter(|entry| entry.value().last_move_at < cutoff_time)
            .map(|entry| *entry.key())
            .collect();
        
        for session_id in expired_ids {
            if self.sessions.remove(&session_id).is_some() {
                removed_count += 1;
            }
        }
        
        removed_count
    }
    
    pub fn session_exists(&self, session_id: &Uuid) -> bool {
        self.sessions.contains_key(session_id)
    }
    
    pub fn set_ai_thinking(&self, session_id: &Uuid, thinking: bool) -> AiBattleResult<()> {
        match self.sessions.get_mut(session_id) {
            Some(mut session) => {
                session.ai_thinking = thinking;
                Ok(())
            }
            None => Err(AiBattleError::GameNotFound { game_id: *session_id }),
        }
    }
    
    pub fn is_ai_thinking(&self, session_id: &Uuid) -> AiBattleResult<bool> {
        match self.sessions.get(session_id) {
            Some(session) => Ok(session.ai_thinking),
            None => Err(AiBattleError::GameNotFound { game_id: *session_id }),
        }
    }
    
    pub fn get_stats(&self) -> SessionStats {
        let total_sessions = self.sessions.len();
        let ai_thinking_count = self.sessions
            .iter()
            .filter(|entry| entry.value().ai_thinking)
            .count();
        
        let mut difficulty_counts = std::collections::HashMap::new();
        for entry in self.sessions.iter() {
            *difficulty_counts.entry(entry.value().ai_difficulty).or_insert(0) += 1;
        }
        
        SessionStats {
            total_sessions,
            max_sessions: self.max_sessions,
            ai_thinking_count,
            difficulty_counts,
        }
    }
}

impl Default for AiBattleSessionManager {
    fn default() -> Self {
        Self::new(100)
    }
}

#[derive(Debug)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub max_sessions: usize,
    pub ai_thinking_count: usize,
    pub difficulty_counts: std::collections::HashMap<AiDifficulty, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    
    #[tokio::test]
    async fn test_create_session() {
        let manager = AiBattleSessionManager::new(10);
        let session_id = manager.create_session(AiDifficulty::Easy).await.unwrap();
        
        assert!(manager.session_exists(&session_id));
        assert_eq!(manager.session_count(), 1);
    }
    
    #[tokio::test]
    async fn test_max_sessions_limit() {
        let manager = AiBattleSessionManager::new(2);
        
        let _session1 = manager.create_session(AiDifficulty::Easy).await.unwrap();
        let _session2 = manager.create_session(AiDifficulty::Medium).await.unwrap();
        
        let result = manager.create_session(AiDifficulty::Hard).await;
        assert!(matches!(result, Err(AiBattleError::MaxSessionsReached { max: 2 })));
    }
    
    #[tokio::test]
    async fn test_get_session() {
        let manager = AiBattleSessionManager::new(10);
        let session_id = manager.create_session(AiDifficulty::Medium).await.unwrap();
        
        let session = manager.get_session(&session_id).unwrap();
        assert_eq!(session.id, session_id);
        assert_eq!(session.ai_difficulty, AiDifficulty::Medium);
    }
    
    #[tokio::test]
    async fn test_get_nonexistent_session() {
        let manager = AiBattleSessionManager::new(10);
        let nonexistent_id = Uuid::new_v4();
        
        let result = manager.get_session(&nonexistent_id);
        assert!(matches!(result, Err(AiBattleError::GameNotFound { .. })));
    }
    
    #[tokio::test]
    async fn test_update_session() {
        let manager = AiBattleSessionManager::new(10);
        let session_id = manager.create_session(AiDifficulty::Easy).await.unwrap();
        
        let mut session = manager.get_session(&session_id).unwrap();
        session.ai_thinking = true;
        
        manager.update_session(session.clone()).unwrap();
        
        let updated_session = manager.get_session(&session_id).unwrap();
        assert!(updated_session.ai_thinking);
    }
    
    #[tokio::test]
    async fn test_remove_session() {
        let manager = AiBattleSessionManager::new(10);
        let session_id = manager.create_session(AiDifficulty::Hard).await.unwrap();
        
        assert!(manager.session_exists(&session_id));
        
        let removed_session = manager.remove_session(&session_id).unwrap();
        assert_eq!(removed_session.id, session_id);
        assert!(!manager.session_exists(&session_id));
    }
    
    #[tokio::test]
    async fn test_list_sessions() {
        let manager = AiBattleSessionManager::new(10);
        
        let _session1 = manager.create_session(AiDifficulty::Easy).await.unwrap();
        let _session2 = manager.create_session(AiDifficulty::Medium).await.unwrap();
        
        let sessions = manager.list_sessions();
        assert_eq!(sessions.len(), 2);
    }
    
    #[tokio::test]
    async fn test_ai_thinking_flag() {
        let manager = AiBattleSessionManager::new(10);
        let session_id = manager.create_session(AiDifficulty::Easy).await.unwrap();
        
        assert!(!manager.is_ai_thinking(&session_id).unwrap());
        
        manager.set_ai_thinking(&session_id, true).unwrap();
        assert!(manager.is_ai_thinking(&session_id).unwrap());
        
        manager.set_ai_thinking(&session_id, false).unwrap();
        assert!(!manager.is_ai_thinking(&session_id).unwrap());
    }
    
    #[tokio::test]
    async fn test_cleanup_inactive_sessions() {
        let manager = AiBattleSessionManager::with_timeout(10, 0);
        
        let _session_id = manager.create_session(AiDifficulty::Easy).await.unwrap();
        assert_eq!(manager.session_count(), 1);
        
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let removed_count = manager.cleanup_inactive_sessions().await;
        
        assert_eq!(removed_count, 1);
        assert_eq!(manager.session_count(), 0);
    }
    
    #[test]
    fn test_session_stats() {
        let manager = AiBattleSessionManager::new(10);
        let stats = manager.get_stats();
        
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.max_sessions, 10);
        assert_eq!(stats.ai_thinking_count, 0);
    }
}