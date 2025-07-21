//! ゲームの基本型定義モジュール
//! リバーシゲームで使用される基本的な型とenum、構造体を定義する。

use serde::{Deserialize, Serialize};

/// 盤面の各マスの状態を表現するenum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Cell {
    Empty,
    Black,
    White,
}

/// ゲームのプレイヤーを表すenum
/// 先手は黒、後手は白
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Player {
    Black,
    White,
}

impl Player {
    /// 相手プレイヤーを返す
    pub fn opposite(self) -> Player {
        match self {
            Player::Black => Player::White,
            Player::White => Player::Black,
        }
    }
    
    /// プレイヤーを対応するセル状態に変換する
    pub fn to_cell(self) -> Cell {
        match self {
            Player::Black => Cell::Black,
            Player::White => Cell::White,
        }
    }
}

/// 8x8リバーシ盤面上の座標を表す構造体
/// row, colともに0-7の範囲で有効
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    /// 範囲チェック付きのコンストラクタ
    /// 8x8盤面の範囲外の座標の場合はNoneを返す
    pub fn new(row: usize, col: usize) -> Option<Position> {
        if row < 8 && col < 8 {
            Some(Position { row, col })
        } else {
            None
        }
    }
    
    /// 座標が有効範囲内かチェックする
    pub fn is_valid(&self) -> bool {
        self.row < 8 && self.col < 8
    }
}

/// ゲームの1手を表現する構造体
/// 手の情報とひっくり返された石の位置、タイムスタンプを保持する
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Move {
    pub player: Player,
    pub position: Position,
    pub flipped: Vec<Position>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Move {
    /// 新しい手を作成する
    /// タイムスタンプは現在時刻で自動設定される
    pub fn new(player: Player, position: Position, flipped: Vec<Position>) -> Self {
        Self {
            player,
            position,
            flipped,
            timestamp: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_opposite() {
        assert_eq!(Player::Black.opposite(), Player::White);
        assert_eq!(Player::White.opposite(), Player::Black);
    }

    #[test]
    fn test_player_to_cell() {
        assert_eq!(Player::Black.to_cell(), Cell::Black);
        assert_eq!(Player::White.to_cell(), Cell::White);
    }

    #[test]
    fn test_position_new_valid() {
        let pos = Position::new(3, 4);
        assert!(pos.is_some());
        assert_eq!(pos.unwrap(), Position { row: 3, col: 4 });
    }

    #[test]
    fn test_position_new_invalid() {
        assert!(Position::new(8, 4).is_none());
        assert!(Position::new(3, 8).is_none());
        assert!(Position::new(10, 10).is_none());
    }

    #[test]
    fn test_position_is_valid() {
        assert!(Position { row: 0, col: 0 }.is_valid());
        assert!(Position { row: 7, col: 7 }.is_valid());
        assert!(!Position { row: 8, col: 0 }.is_valid());
        assert!(!Position { row: 0, col: 8 }.is_valid());
    }
    
    #[test]
    fn test_move_creation() {
        let pos = Position::new(3, 4).unwrap();
        let flipped = vec![Position::new(3, 3).unwrap()];
        let move_obj = Move::new(Player::Black, pos, flipped.clone());
        
        assert_eq!(move_obj.player, Player::Black);
        assert_eq!(move_obj.position, pos);
        assert_eq!(move_obj.flipped, flipped);
    }
}