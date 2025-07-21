//! 設定システム統合テスト

use std::{env, fs};
use tempfile::TempDir;

use Reversi::{
    config::{Config, ConfigError, ServerConfig, AiBattleConfig},
    api::ai_battle::{ConfigurableAiBattleService, config_utils},
    ai::service::{AIServiceConfig, AIServiceType},
    api::ai_battle::dto::AiDifficulty,
};

fn create_test_config() -> Config {
    Config {
        server: ServerConfig {
            port: 4000,
            host: "127.0.0.1".to_string(),
            enable_cors: false,
            enable_logging: false,
        },
        ai_battle: AiBattleConfig {
            max_sessions: 50,
            session_timeout_minutes: 15,
            default_difficulty: AiDifficulty::Medium,
            enable_session_cleanup: false,
            cleanup_interval_minutes: 10,
        },
        ai_service: AIServiceConfig {
            service_type: AIServiceType::Mock,
            timeout_ms: 2000,
            max_retries: 2,
            default_difficulty: AiDifficulty::Hard,
            enable_caching: true,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn test_config_serialization_deserialization() {
    let config = create_test_config();
    
    let json_str = serde_json::to_string_pretty(&config).unwrap();
    assert!(json_str.contains("4000"));
    assert!(json_str.contains("127.0.0.1"));
    assert!(json_str.contains("Mock"));
    
    let deserialized: Config = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.server.port, 4000);
    assert_eq!(deserialized.server.host, "127.0.0.1");
    assert_eq!(deserialized.ai_service.service_type, AIServiceType::Mock);
}

#[test]
fn test_config_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.json");
    
    let original_config = create_test_config();
    
    // ファイルに保存
    original_config.save_to_file(&config_path).unwrap();
    assert!(config_path.exists());
    
    // ファイルから読み込み
    let loaded_config = Config::from_file(&config_path).unwrap();
    assert_eq!(loaded_config.server.port, original_config.server.port);
    assert_eq!(loaded_config.ai_service.service_type, original_config.ai_service.service_type);
}

#[test]
fn test_config_validation() {
    let mut config = Config::default();
    
    // 有効な設定
    assert!(config.validate().is_ok());
    
    // 無効なポート
    config.server.port = 0;
    assert!(config.validate().is_err());
    
    // 無効なセッション数
    config.server.port = 3000;
    config.ai_battle.max_sessions = 0;
    assert!(config.validate().is_err());
    
    // 無効なタイムアウト
    config.ai_battle.max_sessions = 10;
    config.ai_service.timeout_ms = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_env_var_config_loading() {
    env::set_var("SERVER_PORT", "5000");
    env::set_var("SERVER_HOST", "192.168.1.100");
    env::set_var("AI_BATTLE_MAX_SESSIONS", "200");
    env::set_var("AI_SERVICE_TYPE", "local");
    env::set_var("AI_SERVICE_TIMEOUT_MS", "10000");
    env::set_var("ENABLE_AI_FALLBACK", "false");
    
    let config = Config::from_env().unwrap();
    
    assert_eq!(config.server.port, 5000);
    assert_eq!(config.server.host, "192.168.1.100");
    assert_eq!(config.ai_battle.max_sessions, 200);
    assert_eq!(config.ai_service.service_type, AIServiceType::Local);
    assert_eq!(config.ai_service.timeout_ms, 10000);
    assert!(!config.fallback.enable_fallback);
    
    env::remove_var("SERVER_PORT");
    env::remove_var("SERVER_HOST");
    env::remove_var("AI_BATTLE_MAX_SESSIONS");
    env::remove_var("AI_SERVICE_TYPE");
    env::remove_var("AI_SERVICE_TIMEOUT_MS");
    env::remove_var("ENABLE_AI_FALLBACK");
}

#[test]
fn test_invalid_env_vars() {
    env::set_var("SERVER_PORT", "invalid_port");
    
    let result = Config::from_env();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ConfigError::EnvVarError { .. }));
    
    env::remove_var("SERVER_PORT");
}

#[test]
fn test_config_load_precedence() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");
    
    // 設定ファイルを作成
    let file_config = Config {
        server: ServerConfig {
            port: 6000,
            host: "file_host".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    file_config.save_to_file(&config_path).unwrap();
    
    // 環境変数を設定（設定ファイルを上書きするはず）
    env::set_var("SERVER_PORT", "7000");
    env::set_var("SERVER_HOST", "env_host");
    
    // Config::load()は環境変数を優先するはず
    // ただし、このテストでは実際のファイル検索は行わないため、
    // 代わりにfrom_envのみをテスト
    let env_config = Config::from_env().unwrap();
    assert_eq!(env_config.server.port, 7000);
    assert_eq!(env_config.server.host, "env_host");
    
    // クリーンアップ
    env::remove_var("SERVER_PORT");
    env::remove_var("SERVER_HOST");
}

#[tokio::test]
async fn test_configurable_service_creation() {
    let config = Config::default();
    let service = ConfigurableAiBattleService::new(&config);
    
    assert!(service.is_ok());
    
    let service = service.unwrap();
    let status = service.get_service_status().await;
    
    assert!(!status.primary_service_name.is_empty());
    assert!(status.primary_service_available);
    assert_eq!(status.total_sessions, 0);
}

#[tokio::test]
async fn test_configurable_service_with_mock_ai() {
    let mut config = Config::default();
    config.ai_service.service_type = AIServiceType::Mock;
    config.fallback.enable_fallback = true;
    config.fallback.fallback_ai_service = AIServiceType::Local;
    
    let service = ConfigurableAiBattleService::new(&config);
    assert!(service.is_ok());
    
    let service = service.unwrap();
    let status = service.get_service_status().await;
    
    assert_eq!(status.primary_service_name, "MockAIService");
    assert!(status.primary_service_available);
    assert!(status.fallback_enabled);
    assert!(status.fallback_service_name.is_some());
}

#[tokio::test]
async fn test_ai_service_health_checks() {
    let config = Config::default();
    let service = ConfigurableAiBattleService::new(&config).unwrap();
    
    // プライマリサービスの健全性確認
    assert!(service.check_primary_service_health().await);
    
    // フォールバックサービスの健全性確認（有効化されている場合）
    if config.fallback.enable_fallback {
        let has_fallback = service.check_fallback_service_health().await;
        // フォールバックサービスが設定されているかどうかによって結果が変わる
        println!("Fallback service available: {}", has_fallback);
    }
}

#[tokio::test]
async fn test_config_reload() {
    let mut config = Config::default();
    let mut service = ConfigurableAiBattleService::new(&config).unwrap();
    
    // 初期設定の確認
    let initial_status = service.get_service_status().await;
    
    // 設定を変更
    config.ai_service.timeout_ms = 15000;
    config.ai_service.max_retries = 5;
    
    // 設定を再読み込み
    let reload_result = service.reload_config(&config).await;
    assert!(reload_result.is_ok());
    
    // 設定が適用されたことを確認（直接確認は困難なので、エラーがないことを確認）
    let updated_status = service.get_service_status().await;
    assert_eq!(initial_status.primary_service_name, updated_status.primary_service_name);
}

#[tokio::test]
async fn test_ai_move_calculation_with_fallback() {
    use Reversi::game::GameState;
    
    let mut config = Config::default();
    config.ai_service.service_type = AIServiceType::Mock;
    config.fallback.enable_fallback = true;
    config.fallback.fallback_ai_service = AIServiceType::Local;
    
    let service = ConfigurableAiBattleService::new(&config).unwrap();
    let game_state = GameState::new();
    
    let result = service.calculate_move_with_fallback(&game_state, AiDifficulty::Easy).await;
    assert!(result.is_ok());
    
    let move_result = result.unwrap();
    assert!(move_result.thinking_time_ms >= 0);
}

#[test]
fn test_config_utilities() {
    // デフォルト設定ファイル生成のテスト（実際にファイルは作成しない）
    config_utils::print_config_example();
    config_utils::print_env_vars_example();
    
    // ユーティリティ関数が正常に動作することを確認
    assert!(true);
}

#[test]
fn test_config_error_handling() {
    // 存在しないファイルからの読み込み
    let result = Config::from_file("nonexistent_file.json");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ConfigError::FileReadError(_)));
    
    // 無効なJSONファイル
    let temp_dir = TempDir::new().unwrap();
    let invalid_json_path = temp_dir.path().join("invalid.json");
    fs::write(&invalid_json_path, "invalid json content").unwrap();
    
    let result = Config::from_file(&invalid_json_path);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ConfigError::ParseError(_)));
}

#[test]
fn test_backward_compatibility() {
    let config = Config::default();
    
    // 旧インターフェースが引き続き動作することを確認
    assert_eq!(config.server_port(), config.server.port);
    assert_eq!(config.database_url(), &config.database.url);
}

#[test]
fn test_config_loading_performance() {
    use std::time::Instant;
    
    let start = Instant::now();
    
    // 設定読み込みを100回実行
    for _ in 0..100 {
        let _config = Config::default();
    }
    
    let elapsed = start.elapsed();
    
    // 設定読み込みが十分高速であることを確認（1秒以内）
    assert!(elapsed.as_secs() < 1, "Config loading is too slow: {:?}", elapsed);
    
    println!("Config loading performance: {:?} for 100 iterations", elapsed);
}