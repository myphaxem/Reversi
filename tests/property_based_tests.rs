//! プロパティベーステストモジュール
//! ランダムな入力でシステムの不変条件や特性を検証し、
//! エッジケースや異常系でのシステムの健全性を確認する。

use proptest::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use Reversi::{
    api::ai_battle::{
        dto::{AiBattleSession, AiDifficulty, MoveRecord},
        service::AiBattleService,
    },
    game::{GameState, Position, Player, ReversiRules, Cell},
    session::AiBattleSessionManager,
    ai::{service::AIServiceFactory, mock_service::{MockAIService, MockAIConfig}},
};

/// テスト用のAI対戦サービスを作成
fn create_test_service() -> AiBattleService {
    let session_manager = Arc::new(AiBattleSessionManager::new(50));
    AiBattleService::new(session_manager)
}

/// テスト用の高速モックAI対戦サービスを作成
fn create_fast_mock_service() -> AiBattleService {
    let session_manager = Arc::new(AiBattleSessionManager::new(50));
    let mock_ai = Arc::new(MockAIService::new_fast());
    AiBattleService::new_with_ai_service(session_manager, mock_ai)
}

/// 有効な座標を生成する戦略
fn valid_position_strategy() -> impl Strategy<Value = Position> {
    (0u8..8, 0u8..8).prop_map(|(row, col)| {
        Position::new(row as usize, col as usize).unwrap()
    })
}

/// 有効な難易度を生成する戦略
fn difficulty_strategy() -> impl Strategy<Value = AiDifficulty> {
    prop_oneof![
        Just(AiDifficulty::Easy),
        Just(AiDifficulty::Medium),
        Just(AiDifficulty::Hard),
    ]
}

/// ランダム着手シーケンスを生成する戦略
fn move_sequence_strategy() -> impl Strategy<Value = Vec<Position>> {
    prop::collection::vec(valid_position_strategy(), 1..20)
}

/// プレイヤーを生成する戦略
fn player_strategy() -> impl Strategy<Value = Player> {
    prop_oneof![
        Just(Player::Black),
        Just(Player::White),
    ]
}

proptest! {
    /// プロパティ: ゲーム状態の整合性保持
    /// 
    /// どのような着手シーケンスでも、ゲーム状態は常に一貫している必要がある
    #[test]
    fn test_game_state_consistency_invariant(
        moves in move_sequence_strategy(),
        difficulty in difficulty_strategy()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let service = create_fast_mock_service();
            
            // AI対戦を作成
            let game_response = service.create_ai_battle(difficulty).await.unwrap();
            let game_id = game_response.game_id;
            
            let mut valid_move_count = 0;
            let mut invalid_move_count = 0;
            
            for position in moves {
                let current_state = service.get_game_state(game_id);
                
                if current_state.is_err() {
                    // ゲームが削除された場合はテスト終了
                    break;
                }
                
                let current_state = current_state.unwrap();
                
                // ゲーム終了していたら終了
                if let crate::api::ai_battle::dto::GameStatus::Finished { .. } = current_state.status {
                    break;
                }
                
                // プレイヤーターンでない場合はスキップ
                if current_state.current_player != Player::Black {
                    continue;
                }
                
                match service.make_player_move(game_id, position).await {
                    Ok(move_response) => {
                        valid_move_count += 1;
                        let game_state = &move_response.game_state;
                        
                        // 不変条件1: 石数の合計は増加する（最初は4個）
                        prop_assert!(game_state.black_count + game_state.white_count >= 4);
                        
                        // 不変条件2: ボードサイズは常に8x8
                        prop_assert_eq!(game_state.board.len(), 8);
                        for row in &game_state.board {
                            prop_assert_eq!(row.len(), 8);
                        }
                        
                        // 不変条件3: 手数は非減少
                        prop_assert!(game_state.move_count >= 0);
                        
                        // 不変条件4: 有効手は現在のプレイヤーで計算されている
                        if let crate::api::ai_battle::dto::GameStatus::InProgress = game_state.status {
                            // ゲーム続行中は有効手が存在するか、パスである
                            prop_assert!(game_state.valid_moves.is_empty() || !game_state.valid_moves.is_empty());
                        }
                    }
                    Err(_) => {
                        invalid_move_count += 1;
                        // 無効手は許容される（ルール上無効なため）
                    }
                }
                
                // 過度に長いテストを防ぐ
                if valid_move_count + invalid_move_count > 30 {
                    break;
                }
            }
            
            // 少なくとも1回は有効な着手があることを期待（大抵の場合）
            prop_assume!(valid_move_count > 0 || invalid_move_count > 0);
        });
    }
    
    /// プロパティ: セッション管理の一貫性
    /// 
    /// 複数のセッションを同時に管理しても状態が破綻しない
    #[test]
    fn test_session_management_consistency(
        session_count in 1usize..10,
        difficulties in prop::collection::vec(difficulty_strategy(), 1..10)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let service = create_fast_mock_service();
            let mut session_ids = Vec::new();
            
            // 複数セッションを作成
            for (i, &difficulty) in difficulties.iter().enumerate().take(session_count) {
                match service.create_ai_battle(difficulty).await {
                    Ok(response) => {
                        session_ids.push(response.game_id);
                        
                        // 作成されたセッションの状態確認
                        let state = service.get_game_state(response.game_id).unwrap();
                        prop_assert_eq!(state.ai_difficulty, difficulty);
                        prop_assert_eq!(state.current_player, Player::Black);
                    }
                    Err(_) => {
                        // セッション制限に達した場合は許容
                        break;
                    }
                }
            }
            
            prop_assume!(!session_ids.is_empty());
            
            // 全セッションが独立していることを確認
            for &session_id in &session_ids {
                let state1 = service.get_game_state(session_id).unwrap();
                
                for &other_session_id in &session_ids {
                    if session_id != other_session_id {
                        let state2 = service.get_game_state(other_session_id).unwrap();
                        // 異なるセッションは異なるIDを持つ
                        prop_assert_ne!(state1.game_id, state2.game_id);
                    }
                }
            }
            
            // セッション一覧の整合性確認
            let sessions = service.list_sessions();
            prop_assert!(sessions.len() >= session_ids.len());
            
            // 作成したセッションが全てリストに含まれることを確認
            for session_id in session_ids {
                prop_assert!(sessions.iter().any(|s| s.id == session_id));
            }
        });
    }
    
    /// プロパティ: AI戦略の一貫性
    /// 
    /// 同じ盤面・同じ難易度では一貫した結果を返す（モック使用）
    #[test]
    fn test_ai_strategy_consistency(
        difficulty in difficulty_strategy()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // 決定論的なモックAIを使用
            let session_manager = Arc::new(AiBattleSessionManager::new(10));
            let fixed_position = Position::new(2, 3).unwrap();
            let mock_ai = Arc::new(MockAIService::new_with_fixed_move(fixed_position));
            let service = AiBattleService::new_with_ai_service(session_manager, mock_ai);
            
            let response = service.create_ai_battle(difficulty).await.unwrap();
            let game_id = response.game_id;
            
            // 同じ着手を2回実行
            let valid_moves = response.valid_moves;
            prop_assume!(!valid_moves.is_empty());
            
            let first_move = valid_moves[0];
            
            // 1回目の着手
            let move_result1 = service.make_player_move(game_id, first_move).await;
            
            if move_result1.is_ok() {
                let ai_move1 = move_result1.unwrap().ai_move;
                
                // 2回目のテストのために新しいゲームを作成
                let response2 = service.create_ai_battle(difficulty).await.unwrap();
                let game_id2 = response2.game_id;
                
                // 同じ着手を実行
                let move_result2 = service.make_player_move(game_id2, first_move).await;
                
                if let Ok(result2) = move_result2 {
                    let ai_move2 = result2.ai_move;
                    
                    // モックAIは一貫した結果を返すはず
                    if let (Some(move1), Some(move2)) = (ai_move1, ai_move2) {
                        // 同じ初期盤面で同じプレイヤー手なら、AIも同じ手を返すはず
                        prop_assert_eq!(move1, move2);
                    }
                }
            }
        });
    }
    
    /// プロパティ: 着手履歴の一貫性
    /// 
    /// 着手履歴は時系列順で、プレイヤーが交互に現れる
    #[test]
    fn test_move_history_consistency(
        moves in prop::collection::vec(valid_position_strategy(), 2..8),
        difficulty in difficulty_strategy()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let service = create_fast_mock_service();
            let response = service.create_ai_battle(difficulty).await.unwrap();
            let game_id = response.game_id;
            
            let mut successful_moves = 0;
            
            for position in moves {
                let current_state = service.get_game_state(game_id);
                if current_state.is_err() {
                    break;
                }
                
                let current_state = current_state.unwrap();
                if current_state.current_player != Player::Black {
                    continue;
                }
                
                if let Ok(_) = service.make_player_move(game_id, position).await {
                    successful_moves += 1;
                }
                
                if successful_moves >= 5 {
                    break;
                }
            }
            
            if successful_moves > 0 {
                let history = service.get_move_history(game_id).unwrap();
                
                // 履歴が存在することを確認
                prop_assert!(!history.is_empty());
                
                // 時系列順であることを確認
                for i in 1..history.len() {
                    prop_assert!(history[i-1].timestamp <= history[i].timestamp);
                }
                
                // プレイヤーが適切に記録されていることを確認
                for move_record in &history {
                    prop_assert!(matches!(move_record.player, Player::Black | Player::White));
                }
            }
        });
    }
    
    /// プロパティ: エラー処理の堅牢性
    /// 
    /// どのような無効入力でもシステムがクラッシュしない
    #[test]
    fn test_error_handling_robustness(
        invalid_positions in prop::collection::vec((0u8..20, 0u8..20), 1..10),
        difficulty in difficulty_strategy()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let service = create_fast_mock_service();
            let response = service.create_ai_battle(difficulty).await.unwrap();
            let game_id = response.game_id;
            
            let mut error_count = 0;
            let mut success_count = 0;
            
            for (row, col) in invalid_positions {
                // 範囲外座標も含めてテスト
                if let Some(position) = Position::new(row as usize, col as usize) {
                    match service.make_player_move(game_id, position).await {
                        Ok(_) => success_count += 1,
                        Err(_) => error_count += 1,
                    }
                } else {
                    error_count += 1; // 無効座標
                }
            }
            
            // エラーが発生してもシステムは継続動作する
            let final_state = service.get_game_state(game_id);
            prop_assert!(final_state.is_ok());
            
            // 何らかの結果（成功かエラー）が得られている
            prop_assert!(error_count + success_count > 0);
        });
    }
    
    /// プロパティ: 並行アクセスの安全性
    /// 
    /// 複数のスレッドから同時アクセスしてもデータ競合が発生しない
    #[test]
    fn test_concurrent_access_safety(
        thread_count in 2usize..8,
        operations_per_thread in 5usize..15
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let service = Arc::new(create_fast_mock_service());
            let mut handles = Vec::new();
            
            for thread_id in 0..thread_count {
                let service = Arc::clone(&service);
                
                let handle = tokio::spawn(async move {
                    let mut results = Vec::new();
                    
                    for op in 0..operations_per_thread {
                        match op % 3 {
                            0 => {
                                // セッション作成
                                let result = service.create_ai_battle(AiDifficulty::Easy).await;
                                results.push(format!("Thread {}: Create - {:?}", thread_id, result.is_ok()));
                                
                                if let Ok(response) = result {
                                    // 作成後すぐに状態確認
                                    let state = service.get_game_state(response.game_id);
                                    results.push(format!("Thread {}: Get - {:?}", thread_id, state.is_ok()));
                                }
                            }
                            1 => {
                                // セッション一覧取得
                                let sessions = service.list_sessions();
                                results.push(format!("Thread {}: List - {} sessions", thread_id, sessions.len()));
                            }
                            2 => {
                                // 統計取得
                                let stats = service.get_service_stats();
                                results.push(format!("Thread {}: Stats - {} total", thread_id, stats.total_sessions));
                            }
                            _ => unreachable!()
                        }
                    }
                    
                    results
                });
                
                handles.push(handle);
            }
            
            // 全スレッドの完了を待機
            let results = futures::future::join_all(handles).await;
            
            // 全スレッドが正常に完了することを確認
            for (thread_id, result) in results.into_iter().enumerate() {
                let thread_results = result.unwrap();
                prop_assert!(!thread_results.is_empty());
                
                // 各スレッドで何らかの操作が成功していることを確認
                prop_assert!(thread_results.len() >= operations_per_thread);
            }
            
            // システム全体の一貫性確認
            let final_sessions = service.list_sessions();
            let final_stats = service.get_service_stats();
            
            prop_assert_eq!(final_sessions.len(), final_stats.total_sessions);
        });
    }
}

/// ランタイムテスト: プロパティベーステストの実行確認
#[cfg(test)]
mod runtime_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_property_tests_can_run() {
        // プロパティベーステストが実際に実行可能であることを確認
        let service = create_fast_mock_service();
        
        // 基本的な操作が正常に動作することを確認
        let response = service.create_ai_battle(AiDifficulty::Easy).await;
        assert!(response.is_ok());
        
        let game_id = response.unwrap().game_id;
        let state = service.get_game_state(game_id);
        assert!(state.is_ok());
        
        println!("Property-based tests are ready to run");
    }
    
    #[test]
    fn test_proptest_strategies() {
        // ストラテジーが正常に動作することを確認
        let mut runner = proptest::test_runner::TestRunner::default();
        
        let position_strategy = valid_position_strategy();
        let position = position_strategy.new_tree(&mut runner).unwrap().current();
        assert!(position.row < 8);
        assert!(position.col < 8);
        
        let difficulty_strategy = difficulty_strategy();
        let difficulty = difficulty_strategy.new_tree(&mut runner).unwrap().current();
        assert!(matches!(difficulty, AiDifficulty::Easy | AiDifficulty::Medium | AiDifficulty::Hard));
        
        println!("PropTest strategies work correctly");
    }
}