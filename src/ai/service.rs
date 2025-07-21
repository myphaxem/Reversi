//! AIサービスの抽象化層モジュール
//! 異なるAI実装（ローカル、HTTP、モックなど）を統一した
//! インターフェースで提供し、AIサービスの生成と管理を行う。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::game::{GameState, Position};
use crate::api::ai_battle::dto::AiDifficulty;
use crate::error::AIError;

/// AIの手の計算結果を表す構造体
/// 選択した位置と計算の統計情報を含む
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIMoveResult {
    /// AIが選択した手の位置
    pub position: Position,
    /// 思考時間（ミリ秒）
    pub thinking_time_ms: u64,
    /// 盤面評価値（実装によっては省略）
    pub evaluation_score: Option<f64>,
    /// 探索した深度（実装によっては省略）
    pub depth_reached: Option<u32>,
    /// 評価したノード数（実装によっては省略）
    pub nodes_evaluated: Option<u64>,
}

/// AIサービスの種類を表すenum
/// ローカル、リモート、テスト用などの実装を区別する
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AIServiceType {
    /// ローカルAI実装
    Local,
    /// HTTP経由のリモートAIサービス
    Http,
    /// テスト用のモックAI
    Mock,
}

/// AIサービスの状態情報を表す構造体
/// サービスの健全性やパフォーマンスの監視に使用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIServiceStatus {
    pub service_type: AIServiceType,
    pub name: String,
    pub available: bool,
    pub supported_difficulties: Vec<AiDifficulty>,
    pub last_check: DateTime<Utc>,
    pub average_response_time_ms: Option<u64>,
}

/// AIサービスの統一インターフェース
/// 異なるAI実装を同じ方法で呼び出すためのtrait
#[async_trait]
pub trait AIService: Send + Sync {
    /// 指定したゲーム状態と難易度でAIの手を計算する
    async fn calculate_move(
        &self, 
        game_state: &GameState, 
        difficulty: AiDifficulty
    ) -> Result<AIMoveResult, AIError>;
    
    /// サービスが利用可能かチェックする
    async fn is_available(&self) -> bool;
    
    /// サポートしている難易度レベルの一覧を返す
    fn get_supported_difficulties(&self) -> Vec<AiDifficulty>;
    
    /// サービス名を返す
    fn get_name(&self) -> &'static str;
    
    /// サービスの種類を返す
    fn get_service_type(&self) -> AIServiceType;
    
    /// サービスの現在の状態を取得する
    /// デフォルト実装では基本情報のみ提供
    async fn get_status(&self) -> AIServiceStatus {
        AIServiceStatus {
            service_type: self.get_service_type(),
            name: self.get_name().to_string(),
            available: self.is_available().await,
            supported_difficulties: self.get_supported_difficulties(),
            last_check: Utc::now(),
            average_response_time_ms: None,
        }
    }
    
    /// サービスの健全性チェックを実行し、レスポンス時間も測定する
    async fn health_check(&self) -> Result<AIServiceStatus, AIError> {
        let start_time = std::time::Instant::now();
        let available = self.is_available().await;
        let response_time = start_time.elapsed().as_millis() as u64;
        
        if available {
            Ok(AIServiceStatus {
                service_type: self.get_service_type(),
                name: self.get_name().to_string(),
                available: true,
                supported_difficulties: self.get_supported_difficulties(),
                last_check: Utc::now(),
                average_response_time_ms: Some(response_time),
            })
        } else {
            Err(AIError::ServiceUnavailable {
                service_name: self.get_name().to_string(),
                reason: "Service health check failed".to_string(),
            })
        }
    }
}

/// AIサービスの設定を管理する構造体
/// サービスの種類、エンドポイント、タイムアウトなどを設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIServiceConfig {
    pub service_type: AIServiceType,
    pub endpoint_url: Option<String>,
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub default_difficulty: AiDifficulty,
    pub enable_caching: bool,
}

impl Default for AIServiceConfig {
    fn default() -> Self {
        Self {
            service_type: AIServiceType::Local,
            endpoint_url: None,
            timeout_ms: 5000,
            max_retries: 3,
            default_difficulty: AiDifficulty::Easy,
            enable_caching: false,
        }
    }
}

/// AIサービスを生成するファクトリクラス
/// 設定に基づいて適切なAIサービス実装を選択して生成する
pub struct AIServiceFactory;

impl AIServiceFactory {
    /// 設定に基づいてAIサービスを生成する
    /// サービスタイプに応じて適切な実装を選択
    pub fn create_service(config: &AIServiceConfig) -> Result<Box<dyn AIService>, AIError> {
        match config.service_type {
            AIServiceType::Local => {
                // ローカルAIサービスを生成
                use crate::ai::local_service::LocalAIService;
                Ok(Box::new(LocalAIService::new()))
            }
            AIServiceType::Mock => {
                // テスト用モックAIサービスを生成
                use crate::ai::mock_service::{MockAIService, MockAIConfig};
                let mock_config = MockAIConfig::default();
                Ok(Box::new(MockAIService::new(mock_config)))
            }
            AIServiceType::Http => {
                // HTTP AIサービスは未実装
                Err(AIError::ServiceUnavailable {
                    service_name: "HttpAIService".to_string(),
                    reason: "HTTP AI service not yet implemented".to_string(),
                })
            }
        }
    }
    
    /// デフォルト設定のローカルAIサービスを生成する
    pub fn create_default_local() -> Result<Box<dyn AIService>, AIError> {
        use crate::ai::local_service::LocalAIService;
        Ok(Box::new(LocalAIService::new()))
    }
    
    /// 高速モードのローカルAIサービスを生成する
    /// 思考時間のシミュレーションを無効化して高速化
    pub fn create_fast_local() -> Result<Box<dyn AIService>, AIError> {
        use crate::ai::local_service::LocalAIService;
        Ok(Box::new(LocalAIService::new_fast()))
    }
    
    /// カスタム設定でモックAIサービスを生成する
    /// テスト時に特定の動作をシミュレートするために使用
    pub fn create_mock(config: Option<crate::ai::mock_service::MockAIConfig>) -> Result<Box<dyn AIService>, AIError> {
        use crate::ai::mock_service::{MockAIService, MockAIConfig};
        let mock_config = config.unwrap_or_default();
        Ok(Box::new(MockAIService::new(mock_config)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ai_service_config_default() {
        let config = AIServiceConfig::default();
        assert_eq!(config.service_type, AIServiceType::Local);
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.default_difficulty, AiDifficulty::Easy);
        assert!(!config.enable_caching);
    }
    
    #[test]
    fn test_ai_service_type_serialization() {
        let service_type = AIServiceType::Local;
        let serialized = serde_json::to_string(&service_type).unwrap();
        let deserialized: AIServiceType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(service_type, deserialized);
    }
}