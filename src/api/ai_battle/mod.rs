//! AI対戦API モジュール
//! 
//! 既存のリバーシシステムを拡張し、AI相手との対戦機能をWebAPI経由で提供する。

pub mod dto;
pub mod service;
pub mod handlers;
pub mod routes;
pub mod config_service;

pub use dto::*;
pub use service::*;
pub use handlers::*;
pub use routes::*;
pub use config_service::*;