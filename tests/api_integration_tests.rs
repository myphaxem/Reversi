//! AI対戦APIの統合テストモジュール
//! 実際のHTTPリクエストをシミュレートしてAPIの動作を確認し、
//! エンドポイント間の連携やエラーハンドリングをテストする。

use axum::{
    body::Body,
    http::{Request, StatusCode, Method},
    response::Response,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Barrier;
use tower::ServiceExt;
use uuid::Uuid;

use Reversi::{
    api::{handlers::AppState, routes::{create_router, create_ai_battle_router}},
    config::Config,
};

async fn create_test_app() -> axum::Router {
    let state = AppState::new();
    
    create_router()
        .with_state(state.clone())
        .merge(create_ai_battle_router(state))
}

async fn parse_response_json(response: Response<Body>) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn send_request(
    app: &mut axum::Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> Response<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("Content-Type", "application/json");
    
    let request = if let Some(body) = body {
        request.body(Body::from(serde_json::to_vec(&body).unwrap())).unwrap()
    } else {
        request.body(Body::empty()).unwrap()
    };
    
    app.oneshot(request).await.unwrap()
}

#[tokio::test]
async fn test_ai_battle_full_workflow() {
    let mut app = create_test_app().await;
    
    let create_response = send_request(
        &mut app,
        Method::POST,
        "/api/ai-battle",
        Some(json!({"difficulty": "easy"}))
    ).await;
    
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let game_data = parse_response_json(create_response).await;
    let game_id = game_data["game_id"].as_str().unwrap();
    
    let get_response = send_request(
        &mut app,
        Method::GET,
        &format!("/api/ai-battle/{}", game_id),
        None
    ).await;
    
    assert_eq!(get_response.status(), StatusCode::OK);
    let game_state = parse_response_json(get_response).await;
    assert_eq!(game_state["game_id"], game_id);
    assert_eq!(game_state["current_player"], "Black");
    assert!(game_state["valid_moves"].is_array());
    
    let valid_moves = game_state["valid_moves"].as_array().unwrap();
    assert!(!valid_moves.is_empty());
    
    let first_move = &valid_moves[0];
    let move_response = send_request(
        &mut app,
        Method::POST,
        &format!("/api/ai-battle/{}/move", game_id),
        Some(json!({
            "row": first_move[0],
            "col": first_move[1]
        }))
    ).await;
    
    assert_eq!(move_response.status(), StatusCode::OK);
    let move_result = parse_response_json(move_response).await;
    assert_eq!(move_result["success"], true);
    assert!(move_result["ai_move"].is_object() || move_result["ai_move"].is_null());
    
    let history_response = send_request(
        &mut app,
        Method::GET,
        &format!("/api/ai-battle/{}/history", game_id),
        None
    ).await;
    
    assert_eq!(history_response.status(), StatusCode::OK);
    let history = parse_response_json(history_response).await;
    assert!(history["moves"].is_array());
    assert!(history["total_moves"].as_u64().unwrap() >= 1);
    
    let difficulty_response = send_request(
        &mut app,
        Method::PUT,
        &format!("/api/ai-battle/{}/difficulty", game_id),
        Some(json!({"difficulty": "medium"}))
    ).await;
    
    assert_eq!(difficulty_response.status(), StatusCode::OK);
    let updated_game = parse_response_json(difficulty_response).await;
    assert_eq!(updated_game["ai_difficulty"], "Medium");
    
    let delete_response = send_request(
        &mut app,
        Method::DELETE,
        &format!("/api/ai-battle/{}", game_id),
        None
    ).await;
    
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
    
    let get_deleted_response = send_request(
        &mut app,
        Method::GET,
        &format!("/api/ai-battle/{}", game_id),
        None
    ).await;
    
    assert_eq!(get_deleted_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_all_endpoints_success() {
    let mut app = create_test_app().await;
    
    // 難易度一覧取得
    let difficulties_response = send_request(
        &mut app,
        Method::GET,
        "/api/ai-battle/difficulties",
        None
    ).await;
    
    assert_eq!(difficulties_response.status(), StatusCode::OK);
    let difficulties = parse_response_json(difficulties_response).await;
    assert!(difficulties["difficulties"].is_array());
    assert!(difficulties["difficulties"].as_array().unwrap().len() >= 3);
    
    // セッション一覧取得（初期状態では空）
    let sessions_response = send_request(
        &mut app,
        Method::GET,
        "/api/ai-battle/sessions",
        None
    ).await;
    
    assert_eq!(sessions_response.status(), StatusCode::OK);
    let sessions = parse_response_json(sessions_response).await;
    assert_eq!(sessions["total_count"], 0);
    
    // ヘルスチェック
    let health_response = send_request(
        &mut app,
        Method::GET,
        "/health",
        None
    ).await;
    
    assert_eq!(health_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_error_responses() {
    let mut app = create_test_app().await;
    
    // 存在しないゲームID
    let fake_id = Uuid::new_v4();
    let error_response = send_request(
        &mut app,
        Method::GET,
        &format!("/api/ai-battle/{}", fake_id),
        None
    ).await;
    
    assert_eq!(error_response.status(), StatusCode::NOT_FOUND);
    let error_data = parse_response_json(error_response).await;
    assert_eq!(error_data["error"], "GAME_NOT_FOUND");
    
    // 無効な難易度
    let invalid_difficulty_response = send_request(
        &mut app,
        Method::POST,
        "/api/ai-battle",
        Some(json!({"difficulty": "invalid"}))
    ).await;
    
    assert_eq!(invalid_difficulty_response.status(), StatusCode::BAD_REQUEST);
    
    // 無効な座標での着手
    let create_response = send_request(
        &mut app,
        Method::POST,
        "/api/ai-battle",
        Some(json!({"difficulty": "easy"}))
    ).await;
    
    let game_data = parse_response_json(create_response).await;
    let game_id = game_data["game_id"].as_str().unwrap();
    
    let invalid_move_response = send_request(
        &mut app,
        Method::POST,
        &format!("/api/ai-battle/{}/move", game_id),
        Some(json!({"row": 10, "col": 10}))
    ).await;
    
    assert_eq!(invalid_move_response.status(), StatusCode::BAD_REQUEST);
    let error_data = parse_response_json(invalid_move_response).await;
    assert_eq!(error_data["error"], "INVALID_POSITION");
    
    // 無効な着手（ゲームルール上）
    let invalid_game_move_response = send_request(
        &mut app,
        Method::POST,
        &format!("/api/ai-battle/{}/move", game_id),
        Some(json!({"row": 0, "col": 0}))
    ).await;
    
    assert_eq!(invalid_game_move_response.status(), StatusCode::BAD_REQUEST);
    let error_data = parse_response_json(invalid_game_move_response).await;
    assert_eq!(error_data["error"], "INVALID_MOVE");
}

#[tokio::test]
async fn test_concurrent_session_creation() {
    let app = create_test_app().await;
    let app = Arc::new(tokio::sync::Mutex::new(app));
    
    let barrier = Arc::new(Barrier::new(5));
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let app = Arc::clone(&app);
        let barrier = Arc::clone(&barrier);
        
        let handle = tokio::spawn(async move {
            barrier.wait().await;
            
            let mut app = app.lock().await;
            let response = send_request(
                &mut *app,
                Method::POST,
                "/api/ai-battle",
                Some(json!({"difficulty": "easy"}))
            ).await;
            
            (i, response.status())
        });
        
        handles.push(handle);
    }
    
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // 全てのセッション作成が成功することを確認
    for (i, result) in results {
        let (thread_id, status) = result.unwrap();
        println!("Thread {}: {:?}", thread_id, status);
        assert_eq!(status, StatusCode::CREATED);
    }
}

#[tokio::test]
async fn test_session_limit() {
    let mut app = create_test_app().await;
    let mut created_sessions = Vec::new();
    
    // セッションを大量作成（制限に達するまで）
    for i in 0..105 {
        let response = send_request(
            &mut app,
            Method::POST,
            "/api/ai-battle",
            Some(json!({"difficulty": "easy"}))
        ).await;
        
        if response.status() == StatusCode::CREATED {
            let game_data = parse_response_json(response).await;
            created_sessions.push(game_data["game_id"].as_str().unwrap().to_string());
        } else if response.status() == StatusCode::TOO_MANY_REQUESTS {
            // セッション制限に達した
            println!("Session limit reached at attempt {}", i);
            assert!(i >= 100);
            break;
        } else {
            panic!("Unexpected status: {:?}", response.status());
        }
    }
    
    assert!(!created_sessions.is_empty());
    println!("Created {} sessions before hitting limit", created_sessions.len());
}

#[tokio::test]
async fn test_response_times() {
    let mut app = create_test_app().await;
    
    // ゲーム作成のレスポンス時間を測定
    let start = tokio::time::Instant::now();
    
    let response = send_request(
        &mut app,
        Method::POST,
        "/api/ai-battle",
        Some(json!({"difficulty": "easy"}))
    ).await;
    
    let creation_time = start.elapsed();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert!(creation_time.as_millis() < 1000, "Game creation took too long: {}ms", creation_time.as_millis());
    
    let game_data = parse_response_json(response).await;
    let game_id = game_data["game_id"].as_str().unwrap();
    
    // プレイヤー着手のレスポンス時間を測定
    let start = tokio::time::Instant::now();
    
    let move_response = send_request(
        &mut app,
        Method::POST,
        &format!("/api/ai-battle/{}/move", game_id),
        Some(json!({"row": 2, "col": 3}))
    ).await;
    
    let move_time = start.elapsed();
    assert_eq!(move_response.status(), StatusCode::OK);
    assert!(move_time.as_millis() < 5000, "Move execution took too long: {}ms", move_time.as_millis());
    
    println!("Performance metrics:");
    println!("  Game creation: {}ms", creation_time.as_millis());
    println!("  Move execution: {}ms", move_time.as_millis());
}

#[tokio::test]
async fn test_game_state_consistency() {
    let mut app = create_test_app().await;
    
    let create_response = send_request(
        &mut app,
        Method::POST,
        "/api/ai-battle",
        Some(json!({"difficulty": "easy"}))
    ).await;
    
    let game_data = parse_response_json(create_response).await;
    let game_id = game_data["game_id"].as_str().unwrap();
    
    // 初期状態確認
    assert_eq!(game_data["current_player"], "Black");
    assert_eq!(game_data["black_count"], 2);
    assert_eq!(game_data["white_count"], 2);
    assert_eq!(game_data["move_count"], 0);
    assert!(game_data["valid_moves"].as_array().unwrap().len() == 4);
    
    // 複数回着手して状態の一貫性を確認
    for move_num in 1..=3 {
        let current_state_response = send_request(
            &mut app,
            Method::GET,
            &format!("/api/ai-battle/{}", game_id),
            None
        ).await;
        
        let current_state = parse_response_json(current_state_response).await;
        let valid_moves = current_state["valid_moves"].as_array().unwrap();
        
        if valid_moves.is_empty() {
            // パスの場合
            continue;
        }
        
        let first_move = &valid_moves[0];
        let move_response = send_request(
            &mut app,
            Method::POST,
            &format!("/api/ai-battle/{}/move", game_id),
            Some(json!({
                "row": first_move[0],
                "col": first_move[1]
            }))
        ).await;
        
        if move_response.status() == StatusCode::OK {
            let move_result = parse_response_json(move_response).await;
            let game_state = &move_result["game_state"];
            
            // 石数の合計は常に一定以上であることを確認
            let black_count = game_state["black_count"].as_u64().unwrap();
            let white_count = game_state["white_count"].as_u64().unwrap();
            assert!(black_count + white_count >= 4, "Invalid piece count at move {}", move_num);
            
            // ボード状態の一貫性確認
            let board = game_state["board"].as_array().unwrap();
            assert_eq!(board.len(), 8);
            for row in board {
                assert_eq!(row.as_array().unwrap().len(), 8);
            }
        }
    }
}

#[tokio::test]
async fn test_all_http_methods_and_endpoints() {
    let mut app = create_test_app().await;
    
    // 全エンドポイントの存在確認
    let endpoints = vec![
        (Method::POST, "/api/ai-battle", Some(json!({"difficulty": "easy"}))),
        (Method::GET, "/api/ai-battle/difficulties", None),
        (Method::GET, "/api/ai-battle/sessions", None),
        (Method::GET, "/health", None),
    ];
    
    for (method, endpoint, body) in endpoints {
        let response = send_request(&mut app, method, endpoint, body).await;
        assert!(
            response.status().is_success() || response.status() == StatusCode::CREATED,
            "Endpoint {} {} failed with status: {:?}",
            method,
            endpoint,
            response.status()
        );
    }
    
    // 個別ゲーム操作エンドポイントのテスト（ゲーム作成後）
    let create_response = send_request(
        &mut app,
        Method::POST,
        "/api/ai-battle",
        Some(json!({"difficulty": "easy"}))
    ).await;
    
    let game_data = parse_response_json(create_response).await;
    let game_id = game_data["game_id"].as_str().unwrap();
    
    let game_endpoints = vec![
        (Method::GET, format!("/api/ai-battle/{}", game_id), None),
        (Method::GET, format!("/api/ai-battle/{}/history", game_id), None),
        (Method::PUT, format!("/api/ai-battle/{}/difficulty", game_id), Some(json!({"difficulty": "medium"}))),
        (Method::POST, format!("/api/ai-battle/{}/move", game_id), Some(json!({"row": 2, "col": 3}))),
        (Method::DELETE, format!("/api/ai-battle/{}", game_id), None),
    ];
    
    for (method, endpoint, body) in game_endpoints {
        let response = send_request(&mut app, method, &endpoint, body).await;
        assert!(
            response.status().is_success() || response.status().is_client_error(),
            "Endpoint {} {} failed with status: {:?}",
            method,
            endpoint,
            response.status()
        );
    }
}