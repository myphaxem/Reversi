//! リバーシのルールとゲームロジック実装モジュール
//! 合法手の判定、石のフリップ処理、ゲーム終了判定などを担当する。

use super::types::{Cell, Player, Position, Move};
use super::board::Board;
use super::state::GameState;
use crate::error::{GameError, Result};

/// 盤面上の8方向への移動ベクトル
/// 上下左右および斜めの8方向で石のフリップをチェックする
const DIRECTIONS: [(i8, i8); 8] = [
    (-1, -1), (-1, 0), (-1, 1),  // 左上、上、右上
    (0, -1),           (0, 1),   // 左、右
    (1, -1),  (1, 0),  (1, 1),   // 左下、下、右下
];

/// リバーシのルールを実装する構造体
/// スタティックメソッドのみを提供する
pub struct ReversiRules;

impl ReversiRules {
    /// 指定した位置に現在のプレイヤーが置けるかチェックする
    /// 空のマスで、かつ相手の石を少なくとも1個フリップできる必要がある
    pub fn is_valid_move(board: &Board, position: Position, player: Player) -> bool {
        if !board.is_empty(position) {
            return false;
        }
        
        // 少なくとも1個の石をフリップできるかチェック
        Self::get_flipped_positions(board, position, player).len() > 0
    }
    
    /// 指定した位置に石を置いた場合にフリップされる石の位置を返す
    /// リバーシの核心アルゴリズム：8方向を探索して相手の石をふまんでいる部分を特定
    pub fn get_flipped_positions(board: &Board, position: Position, player: Player) -> Vec<Position> {
        let mut flipped = Vec::new();
        let player_cell = player.to_cell();
        let opponent_cell = player.opposite().to_cell();
        
        // 8方向に向かって探索し、フリップ可能な石を探す
        for &(dr, dc) in &DIRECTIONS {
            let mut line_flipped = Vec::new();
            let mut current_row = position.row as i8 + dr;
            let mut current_col = position.col as i8 + dc;
            
            // この方向に盤面の端まで探索
            while current_row >= 0 && current_row < 8 && current_col >= 0 && current_col < 8 {
                let current_pos = Position {
                    row: current_row as usize,
                    col: current_col as usize,
                };
                
                match board.get_cell(current_pos) {
                    Some(cell) if cell == opponent_cell => {
                        // 相手の石を発見、フリップ候補に追加
                        line_flipped.push(current_pos);
                    }
                    Some(cell) if cell == player_cell => {
                        // 自分の石を発見、この方向のフリップが確定
                        flipped.extend(line_flipped);
                        break;
                    }
                    _ => {
                        // 空マスまたは範囲外、この方向のフリップは無効
                        break;
                    }
                }
                
                current_row += dr;
                current_col += dc;
            }
        }
        
        flipped
    }
    
    /// 指定したプレイヤーの合法手を全て取得する
    /// 盤面全体をスキャンして合法手を探索する
    pub fn get_valid_moves(board: &Board, player: Player) -> Vec<Position> {
        let mut valid_moves = Vec::new();
        
        // 盤面全体をスキャンして合法手を探索
        for row in 0..8 {
            for col in 0..8 {
                if let Some(position) = Position::new(row, col) {
                    if Self::is_valid_move(board, position, player) {
                        valid_moves.push(position);
                    }
                }
            }
        }
        
        valid_moves
    }
    
    /// 指定した位置に手を適用し、盤面を更新する
    /// 戻り値はフリップされた石の位置リスト
    pub fn apply_move(game_state: &mut GameState, position: Position) -> Result<Vec<Position>> {
        if game_state.is_finished() {
            return Err(GameError::GameFinished);
        }
        
        if !Self::is_valid_move(&game_state.board, position, game_state.current_player) {
            return Err(GameError::InvalidMove {
                reason: format!("Position ({}, {}) is not a valid move for {:?}", 
                    position.row, position.col, game_state.current_player)
            });
        }
        
        let flipped_positions = Self::get_flipped_positions(&game_state.board, position, game_state.current_player);
        
        // 新しい石を配置
        game_state.board.set_cell(position, game_state.current_player.to_cell());
        
        // フリップされた石を全て自分の色に変更
        for flip_pos in &flipped_positions {
            game_state.board.set_cell(*flip_pos, game_state.current_player.to_cell());
        }
        
        // 手の履歴に記録
        let game_move = Move::new(game_state.current_player, position, flipped_positions.clone());
        game_state.add_move(game_move);
        
        Ok(flipped_positions)
    }
    
    /// 指定したプレイヤーに合法手があるかチェックする
    /// パス判定に使用される
    pub fn has_valid_moves(board: &Board, player: Player) -> bool {
        Self::get_valid_moves(board, player).len() > 0
    }
    
    /// ゲーム終了判定（両プレイヤーとも合法手がない）
    pub fn is_game_over(board: &Board) -> bool {
        !Self::has_valid_moves(board, Player::Black) && !Self::has_valid_moves(board, Player::White)
    }
    
    /// 最終スコアに基づいて勝者を決定する
    /// 同数の場合はNone（引き分け）を返す
    pub fn determine_winner(board: &Board) -> Option<Player> {
        let (black_count, white_count) = board.count_pieces();
        
        if black_count > white_count {
            Some(Player::Black)
        } else if white_count > black_count {
            Some(Player::White)
        } else {
            None
        }
    }
    
    /// ターン処理とパス判定を管理する
    /// 戻り値: ターンが切り替わったかまたはゲームが終了したか
    pub fn handle_turn(game_state: &mut GameState) -> bool {
        if Self::has_valid_moves(&game_state.board, game_state.current_player) {
            // 現在のプレイヤーに合法手があるのでターン継続
            return false;
        }
        
        // 現在のプレイヤーはパス、相手にターンを渡す
        game_state.switch_player();
        
        if Self::has_valid_moves(&game_state.board, game_state.current_player) {
            // 相手に合法手があるのでゲーム継続
            return true;
        }
        
        // 両プレイヤーとも合法手がないのでゲーム終了
        let winner = Self::determine_winner(&game_state.board);
        game_state.finish(winner);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_move_initial_board() {
        let board = Board::new();
        
        assert!(ReversiRules::is_valid_move(&board, Position::new(2, 3).unwrap(), Player::Black));
        assert!(ReversiRules::is_valid_move(&board, Position::new(3, 2).unwrap(), Player::Black));
        assert!(ReversiRules::is_valid_move(&board, Position::new(4, 5).unwrap(), Player::Black));
        assert!(ReversiRules::is_valid_move(&board, Position::new(5, 4).unwrap(), Player::Black));
        
        assert!(!ReversiRules::is_valid_move(&board, Position::new(0, 0).unwrap(), Player::Black));
        assert!(!ReversiRules::is_valid_move(&board, Position::new(3, 3).unwrap(), Player::Black));
    }

    #[test]
    fn test_get_flipped_positions() {
        let board = Board::new();
        
        let flipped = ReversiRules::get_flipped_positions(&board, Position::new(2, 3).unwrap(), Player::Black);
        assert_eq!(flipped.len(), 1);
        assert!(flipped.contains(&Position::new(3, 3).unwrap()));
    }

    #[test]
    fn test_get_valid_moves_initial() {
        let board = Board::new();
        let valid_moves = ReversiRules::get_valid_moves(&board, Player::Black);
        
        assert_eq!(valid_moves.len(), 4);
        assert!(valid_moves.contains(&Position::new(2, 3).unwrap()));
        assert!(valid_moves.contains(&Position::new(3, 2).unwrap()));
        assert!(valid_moves.contains(&Position::new(4, 5).unwrap()));
        assert!(valid_moves.contains(&Position::new(5, 4).unwrap()));
    }

    #[test]
    fn test_apply_move() {
        let mut game_state = GameState::new();
        let position = Position::new(2, 3).unwrap();
        
        let result = ReversiRules::apply_move(&mut game_state, position);
        assert!(result.is_ok());
        
        let flipped = result.unwrap();
        assert_eq!(flipped.len(), 1);
        
        assert_eq!(game_state.board.get_cell(position), Some(Cell::Black));
        
        assert_eq!(game_state.get_move_count(), 1);
    }

    #[test]
    fn test_apply_invalid_move() {
        let mut game_state = GameState::new();
        let position = Position::new(0, 0).unwrap();
        
        let result = ReversiRules::apply_move(&mut game_state, position);
        assert!(result.is_err());
        
        if let Err(GameError::InvalidMove { reason }) = result {
            assert!(reason.contains("not a valid move"));
        } else {
            panic!("Expected InvalidMove error");
        }
    }

    #[test]
    fn test_apply_move_finished_game() {
        let mut game_state = GameState::new();
        game_state.finish(Some(Player::Black));
        
        let position = Position::new(2, 3).unwrap();
        let result = ReversiRules::apply_move(&mut game_state, position);
        
        assert!(matches!(result, Err(GameError::GameFinished)));
    }

    #[test]
    fn test_has_valid_moves() {
        let board = Board::new();
        
        assert!(ReversiRules::has_valid_moves(&board, Player::Black));
        assert!(ReversiRules::has_valid_moves(&board, Player::White));
    }

    #[test]
    fn test_is_game_over_initial() {
        let board = Board::new();
        assert!(!ReversiRules::is_game_over(&board));
    }

    #[test]
    fn test_determine_winner() {
        let mut board = Board::new();
        
        assert_eq!(ReversiRules::determine_winner(&board), None);
        
        board.set_cell(Position::new(0, 0).unwrap(), Cell::Black);
        assert_eq!(ReversiRules::determine_winner(&board), Some(Player::Black));
        
        board.set_cell(Position::new(0, 1).unwrap(), Cell::White);
        board.set_cell(Position::new(0, 2).unwrap(), Cell::White);
        assert_eq!(ReversiRules::determine_winner(&board), Some(Player::White));
    }

    #[test]
    fn test_handle_turn_with_moves() {
        let mut game_state = GameState::new();
        
        let switched = ReversiRules::handle_turn(&mut game_state);
        assert!(!switched);
        assert_eq!(game_state.current_player, Player::Black);
    }
}