//! ゲーム状態管理モジュール
//! リバーシゲームの全体的な状態（盤面、プレイヤー、進行状態など）を管理する。

use super::types::{Move, Player, Position};
use super::board::Board;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// ゲームの進行状態を表すenum
/// ゲームの状態遷移と終了時の情報を管理する
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameStatus {
    /// ゲーム進行中
    InProgress,
    /// ゲーム終了（勝者と最終スコアを記録）
    Finished { 
        winner: Option<Player>, 
        score: (u8, u8) 
    },
    /// ゲーム一時停止
    Paused,
}

/// リバーシゲームの全体状態を保持する構造体
/// 盤面、現在のプレイヤー、手の履歴などを全て含む
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub id: Uuid,
    pub board: Board,
    pub current_player: Player,
    pub game_status: GameStatus,
    pub move_history: Vec<Move>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl GameState {
    /// 新しいゲーム状態を作成する
    /// 初期状態：黒の番でゲーム開始
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            board: Board::new(),
            current_player: Player::Black,
            game_status: GameStatus::InProgress,
            move_history: Vec::new(),
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }
    
    /// 指定IDで新しいゲーム状態を作成する
    /// テストや特定のIDが必要な場合に使用
    pub fn new_with_id(id: Uuid) -> Self {
        Self {
            id,
            board: Board::new(),
            current_player: Player::Black,
            game_status: GameStatus::InProgress,
            move_history: Vec::new(),
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }
    
    /// ゲームが終了しているかチェックする
    pub fn is_finished(&self) -> bool {
        matches!(self.game_status, GameStatus::Finished { .. })
    }
    
    /// ゲームが一時停止中かチェックする
    pub fn is_paused(&self) -> bool {
        matches!(self.game_status, GameStatus::Paused)
    }
    
    /// 現在のプレイヤーを交代する
    /// 手の実行後やパス時に呼び出される
    pub fn switch_player(&mut self) {
        self.current_player = self.current_player.opposite();
        self.last_updated = Utc::now();
    }
    
    /// 手の履歴に新しい手を追加する
    /// 最終更新時刻も同時に更新する
    pub fn add_move(&mut self, game_move: Move) {
        self.move_history.push(game_move);
        self.last_updated = Utc::now();
    }
    
    /// ゲームを一時停止する
    /// 進行中のゲームのみ停止可能
    pub fn pause(&mut self) {
        if matches!(self.game_status, GameStatus::InProgress) {
            self.game_status = GameStatus::Paused;
            self.last_updated = Utc::now();
        }
    }
    
    /// 一時停止中のゲームを再開する
    pub fn resume(&mut self) {
        if matches!(self.game_status, GameStatus::Paused) {
            self.game_status = GameStatus::InProgress;
            self.last_updated = Utc::now();
        }
    }
    
    /// ゲームを終了させる
    /// 勝者と最終スコアを記録する
    pub fn finish(&mut self, winner: Option<Player>) {
        let (black_count, white_count) = self.board.count_pieces();
        self.game_status = GameStatus::Finished {
            winner,
            score: (black_count, white_count),
        };
        self.last_updated = Utc::now();
    }
    
    /// 現在のスコアを取得する
    /// 戻り値: (黒石数, 白石数)
    pub fn get_score(&self) -> (u8, u8) {
        self.board.count_pieces()
    }
    
    /// これまでの手数を取得する
    pub fn get_move_count(&self) -> usize {
        self.move_history.len()
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_state_new() {
        let game = GameState::new();
        
        assert_eq!(game.current_player, Player::Black);
        assert!(matches!(game.game_status, GameStatus::InProgress));
        assert_eq!(game.move_history.len(), 0);
        assert_eq!(game.get_score(), (2, 2));
    }

    #[test]
    fn test_game_state_new_with_id() {
        let custom_id = Uuid::new_v4();
        let game = GameState::new_with_id(custom_id);
        
        assert_eq!(game.id, custom_id);
        assert_eq!(game.current_player, Player::Black);
    }

    #[test]
    fn test_game_state_status_checks() {
        let mut game = GameState::new();
        
        assert!(!game.is_finished());
        assert!(!game.is_paused());
        
        game.pause();
        assert!(game.is_paused());
        assert!(!game.is_finished());
        
        game.resume();
        assert!(!game.is_paused());
        
        game.finish(Some(Player::Black));
        assert!(game.is_finished());
    }

    #[test]
    fn test_game_state_switch_player() {
        let mut game = GameState::new();
        
        assert_eq!(game.current_player, Player::Black);
        
        game.switch_player();
        assert_eq!(game.current_player, Player::White);
        
        game.switch_player();
        assert_eq!(game.current_player, Player::Black);
    }

    #[test]
    fn test_game_state_add_move() {
        let mut game = GameState::new();
        let pos = Position::new(2, 3).unwrap();
        let game_move = Move::new(Player::Black, pos, vec![]);
        
        assert_eq!(game.get_move_count(), 0);
        
        game.add_move(game_move.clone());
        assert_eq!(game.get_move_count(), 1);
        assert_eq!(game.move_history[0].position, pos);
    }

    #[test]
    fn test_game_state_finish() {
        let mut game = GameState::new();
        
        game.finish(Some(Player::Black));
        
        assert!(game.is_finished());
        if let GameStatus::Finished { winner, score } = &game.game_status {
            assert_eq!(*winner, Some(Player::Black));
            assert_eq!(*score, (2, 2)); // Initial board state
        } else {
            panic!("Game should be finished");
        }
    }

    #[test]
    fn test_game_state_pause_resume() {
        let mut game = GameState::new();
        
        // Can only pause in progress games
        game.pause();
        assert!(game.is_paused());
        
        // Can only resume paused games
        game.resume();
        assert!(!game.is_paused());
        assert!(matches!(game.game_status, GameStatus::InProgress));
        
        // Cannot pause finished games
        game.finish(None);
        game.pause();
        assert!(game.is_finished()); // Should still be finished
    }
}