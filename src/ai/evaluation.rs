//! AIの盤面評価システム
//! リバーシのAIが盤面の優劣を判定するための評価関数を提供する。
//! 石数、コーナー制御、エッジ制御などの要素で評価する。

use crate::game::{Board, Player, Position};

/// 評価関数の重み係数を管理する構造体
/// 各評価要素の重要度を調整してAIの戦略を変更できる
#[derive(Debug, Clone)]
pub struct EvalWeights {
    /// 石数の重み
    pub piece_count: f32,
    /// コーナー制御の重み（通常最も重要）
    pub corner_control: f32,
    /// エッジ制御の重み
    pub edge_control: f32,
    /// 可動性（合法手数）の重み
    pub mobility: f32,
}

impl Default for EvalWeights {
    /// バランスの取れたデフォルト重み係数
    /// コーナー制御を最重要視する設定
    fn default() -> Self {
        Self {
            piece_count: 1.0,
            corner_control: 10.0,
            edge_control: 5.0,
            mobility: 3.0,
        }
    }
}

/// 盤面評価を行うスタティックメソッド集
pub struct BoardEvaluator;

impl BoardEvaluator {
    /// 指定したプレイヤーにとっての盤面の総合評価値を計算する
    /// 正の値が有利、負の値が不利を表す
    pub fn evaluate_position(board: &Board, player: Player, weights: &EvalWeights) -> f32 {
        // 各評価要素を計算して重み付きで結合
        let piece_score = Self::evaluate_piece_count(board, player) * weights.piece_count;
        let corner_score = Self::evaluate_corner_control(board, player) * weights.corner_control;
        let edge_score = Self::evaluate_edge_control(board, player) * weights.edge_control;
        
        piece_score + corner_score + edge_score
    }
    
    /// 石数に基づく評価
    /// 自分の石数 - 相手の石数で計算
    pub fn evaluate_piece_count(board: &Board, player: Player) -> f32 {
        let (black_count, white_count) = board.count_pieces();
        
        match player {
            Player::Black => (black_count as f32) - (white_count as f32),
            Player::White => (white_count as f32) - (black_count as f32),
        }
    }
    
    /// コーナー制御の評価
    /// コーナーは取られると絶対にひっくり返されないため極めて重要
    pub fn evaluate_corner_control(board: &Board, player: Player) -> f32 {
        // 4つのコーナー位置を定義
        let corners = [
            Position::new(0, 0).unwrap(),   // 左上
            Position::new(0, 7).unwrap(),   // 右上
            Position::new(7, 0).unwrap(),   // 左下
            Position::new(7, 7).unwrap(),   // 右下
        ];
        
        let player_cell = player.to_cell();
        let opponent_cell = player.opposite().to_cell();
        
        // 各コーナーをチョックしてスコアを計算
        let mut score = 0.0;
        for corner in &corners {
            match board.get_cell(*corner) {
                Some(cell) if cell == player_cell => score += 1.0,
                Some(cell) if cell == opponent_cell => score -= 1.0,
                _ => {}
            }
        }
        
        score
    }
    
    /// エッジ制御の評価
    /// 盤面の端に近い位置は安定しているため有利
    pub fn evaluate_edge_control(board: &Board, player: Player) -> f32 {
        let player_cell = player.to_cell();
        let opponent_cell = player.opposite().to_cell();
        
        let mut score = 0.0;
        
        // 上下のエッジをチェック
        for col in 0..8 {
            for &row in &[0, 7] {
                if let Some(position) = Position::new(row, col) {
                    match board.get_cell(position) {
                        Some(cell) if cell == player_cell => score += 0.5,
                        Some(cell) if cell == opponent_cell => score -= 0.5,
                        _ => {}
                    }
                }
            }
        }
        
        // 左右のエッジをチェック（コーナー除く）
        for row in 1..7 {
            for &col in &[0, 7] {
                if let Some(position) = Position::new(row, col) {
                    match board.get_cell(position) {
                        Some(cell) if cell == player_cell => score += 0.5,
                        Some(cell) if cell == opponent_cell => score -= 0.5,
                        _ => {}
                    }
                }
            }
        }
        
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Board, Cell};

    #[test]
    fn test_eval_weights_default() {
        let weights = EvalWeights::default();
        assert_eq!(weights.piece_count, 1.0);
        assert_eq!(weights.corner_control, 10.0);
        assert_eq!(weights.edge_control, 5.0);
        assert_eq!(weights.mobility, 3.0);
    }

    #[test]
    fn test_evaluate_piece_count_initial() {
        let board = Board::new();
        
        let black_score = BoardEvaluator::evaluate_piece_count(&board, Player::Black);
        let white_score = BoardEvaluator::evaluate_piece_count(&board, Player::White);
        
        assert_eq!(black_score, 0.0);
        assert_eq!(white_score, 0.0);
    }

    #[test]
    fn test_evaluate_piece_count_advantage() {
        let mut board = Board::new();
        board.set_cell(Position::new(0, 0).unwrap(), Cell::Black);
        
        let black_score = BoardEvaluator::evaluate_piece_count(&board, Player::Black);
        let white_score = BoardEvaluator::evaluate_piece_count(&board, Player::White);
        
        assert_eq!(black_score, 1.0);
        assert_eq!(white_score, -1.0);
    }

    #[test]
    fn test_evaluate_corner_control_empty() {
        let board = Board::new();
        
        let black_score = BoardEvaluator::evaluate_corner_control(&board, Player::Black);
        let white_score = BoardEvaluator::evaluate_corner_control(&board, Player::White);
        
        assert_eq!(black_score, 0.0);
        assert_eq!(white_score, 0.0);
    }

    #[test]
    fn test_evaluate_corner_control_with_pieces() {
        let mut board = Board::new();
        
        board.set_cell(Position::new(0, 0).unwrap(), Cell::Black);
        board.set_cell(Position::new(7, 7).unwrap(), Cell::Black);
        
        board.set_cell(Position::new(0, 7).unwrap(), Cell::White);
        
        let black_score = BoardEvaluator::evaluate_corner_control(&board, Player::Black);
        let white_score = BoardEvaluator::evaluate_corner_control(&board, Player::White);
        
        assert_eq!(black_score, 1.0);
        assert_eq!(white_score, -1.0);
    }

    #[test]
    fn test_evaluate_edge_control() {
        let mut board = Board::new();
        
        board.set_cell(Position::new(0, 1).unwrap(), Cell::Black);
        board.set_cell(Position::new(1, 0).unwrap(), Cell::Black);
        
        board.set_cell(Position::new(0, 2).unwrap(), Cell::White);
        
        let black_score = BoardEvaluator::evaluate_edge_control(&board, Player::Black);
        let white_score = BoardEvaluator::evaluate_edge_control(&board, Player::White);
        
        assert_eq!(black_score, 0.5);
        assert_eq!(white_score, -0.5);
    }

    #[test]
    fn test_evaluate_position_comprehensive() {
        let mut board = Board::new();
        let weights = EvalWeights::default();
        
        board.set_cell(Position::new(0, 0).unwrap(), Cell::Black);
        board.set_cell(Position::new(0, 1).unwrap(), Cell::Black);
        board.set_cell(Position::new(1, 1).unwrap(), Cell::Black);
        
        let score = BoardEvaluator::evaluate_position(&board, Player::Black, &weights);
        
        let expected = 3.0 * weights.piece_count +
                      1.0 * weights.corner_control +
                      1.0 * weights.edge_control;
        
        assert_eq!(score, expected);
    }
}