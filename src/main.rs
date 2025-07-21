//! Reversi APIサーバーのエントリポイント
//! 設定読み込み、AIサービス初期化、HTTPサーバー起動を行う。

use std::sync::Arc;

use Reversi::{
    api::{routes::{create_router, create_ai_battle_router}, handlers::AppState},
    api::ai_battle::{ConfigurableAiBattleService, config_utils},
    config::Config,
};
use tokio::net::TcpListener;

/// メイン関数 - サーバーの初期化と起動を担当
#[tokio::main]
async fn main() {
    // 設定ファイルと環境変数から統合設定を読み込み
    let config = Config::load();
    if let Err(e) = config.validate() {
        eprintln!("設定エラー: {}", e);
        eprintln!("デフォルト設定を生成: cargo run -- --generate-config");
        std::process::exit(1);
    }
    
    println!("設定読み込み完了:");
    println!("  サーバー: {}:{}", config.server.host, config.server.port);
    println!("  データベース: {}", config.database.url);
    println!("  AIサービス: {:?}", config.ai_service.service_type);
    println!("  フォールバック: {}", config.fallback.enable_fallback);
    println!("  最大セッション数: {}", config.ai_battle.max_sessions);
    
    let configurable_service = match ConfigurableAiBattleService::new(&config) {
        Ok(service) => Arc::new(service),
        Err(e) => {
            eprintln!("AI対戦サービス作成失敗: {}", e);
            eprintln!("AIサービス設定を確認してください");
            std::process::exit(1);
        }
    };
    
    let state = AppState::new_with_configurable_service(Arc::clone(&configurable_service));
    
    let app = create_router()
        .with_state(state.clone())
        .merge(create_ai_battle_router(state));
    
    let bind_address = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&bind_address)
        .await
        .unwrap_or_else(|e| {
            eprintln!("アドレスバインド失敗 {}: {}", bind_address, e);
            std::process::exit(1);
        });
    
    println!("Reversi APIサーバー開始: {}", bind_address);
    
    if !configurable_service.check_primary_service_health().await {
        eprintln!("警告: プライマリAIサービスが不健全");
        if configurable_service.check_fallback_service_health().await {
            println!("フォールバックAIサービス利用可能");
        }
    } else {
        println!("AIサービス正常");
    }
    
    println!("サーバー稼働中 (Ctrl+C で停止)");
    
    // Axumサーバーを開始し、リクエストの処理を開始
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}