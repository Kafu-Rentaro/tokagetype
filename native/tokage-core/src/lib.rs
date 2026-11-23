//! tokage-core: tokagetype の文書モデルと検証（Web 版と共有する doc.json 形式）。
//!
//! - [`ProjectDoc`] を中心とする文書モデル（serde、Web 版と同一の JSON 構造）
//! - 読み込み（[`ProjectDoc::from_json`]）→ 検証（[`validate_doc`]）→ 再シリアライズ（[`ProjectDoc::to_json`]）

pub mod archive;
pub mod edit;
pub mod factory;
pub mod history;
pub mod model;
pub mod validate;

pub use archive::{read_tokage, write_tokage, ArchiveError, AssetBlobs};
pub use factory::{create_frame, create_node, create_project, default_action, new_id, NewNodeKind};
pub use history::History;
pub use model::{
    Action, AssetKind, AssetRef, ButtonNode, CameraDef, Frame, Node, NodeBase, ProjectDoc,
    RectNode, TextAlign, TextNode, View3DNode,
};
pub use validate::{validate_action, validate_doc, Level, ValidationIssue};

use std::fmt;

#[derive(Debug)]
pub enum DocError {
    /// JSON として読めない / 構造が合わない
    Parse(serde_json::Error),
    /// 構造は読めたが未対応のバージョンなど意味的に不正
    Invalid(String),
}

impl fmt::Display for DocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocError::Parse(e) => write!(f, "文書の読み込みに失敗しました: {e}"),
            DocError::Invalid(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for DocError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DocError::Parse(e) => Some(e),
            DocError::Invalid(_) => None,
        }
    }
}

impl From<serde_json::Error> for DocError {
    fn from(e: serde_json::Error) -> Self {
        DocError::Parse(e)
    }
}

impl ProjectDoc {
    /// JSON 文字列から文書を復元する。Web 版 deserializeDoc に対応。
    pub fn from_json(json: &str) -> Result<Self, DocError> {
        let doc: ProjectDoc = serde_json::from_str(json)?;
        if doc.version != 1 {
            return Err(DocError::Invalid(format!(
                "未対応の文書バージョンです: {}",
                doc.version
            )));
        }
        Ok(doc)
    }

    /// JSON 文字列化（保存・書き出し用）。Web 版 serializeDoc に対応。
    pub fn to_json(&self) -> Result<String, DocError> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn to_json_pretty(&self) -> Result<String, DocError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}
