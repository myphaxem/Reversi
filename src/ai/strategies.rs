//! AI戦略の実装モジュール
//! 異なるAI戦略（ランダム、ミニマックス、αβ法など）を定義し、
//! 統一されたインターフェースで提供する。

use crate::game::{GameState, Position, Player, ReversiRules};
use crate::error::{AIError, Result as GameResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// AIの難易度を表すenum
/// 異なる戦略や探索深度に対応する
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    /// 初心者レベル（ランダム戦略）
    Beginner,
    /// 中級者レベル（ミニマックス法）
    Intermediate,
    /// 上級者レベル（αβ法）
    Advanced,
}

/// AI戦略の共通インターフェース
/// 異なるAI実装を統一して扱うためのtrait
pub trait AIStrategy: Send + Sync {
    /// ゲーム状態から最適な手を計算する
    fn calculate_move(&self, game_state: &GameState) -> Result<Position, AIError>;
    /// このAIの難易度を返す
    fn get_difficulty(&self) -> Difficulty;
    /// AIの名前を返す
    fn get_name(&self) -> &'static str;
}

/// ランダムに手を選択するAI実装
/// 初心者レベルで、合法手の中からランダムに選ぶ
#[derive(Debug, Clone)]
pub struct RandomAI;

impl RandomAI {
    /// 新しいRandomAIインスタンスを作成する
    pub fn new() -> Self {
        RandomAI
    }
}

impl Default for RandomAI {
    fn default() -> Self {
        Self::new()
    }
}

impl AIStrategy for RandomAI {
    /// 合法手の中から擬似ランダムで選択する
    /// 真の乱数の代わりに手数ベースの決定的アルゴリズムを使用
    fn calculate_move(&self, game_state: &GameState) -> Result<Position, AIError> {
        if game_state.is_finished() {
            return Err(AIError::StrategyError {
                message: "Cannot calculate move for finished game".to_string(),
            });
        }
        
        let valid_moves = ReversiRules::get_valid_moves(&game_state.board, game_state.current_player);
        
        if valid_moves.is_empty() {
            return Err(AIError::NoValidMoves);
        }
        
        // 手数とプレイヤー情報から擬似ランダムなインデックスを生成
        let index = (game_state.get_move_count() * 7 + 
                    game_state.current_player as usize * 3) % valid_moves.len();
        
        Ok(valid_moves[index])
    }
    
    fn get_difficulty(&self) -> Difficulty {
        Difficulty::Beginner
    }
    
    fn get_name(&self) -> &'static str {
        "RandomAI"
    }
}

/// ミニマックス法を使用するAI実装（未実装）
/// 指定した深度までゲームツリーを探索して最適手を見つける
#[derive(Debug, Clone)]
pub struct MinimaxAI {
    /// 探索深度（手数）
    pub depth: u8,
}

impl MinimaxAI {
    /// 指定した探索深度で新しいMinimaxAIを作成する
    pub fn new(depth: u8) -> Self {
        MinimaxAI { depth }
    }
}

impl AIStrategy for MinimaxAI {
    /// ミニマックス法で最適手を計算する（未実装）
    fn calculate_move(&self, _game_state: &GameState) -> Result<Position, AIError> {
        Err(AIError::StrategyError {
            message: "MinimaxAI not yet implemented".to_string(),
        })
    }
    
    fn get_difficulty(&self) -> Difficulty {
        Difficulty::Intermediate
    }
    
    fn get_name(&self) -> &'static str {
        "MinimaxAI"
    }
}

/// αβ法（アルファベータ法）を使用するAI実装（未実装）
/// ミニマックス法に枝刈りを追加して高速化したAI
#[derive(Debug, Clone)]
pub struct AlphaBetaAI {
    /// 探索深度（手数）
    pub depth: u8,
}

impl AlphaBetaAI {
    /// 指定した探索深度で新しいAlphaBetaAIを作成する
    pub fn new(depth: u8) -> Self {
        AlphaBetaAI { depth }
    }
}

impl AIStrategy for AlphaBetaAI {
    /// αβ法で最適手を計算する（未実装）
    fn calculate_move(&self, _game_state: &GameState) -> Result<Position, AIError> {
        Err(AIError::StrategyError {
            message: "AlphaBetaAI not yet implemented".to_string(),
        })
    }
    
    fn get_difficulty(&self) -> Difficulty {
        Difficulty::Advanced
    }
    
    fn get_name(&self) -> &'static str {
        "AlphaBetaAI"
    }
}

/// 難易度に応じたAI戦略を生成するファクトリ関数
/// 難易度に応じて適切なAI実装を選択して返す
pub fn create_ai_strategy(difficulty: Difficulty) -> Box<dyn AIStrategy> {
    match difficulty {
        Difficulty::Beginner => Box::new(RandomAI::new()),
        Difficulty::Intermediate => Box::new(MinimaxAI::new(3)),  // 深度3手
        Difficulty::Advanced => Box::new(AlphaBetaAI::new(5)),     // 深度5手
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::GameState;

    #[test]
    fn test_random_ai_creation() {
        let ai = RandomAI::new();
        assert_eq!(ai.get_name(), "RandomAI");
        assert!(matches!(ai.get_difficulty(), Difficulty::Beginner));
    }

    #[test]
    fn test_random_ai_calculate_move() {
        let game_state = GameState::new();
        let ai = RandomAI::new();
        
        let result = ai.calculate_move(&game_state);
        assert!(result.is_ok());
        
        let position = result.unwrap();
        assert!(position.is_valid());
        
        assert!(ReversiRules::is_valid_move(&game_state.board, position, game_state.current_player));
    }

    #[test]
    fn test_random_ai_finished_game() {
        let mut game_state = GameState::new();
        game_state.finish(Some(Player::Black));
        
        let ai = RandomAI::new();
        let result = ai.calculate_move(&game_state);
        
        assert!(result.is_err());
        if let Err(AIError::StrategyError { message }) = result {
            assert!(message.contains("finished game"));
        } else {
            panic!("Expected StrategyError for finished game");
        }
    }

    #[test]
    fn test_minimax_ai_creation() {
        let ai = MinimaxAI::new(5);
        assert_eq!(ai.depth, 5);
        assert_eq!(ai.get_name(), "MinimaxAI");
        assert!(matches!(ai.get_difficulty(), Difficulty::Intermediate));
    }

    #[test]
    fn test_minimax_ai_not_implemented() {
        let game_state = GameState::new();
        let ai = MinimaxAI::new(3);
        
        let result = ai.calculate_move(&game_state);
        assert!(result.is_err());
        
        if let Err(AIError::StrategyError { message }) = result {
            assert!(message.contains("not yet implemented"));
        } else {
            panic!("Expected StrategyError for unimplemented MinimaxAI");
        }
    }

    #[test]
    fn test_alphabeta_ai_creation() {
        let ai = AlphaBetaAI::new(7);
        assert_eq!(ai.depth, 7);
        assert_eq!(ai.get_name(), "AlphaBetaAI");
        assert!(matches!(ai.get_difficulty(), Difficulty::Advanced));
    }

    #[test]
    fn test_create_ai_strategy_factory() {
        let beginner = create_ai_strategy(Difficulty::Beginner);
        assert_eq!(beginner.get_name(), "RandomAI");
        
        let intermediate = create_ai_strategy(Difficulty::Intermediate);
        assert_eq!(intermediate.get_name(), "MinimaxAI");
        
        let advanced = create_ai_strategy(Difficulty::Advanced);
        assert_eq!(advanced.get_name(), "AlphaBetaAI");
    }
    
    #[test]
    fn test_ai_strategy_trait_object() {
        let ai: Box<dyn AIStrategy> = Box::new(RandomAI::new());
        let game_state = GameState::new();
        
        let result = ai.calculate_move(&game_state);
        assert!(result.is_ok());
    }
}