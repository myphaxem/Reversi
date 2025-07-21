//! アプリケーション設定管理モジュール
//! サーバー、データベース、AIサービスなどの設定を
//! 設定ファイルと環境変数から読み込んで管理する。

use serde::{Deserialize, Serialize};
use std::{env, fs, path::Path, time::Duration};

use crate::ai::service::{AIServiceConfig, AIServiceType};
use crate::api::ai_battle::dto::AiDifficulty;

/// Duration型をJSONでシリアライズするためのモジュール
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    /// Durationを(secs, nanos)のタプルとしてシリアライズ
    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let secs = duration.as_secs();
        let nanos = duration.subsec_nanos();
        (secs, nanos).serialize(serializer)
    }

    /// (secs, nanos)のタプルからDurationをデシリアライズ
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (secs, nanos) = <(u64, u32)>::deserialize(deserializer)?;
        Ok(Duration::new(secs, nanos))
    }
}

/// システムの制限値を定義する構造体
/// 同時ゲーム数、タイムアウト値などのリソース制限を管理
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLimits {
    /// 同時実行可能なゲーム数の上限
    pub max_concurrent_games: usize,
    /// AIの計算時間の上限
    #[serde(with = "duration_serde")]
    pub max_ai_calculation_time: Duration,
    /// セッションのタイムアウト時間
    #[serde(with = "duration_serde")]
    pub session_timeout: Duration,
    /// 保存する手の履歴の上限数
    pub max_move_history: usize,
}

impl Default for SystemLimits {
    /// バランスの取れたデフォルト制限値
    fn default() -> Self {
        Self {
            max_concurrent_games: 100,
            max_ai_calculation_time: Duration::from_secs(30),
            session_timeout: Duration::from_secs(3600),  // 1時間
            max_move_history: 1000,
        }
    }
}

/// サーバーの設定を管理する構造体
/// ポート番号、ホスト名、CORS設定などを含む
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
    pub enable_cors: bool,
    pub enable_logging: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "0.0.0.0".to_string(),
            enable_cors: true,
            enable_logging: true,
        }
    }
}

/// データベース接続の設定を管理する構造体
/// 接続文字列、コネクションプールの設定などを含む
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    #[serde(with = "duration_serde")]
    pub connection_timeout: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite:reversi.db".to_string(),
            max_connections: 5,
            connection_timeout: Duration::from_secs(30),
        }
    }
}

/// AI対戦セッションの設定を管理する構造体
/// セッション数制限、タイムアウト、クリーンアップ設定など
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiBattleConfig {
    pub max_sessions: usize,
    pub session_timeout_minutes: i64,
    pub default_difficulty: AiDifficulty,
    pub enable_session_cleanup: bool,
    pub cleanup_interval_minutes: u64,
}

impl Default for AiBattleConfig {
    fn default() -> Self {
        Self {
            max_sessions: 100,
            session_timeout_minutes: 30,
            default_difficulty: AiDifficulty::Easy,
            enable_session_cleanup: true,
            cleanup_interval_minutes: 5,
        }
    }
}

/// AIサービスのフォールバック設定を管理する構造体
/// メインAIが利用不可能な場合のフォールバック戦略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    pub enable_fallback: bool,
    pub fallback_ai_service: AIServiceType,
    pub max_retry_attempts: u32,
    pub retry_delay_ms: u64,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enable_fallback: true,
            fallback_ai_service: AIServiceType::Local,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// アプリケーションの全設定を統合するメイン設定構造体
/// 各サブシステムの設定をまとめて管理する
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub system_limits: SystemLimits,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub ai_battle: AiBattleConfig,
    pub ai_service: AIServiceConfig,
    pub fallback: FallbackConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            system_limits: SystemLimits::default(),
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            ai_battle: AiBattleConfig::default(),
            ai_service: AIServiceConfig::default(),
            fallback: FallbackConfig::default(),
        }
    }
}

/// 設定関連のエラーを表すenum
/// ファイル読み込み、パース、検証エラーなどを含む
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("設定ファイル読み込みエラー: {0}")]
    FileReadError(#[from] std::io::Error),
    
    #[error("設定ファイル解析エラー: {0}")]
    ParseError(#[from] serde_json::Error),
    
    #[error("環境変数エラー: {name} = {value}")]
    EnvVarError { name: String, value: String },
    
    #[error("設定値が無効です: {field} = {value}")]
    InvalidValue { field: String, value: String },
}

impl Config {
    /// 指定したファイルパスから設定を読み込む
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    /// 環境変数から設定を読み込む
    /// デフォルト値をベースに環境変数で上書きする
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Config::default();
        
        if let Ok(port) = env::var("SERVER_PORT") {
            config.server.port = port.parse().map_err(|_| ConfigError::EnvVarError {
                name: "SERVER_PORT".to_string(),
                value: port,
            })?;
        }
        
        if let Ok(host) = env::var("SERVER_HOST") {
            config.server.host = host;
        }
        
        if let Ok(database_url) = env::var("DATABASE_URL") {
            config.database.url = database_url;
        }
        
        if let Ok(max_sessions) = env::var("AI_BATTLE_MAX_SESSIONS") {
            config.ai_battle.max_sessions = max_sessions.parse().map_err(|_| ConfigError::EnvVarError {
                name: "AI_BATTLE_MAX_SESSIONS".to_string(),
                value: max_sessions,
            })?;
        }
        
        if let Ok(session_timeout) = env::var("AI_BATTLE_SESSION_TIMEOUT_MINUTES") {
            config.ai_battle.session_timeout_minutes = session_timeout.parse().map_err(|_| ConfigError::EnvVarError {
                name: "AI_BATTLE_SESSION_TIMEOUT_MINUTES".to_string(),
                value: session_timeout,
            })?;
        }
        
        if let Ok(ai_service_type) = env::var("AI_SERVICE_TYPE") {
            config.ai_service.service_type = match ai_service_type.to_lowercase().as_str() {
                "local" => AIServiceType::Local,
                "http" => AIServiceType::Http,
                "mock" => AIServiceType::Mock,
                _ => return Err(ConfigError::EnvVarError {
                    name: "AI_SERVICE_TYPE".to_string(),
                    value: ai_service_type,
                }),
            };
        }
        
        if let Ok(endpoint_url) = env::var("AI_SERVICE_ENDPOINT_URL") {
            config.ai_service.endpoint_url = Some(endpoint_url);
        }
        
        if let Ok(timeout) = env::var("AI_SERVICE_TIMEOUT_MS") {
            config.ai_service.timeout_ms = timeout.parse().map_err(|_| ConfigError::EnvVarError {
                name: "AI_SERVICE_TIMEOUT_MS".to_string(),
                value: timeout,
            })?;
        }
        
        if let Ok(retries) = env::var("AI_SERVICE_MAX_RETRIES") {
            config.ai_service.max_retries = retries.parse().map_err(|_| ConfigError::EnvVarError {
                name: "AI_SERVICE_MAX_RETRIES".to_string(),
                value: retries,
            })?;
        }
        
        if let Ok(enable_fallback) = env::var("ENABLE_AI_FALLBACK") {
            config.fallback.enable_fallback = enable_fallback.parse().map_err(|_| ConfigError::EnvVarError {
                name: "ENABLE_AI_FALLBACK".to_string(),
                value: enable_fallback,
            })?;
        }
        
        Ok(config)
    }
    
    /// 設定ファイルと環境変数を結合して設定を読み込む
    /// 設定ファイルがなくてもデフォルト値で動作する
    pub fn load() -> Self {
        let mut config = Config::default();
        
        if let Ok(file_config) = Self::from_file("config.json") {
            config = file_config;
        } else if let Ok(file_config) = Self::from_file("config/app.json") {
            config = file_config;
        } else if let Ok(file_config) = Self::from_file("/etc/reversi/config.json") {
            config = file_config;
        }
        
        // 環境変数で設定を上書き
        if let Ok(env_config) = Self::from_env() {
            config.server.port = env_config.server.port;
            config.server.host = env_config.server.host;
            config.database.url = env_config.database.url;
            config.ai_battle.max_sessions = env_config.ai_battle.max_sessions;
            config.ai_battle.session_timeout_minutes = env_config.ai_battle.session_timeout_minutes;
            config.ai_service = env_config.ai_service;
            config.fallback = env_config.fallback;
        }
        
        config
    }
    
    /// 現在の設定を指定したファイルに保存する
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
    
    /// 設定値の妥当性をチェックする
    /// 不正な値がある場合はConfigErrorを返す
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.server.port == 0 {
            return Err(ConfigError::InvalidValue {
                field: "server.port".to_string(),
                value: self.server.port.to_string(),
            });
        }
        
        if self.ai_battle.max_sessions == 0 {
            return Err(ConfigError::InvalidValue {
                field: "ai_battle.max_sessions".to_string(),
                value: self.ai_battle.max_sessions.to_string(),
            });
        }
        
        if self.ai_service.timeout_ms == 0 {
            return Err(ConfigError::InvalidValue {
                field: "ai_service.timeout_ms".to_string(),
                value: self.ai_service.timeout_ms.to_string(),
            });
        }
        
        Ok(())
    }
}

/// 後方互換性を保つためのメソッド群
impl Config {
    /// サーバーポート番号を取得する（後方互換用）
    pub fn server_port(&self) -> u16 {
        self.server.port
    }
    
    /// データベースURLを取得する（後方互換用）
    pub fn database_url(&self) -> &str {
        &self.database.url
    }
}