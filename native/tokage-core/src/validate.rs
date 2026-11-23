//! 文書の検証。Web 版 `src/model/validate.ts` の validateDoc / validateAction に対応する。

use crate::model::{Action, Node, ProjectDoc};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub level: Level,
    pub message: String,
    pub node_id: Option<String>,
    pub frame_id: Option<String>,
}

impl ValidationIssue {
    fn error(message: impl Into<String>) -> Self {
        Self { level: Level::Error, message: message.into(), node_id: None, frame_id: None }
    }
    fn warning(message: impl Into<String>) -> Self {
        Self { level: Level::Warning, message: message.into(), node_id: None, frame_id: None }
    }
    fn at(mut self, node_id: &str, frame_id: &str) -> Self {
        self.node_id = Some(node_id.to_string());
        self.frame_id = Some(frame_id.to_string());
        self
    }
}

pub fn find_frame<'a>(doc: &'a ProjectDoc, frame_id: &str) -> Option<&'a crate::model::Frame> {
    doc.frames.iter().find(|f| f.id == frame_id)
}

pub fn find_node<'a>(doc: &'a ProjectDoc, node_id: &str) -> Option<&'a Node> {
    doc.frames.iter().flat_map(|f| f.nodes.iter()).find(|n| n.base().id == node_id)
}

/// Action の参照先が実在するか検証する。
pub fn validate_action(doc: &ProjectDoc, action: &Action) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    match action {
        Action::SetCamera { view_node_id, camera_id } => match find_node(doc, view_node_id) {
            Some(Node::View3d(view)) => {
                if !view.cameras.iter().any(|c| &c.id == camera_id) {
                    issues.push(ValidationIssue::warning(format!(
                        "setCamera のカメラ ({camera_id}) が存在しません"
                    )));
                }
            }
            _ => issues.push(ValidationIssue::warning(format!(
                "setCamera の参照先 3Dビュー ({view_node_id}) が存在しません"
            ))),
        },
        Action::Navigate { frame_id } => {
            if find_frame(doc, frame_id).is_none() {
                issues.push(ValidationIssue::warning(format!(
                    "navigate の遷移先フレーム ({frame_id}) が存在しません"
                )));
            }
        }
        Action::ToggleVisible { node_id } => {
            if find_node(doc, node_id).is_none() {
                issues.push(ValidationIssue::warning(format!(
                    "toggleVisible の対象ノード ({node_id}) が存在しません"
                )));
            }
        }
        Action::OpenUrl { url } => {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                issues.push(ValidationIssue::warning(format!("openUrl の URL が不正です: {url}")));
            }
        }
    }
    issues
}

/// 文書全体の検証。ID の一意性、参照整合性、必須フィールドを見る。
pub fn validate_doc(doc: &ProjectDoc) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if doc.version != 1 {
        issues.push(ValidationIssue::error(format!("未対応の文書バージョンです: {}", doc.version)));
    }
    if doc.frames.is_empty() {
        issues.push(ValidationIssue::error("フレームが1つもありません"));
    }
    if find_frame(doc, &doc.start_frame_id).is_none() {
        issues.push(ValidationIssue::error(format!(
            "開始フレーム ({}) が存在しません",
            doc.start_frame_id
        )));
    }

    fn dup(
        ids: &mut HashSet<String>,
        id: &str,
        what: &str,
        issues: &mut Vec<ValidationIssue>,
    ) {
        if !ids.insert(id.to_string()) {
            issues.push(ValidationIssue::error(format!("{what} の ID が重複しています: {id}")));
        }
    }
    let mut ids: HashSet<String> = HashSet::new();
    dup(&mut ids, &doc.id, "プロジェクト", &mut issues);
    for asset in &doc.assets {
        dup(&mut ids, &asset.id, "アセット", &mut issues);
    }
    for frame in &doc.frames {
        dup(&mut ids, &frame.id, "フレーム", &mut issues);
        if !(frame.width > 0.0) || !(frame.height > 0.0) {
            let mut issue = ValidationIssue::error(format!("フレーム {} のサイズが不正です", frame.name));
            issue.frame_id = Some(frame.id.clone());
            issues.push(issue);
        }
        for node in &frame.nodes {
            let base = node.base();
            dup(&mut ids, &base.id, "ノード", &mut issues);
            if let Node::View3d(view) = node {
                for cam in &view.cameras {
                    dup(&mut ids, &cam.id, "カメラ", &mut issues);
                }
                if !view.cameras.iter().any(|c| c.id == view.active_camera_id) {
                    issues.push(
                        ValidationIssue::warning(format!(
                            "3Dビュー {} のアクティブカメラが存在しません",
                            base.name
                        ))
                        .at(&base.id, &frame.id),
                    );
                }
                if let Some(asset_id) = &view.asset_id {
                    if !doc.assets.iter().any(|a| &a.id == asset_id) {
                        issues.push(
                            ValidationIssue::warning(format!(
                                "3Dビュー {} のアセット ({asset_id}) が存在しません",
                                base.name
                            ))
                            .at(&base.id, &frame.id),
                        );
                    }
                }
            }
            if let Some(actions) = &base.actions {
                for action in actions {
                    for issue in validate_action(doc, action) {
                        issues.push(issue.at(&base.id, &frame.id));
                    }
                }
            }
        }
    }
    issues
}

impl ProjectDoc {
    /// エラー級の問題だけを返す（警告は含まない）。
    pub fn errors(&self) -> Vec<ValidationIssue> {
        validate_doc(self).into_iter().filter(|i| i.level == Level::Error).collect()
    }
    pub fn is_valid(&self) -> bool {
        self.errors().is_empty()
    }
}
