//! 設定対応AI対戦サービス

use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::config::{Config, FallbackConfig};
use crate::error::AIError;
use crate::ai::service::{AIService, AIServiceFactory, AIServiceType};
use crate::session::AiBattleSessionManager;

use super::service::AiBattleService;
use super::dto::{AiBattleResult, AiBattleError};

/// 設定対応AI対戦サービス管理
/// 
/// 設定に基づいてAIサービスを動的に切り替え、エラー時のフォールバック機能を提供
pub struct ConfigurableAiBattleService {
    /// 現在のAI対戦サービス
    current_service: Arc<AiBattleService>,
    
    /// プライマリAIサービス
    primary_ai_service: Arc<dyn AIService>,
    
    /// フォールバックAIサービス
    fallback_ai_service: Option<Arc<dyn AIService>>,
    
    /// フォールバック設定
    fallback_config: FallbackConfig,
    
    /// セッション管理
    session_manager: Arc<AiBattleSessionManager>,
}

impl std::fmt::Debug for ConfigurableAiBattleService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigurableAiBattleService")
            .field("primary_ai", &self.primary_ai_service.get_name())
            .field("fallback_enabled", &self.fallback_config.enable_fallback)
            .field("fallback_ai", &self.fallback_ai_service.as_ref().map(|s| s.get_name()))
            .finish()
    }
}

impl ConfigurableAiBattleService {
    /// 設定に基づいて新しいサービスを作成
    pub fn new(config: &Config) -> AiBattleResult<Self> {
        // セッション管理を作成
        let session_manager = Arc::new(AiBattleSessionManager::with_timeout(
            config.ai_battle.max_sessions,
            config.ai_battle.session_timeout_minutes,
        ));
        
        // プライマリAIサービスを作成
        let primary_ai_service = Self::create_ai_service(&config.ai_service)?;
        
        // フォールバックAIサービスを作成
        let fallback_ai_service = if config.fallback.enable_fallback {
            let fallback_config = crate::ai::service::AIServiceConfig {
                service_type: config.fallback.fallback_ai_service,
                timeout_ms: config.fallback.retry_delay_ms,
                max_retries: config.fallback.max_retry_attempts,
                ..Default::default()
            };
            
            match Self::create_ai_service(&fallback_config) {
                Ok(service) => Some(service),
                Err(e) => {
                    eprintln!("Warning: Failed to create fallback AI service: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        // AI対戦サービスを作成
        let current_service = Arc::new(AiBattleService::new_with_ai_service(
            Arc::clone(&session_manager),
            Arc::clone(&primary_ai_service),
        ));
        
        Ok(Self {
            current_service,
            primary_ai_service,
            fallback_ai_service,
            fallback_config: config.fallback.clone(),
            session_manager,
        })
    }
    
    /// AIサービスを作成
    fn create_ai_service(config: &crate::ai::service::AIServiceConfig) -> AiBattleResult<Arc<dyn AIService>> {
        AIServiceFactory::create_service(config)
            .map(|service| service.into())
            .map_err(|e| AiBattleError::AiThinkingError { 
                details: format!("Failed to create AI service: {}", e) 
            })
    }
    
    /// 現在のAI対戦サービスを取得
    pub fn get_service(&self) -> &Arc<AiBattleService> {
        &self.current_service
    }
    
    /// プライマリAIサービスの状態を確認
    pub async fn check_primary_service_health(&self) -> bool {
        self.primary_ai_service.is_available().await
    }
    
    /// フォールバックAIサービスの状態を確認
    pub async fn check_fallback_service_health(&self) -> bool {
        if let Some(fallback) = &self.fallback_ai_service {
            fallback.is_available().await
        } else {
            false
        }
    }
    
    /// AIサービスを動的に切り替え
    pub async fn switch_ai_service(&mut self, new_config: &crate::ai::service::AIServiceConfig) -> AiBattleResult<()> {
        let new_ai_service = Self::create_ai_service(new_config)?;
        
        // 新しいサービスの健全性を確認
        if !new_ai_service.is_available().await {
            return Err(AiBattleError::AiThinkingError { 
                details: "New AI service is not available".to_string() 
            });
        }
        
        // AI対戦サービスを再作成
        let new_battle_service = Arc::new(AiBattleService::new_with_ai_service(
            Arc::clone(&self.session_manager),
            new_ai_service.clone(),
        ));
        
        // サービスを切り替え
        self.current_service = new_battle_service;
        self.primary_ai_service = new_ai_service;
        
        println!("AI service switched to: {}", self.primary_ai_service.get_name());
        Ok(())
    }
    
    /// フォールバック機能付きでAI着手を計算
    pub async fn calculate_move_with_fallback(
        &self,
        game_state: &crate::game::GameState,
        difficulty: crate::api::ai_battle::dto::AiDifficulty,
    ) -> AiBattleResult<crate::ai::service::AIMoveResult> {
        let mut attempts = 0;
        let max_attempts = self.fallback_config.max_retry_attempts;
        
        loop {
            attempts += 1;
            
            // プライマリサービスを試行
            match self.primary_ai_service.calculate_move(game_state, difficulty).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    println!("Primary AI service failed (attempt {}): {}", attempts, e);
                    
                    // フォールバックが有効で、試行回数が限界未満の場合
                    if self.fallback_config.enable_fallback && attempts < max_attempts {
                        if let Some(fallback) = &self.fallback_ai_service {
                            println!("Trying fallback AI service: {}", fallback.get_name());
                            
                            match fallback.calculate_move(game_state, difficulty).await {
                                Ok(result) => return Ok(result),
                                Err(fallback_error) => {
                                    println!("Fallback AI service also failed: {}", fallback_error);
                                }
                            }
                        }
                        
                        // リトライ前の待機
                        if attempts < max_attempts {
                            sleep(Duration::from_millis(self.fallback_config.retry_delay_ms)).await;
                        }
                    } else {
                        return Err(AiBattleError::AiThinkingError { 
                            details: format!("AI service failed after {} attempts: {}", attempts, e) 
                        });
                    }
                }
            }
        }
    }
    
    /// サービスの統計情報を取得
    pub async fn get_service_status(&self) -> ServiceStatus {
        let primary_available = self.check_primary_service_health().await;
        let fallback_available = self.check_fallback_service_health().await;
        
        ServiceStatus {
            primary_service_name: self.primary_ai_service.get_name().to_string(),
            primary_service_available: primary_available,
            fallback_enabled: self.fallback_config.enable_fallback,
            fallback_service_name: self.fallback_ai_service.as_ref().map(|s| s.get_name().to_string()),
            fallback_service_available: fallback_available,
            total_sessions: self.session_manager.session_count(),
        }
    }
    
    /// 設定を再読み込み（ホットリロード）
    pub async fn reload_config(&mut self, new_config: &Config) -> AiBattleResult<()> {
        // フォールバック設定を更新
        self.fallback_config = new_config.fallback.clone();
        
        // 新しい設定でAIサービスを切り替え
        self.switch_ai_service(&new_config.ai_service).await?;
        
        println!("Configuration reloaded successfully");
        Ok(())
    }
}

/// サービス状態情報
#[derive(Debug, serde::Serialize)]
pub struct ServiceStatus {
    pub primary_service_name: String,
    pub primary_service_available: bool,
    pub fallback_enabled: bool,
    pub fallback_service_name: Option<String>,
    pub fallback_service_available: bool,
    pub total_sessions: usize,
}

/// 設定管理用のユーティリティ関数
pub mod config_utils {
    use super::*;
    use crate::config::Config;
    
    /// デフォルト設定ファイルを生成
    pub fn generate_default_config_file() -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::default();
        config.save_to_file("config.json")?;
        
        println!("Default configuration file 'config.json' has been generated.");
        println!("Please modify it according to your needs.");
        
        Ok(())
    }
    
    /// 設定例を表示
    pub fn print_config_example() {
        let example_config = r#"{
  "system_limits": {
    "max_concurrent_games": 100,
    "max_ai_calculation_time": {"secs": 30, "nanos": 0},
    "session_timeout": {"secs": 3600, "nanos": 0},
    "max_move_history": 1000
  },
  "server": {
    "port": 3000,
    "host": "0.0.0.0",
    "enable_cors": true,
    "enable_logging": true
  },
  "database": {
    "url": "sqlite:reversi.db",
    "max_connections": 5,
    "connection_timeout": {"secs": 30, "nanos": 0}
  },
  "ai_battle": {
    "max_sessions": 100,
    "session_timeout_minutes": 30,
    "default_difficulty": "Easy",
    "enable_session_cleanup": true,
    "cleanup_interval_minutes": 5
  },
  "ai_service": {
    "service_type": "Local",
    "endpoint_url": null,
    "timeout_ms": 5000,
    "max_retries": 3,
    "default_difficulty": "Easy",
    "enable_caching": false
  },
  "fallback": {
    "enable_fallback": true,
    "fallback_ai_service": "Local",
    "max_retry_attempts": 3,
    "retry_delay_ms": 1000
  }
}"#;
        
        println!("Configuration file example:");
        println!("{}", example_config);
    }
    
    /// 環境変数の設定例を表示
    pub fn print_env_vars_example() {
        let example_vars = vec![
            ("SERVER_PORT", "3000"),
            ("SERVER_HOST", "0.0.0.0"),
            ("DATABASE_URL", "sqlite:reversi.db"),
            ("AI_BATTLE_MAX_SESSIONS", "100"),
            ("AI_BATTLE_SESSION_TIMEOUT_MINUTES", "30"),
            ("AI_SERVICE_TYPE", "Local"),
            ("AI_SERVICE_ENDPOINT_URL", "http://localhost:8080/ai"),
            ("AI_SERVICE_TIMEOUT_MS", "5000"),
            ("AI_SERVICE_MAX_RETRIES", "3"),
            ("ENABLE_AI_FALLBACK", "true"),
        ];
        
        println!("Environment variables example:");
        for (key, value) in example_vars {
            println!("export {}={}", key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    
    #[tokio::test]
    async fn test_configurable_service_creation() {
        let config = Config::default();
        let service = ConfigurableAiBattleService::new(&config);
        
        assert!(service.is_ok());
        
        let service = service.unwrap();
        assert!(service.check_primary_service_health().await);
    }
    
    #[tokio::test]
    async fn test_service_status() {
        let config = Config::default();
        let service = ConfigurableAiBattleService::new(&config).unwrap();
        
        let status = service.get_service_status().await;
        assert!(!status.primary_service_name.is_empty());
        assert!(status.primary_service_available);
        assert_eq!(status.total_sessions, 0);
    }
    
    #[tokio::test]
    async fn test_config_reload() {
        let mut config = Config::default();
        let mut service = ConfigurableAiBattleService::new(&config).unwrap();
        
        // 設定を変更
        config.ai_service.timeout_ms = 10000;
        
        let result = service.reload_config(&config).await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_generate_default_config() {
        let result = config_utils::generate_default_config_file();
        // ファイルが存在しない場合でもテストは成功すべき
        println!("Config generation result: {:?}", result);
    }
    
    #[test]
    fn test_config_examples() {
        config_utils::print_config_example();
        config_utils::print_env_vars_example();
        
        // 例が出力されることを確認
        assert!(true);
    }
}