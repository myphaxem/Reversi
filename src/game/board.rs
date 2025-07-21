//! リバーシゲームの盤面状態を管理するモジュール
//! 8x8グリッドの盤面と石の配置、操作を担当する。

use super::types::{Cell, Player, Position};
use serde::{Deserialize, Serialize};

/// 8x8リバーシ盤面を表現する構造体
/// 各マスのCell状態を保持し、盤面操作を提供する
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    cells: [[Cell; 8]; 8],
}

impl Board {
    /// 新しいリバーシ盤面を作成する
    /// 中央の4マスに初期配置（白黒交互）を設定する
    pub fn new() -> Self {
        let mut board = Board {
            cells: [[Cell::Empty; 8]; 8],
        };
        
        // リバーシの標準初期配置
        board.cells[3][3] = Cell::White;
        board.cells[3][4] = Cell::Black;
        board.cells[4][3] = Cell::Black;
        board.cells[4][4] = Cell::White;
        
        board
    }
    
    /// 指定した位置のセル状態を取得する
    /// 範囲外の場合はNoneを返す
    pub fn get_cell(&self, position: Position) -> Option<Cell> {
        if position.is_valid() {
            Some(self.cells[position.row][position.col])
        } else {
            None
        }
    }
    
    /// 指定した位置にセル状態を設定する
    /// 範囲外の場合はfalseを返す
    pub fn set_cell(&mut self, position: Position, cell: Cell) -> bool {
        if position.is_valid() {
            self.cells[position.row][position.col] = cell;
            true
        } else {
            false
        }
    }
    
    /// 指定した位置が空かチェックする
    pub fn is_empty(&self, position: Position) -> bool {
        matches!(self.get_cell(position), Some(Cell::Empty))
    }
    
    /// 盤面上の黒石と白石の数を数える
    /// 戻り値: (黒石数, 白石数)
    pub fn count_pieces(&self) -> (u8, u8) {
        let mut black_count = 0;
        let mut white_count = 0;
        
        for row in &self.cells {
            for &cell in row {
                match cell {
                    Cell::Black => black_count += 1,
                    Cell::White => white_count += 1,
                    Cell::Empty => {}
                }
            }
        }
        
        (black_count, white_count)
    }
    
    /// デバッグ用の盤面表示文字列を生成する
    /// •で黒、○で白、.で空マスを表現
    pub fn display(&self) -> String {
        let mut result = String::new();
        result.push_str("  0 1 2 3 4 5 6 7\n");
        
        // 各行を処理して表示文字列を構築
        for (row_idx, row) in self.cells.iter().enumerate() {
            result.push_str(&format!("{} ", row_idx));
            // 各セルをシンボルに変換
            for &cell in row {
                let symbol = match cell {
                    Cell::Empty => ".",
                    Cell::Black => "●",
                    Cell::White => "○",
                };
                result.push_str(&format!("{} ", symbol));
            }
            result.push('\n');
        }
        
        result
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_new_initial_state() {
        let board = Board::new();
        
        assert_eq!(board.get_cell(Position::new(3, 3).unwrap()), Some(Cell::White));
        assert_eq!(board.get_cell(Position::new(3, 4).unwrap()), Some(Cell::Black));
        assert_eq!(board.get_cell(Position::new(4, 3).unwrap()), Some(Cell::Black));
        assert_eq!(board.get_cell(Position::new(4, 4).unwrap()), Some(Cell::White));
        
        assert_eq!(board.get_cell(Position::new(0, 0).unwrap()), Some(Cell::Empty));
        assert_eq!(board.get_cell(Position::new(7, 7).unwrap()), Some(Cell::Empty));
    }

    #[test]
    fn test_board_get_cell_invalid_position() {
        let board = Board::new();
        assert_eq!(board.get_cell(Position { row: 8, col: 0 }), None);
        assert_eq!(board.get_cell(Position { row: 0, col: 8 }), None);
    }

    #[test]
    fn test_board_set_cell() {
        let mut board = Board::new();
        let pos = Position::new(0, 0).unwrap();
        
        assert!(board.set_cell(pos, Cell::Black));
        assert_eq!(board.get_cell(pos), Some(Cell::Black));
    }

    #[test]
    fn test_board_set_cell_invalid_position() {
        let mut board = Board::new();
        assert!(!board.set_cell(Position { row: 8, col: 0 }, Cell::Black));
    }

    #[test]
    fn test_board_is_empty() {
        let board = Board::new();
        
        assert!(board.is_empty(Position::new(0, 0).unwrap()));
        assert!(!board.is_empty(Position::new(3, 3).unwrap()));
    }

    #[test]
    fn test_board_count_pieces_initial() {
        let board = Board::new();
        let (black_count, white_count) = board.count_pieces();
        
        assert_eq!(black_count, 2);
        assert_eq!(white_count, 2);
    }

    #[test]
    fn test_board_display() {
        let board = Board::new();
        let display = board.display();
        
        assert!(display.contains("0 1 2 3 4 5 6 7"));
        assert!(display.contains("●"));
        assert!(display.contains("○"));
        assert!(display.contains("."));
    }
}