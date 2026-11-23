//! ノード/フレームの生成。Web 版 `src/model/factory.ts` に対応する。
//!
//! ID 形式・既定値（サイズ、色、カメラプリセット）は Web 版に揃える。

use crate::model::{
    Action, ButtonNode, CameraDef, Frame, Node, NodeBase, ProjectDoc, RectNode, TextAlign,
    TextNode, View3DNode,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// 文書内で一意な ID を生成する（Web 版 newId に対応）。
/// 形式: `<prefix>_<乱数8桁hex><カウンタ36進>`。
pub fn new_id(prefix: &str) -> String {
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    // 時刻とカウンタから xorshift で乱数を得る（外部クレート不要の範囲で十分に分散させる）
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        ^ (std::process::id() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ count.wrapping_mul(0x2545_F491_4F6C_DD1D);
    let mut x = seed | 1;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    let rand = (x >> 32) as u32;
    format!("{prefix}_{rand:08x}{:x}", count + 1)
}

pub fn front_camera() -> CameraDef {
    CameraDef {
        id: "cam_front".to_string(),
        name: "front".to_string(),
        position: [0.0, 0.8, 3.2],
        target: [0.0, 0.8, 0.0],
    }
}

pub fn back_camera() -> CameraDef {
    CameraDef {
        id: "cam_back".to_string(),
        name: "back".to_string(),
        position: [0.0, 0.8, -3.2],
        target: [0.0, 0.8, 0.0],
    }
}

pub fn default_cameras() -> Vec<CameraDef> {
    vec![front_camera(), back_camera()]
}

/// Web 版 createFrame に対応（既定 390x844、白背景）。
pub fn create_frame(name: &str) -> Frame {
    Frame {
        id: new_id("frame"),
        name: name.to_string(),
        width: 390.0,
        height: 844.0,
        background: "#ffffff".to_string(),
        nodes: Vec::new(),
    }
}

/// Web 版 createProject に対応。
pub fn create_project(name: &str) -> ProjectDoc {
    let frame = create_frame("ホーム");
    let start = frame.id.clone();
    ProjectDoc {
        version: 1,
        id: new_id("proj"),
        name: name.to_string(),
        frames: vec![frame],
        assets: Vec::new(),
        start_frame_id: start,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewNodeKind {
    Rect,
    Text,
    Button,
    View3d,
}

fn base(kind_name: &str, x: f64, y: f64, width: f64, height: f64) -> NodeBase {
    let _ = kind_name;
    NodeBase {
        id: new_id("node"),
        name: String::new(),
        x,
        y,
        width,
        height,
        rotation: Some(0.0),
        opacity: Some(1.0),
        visible: Some(true),
        actions: None,
    }
}

/// Web 版 createNode に対応する既定値でノードを作る。
pub fn create_node(kind: NewNodeKind, x: f64, y: f64) -> Node {
    match kind {
        NewNodeKind::Rect => {
            let mut b = base("rect", x, y, 160.0, 100.0);
            b.name = "矩形".to_string();
            Node::Rect(RectNode {
                base: b,
                fill: "#4f8ff7".to_string(),
                corner_radius: 8.0,
                stroke: None,
                stroke_width: None,
            })
        }
        NewNodeKind::Text => {
            let mut b = base("text", x, y, 220.0, 40.0);
            b.name = "テキスト".to_string();
            Node::Text(TextNode {
                base: b,
                text: "テキスト".to_string(),
                font_size: 20.0,
                fill: "#222222".to_string(),
                font_style: Some("normal".to_string()),
                align: Some(TextAlign::Left),
            })
        }
        NewNodeKind::Button => {
            let mut b = base("button", x, y, 180.0, 48.0);
            b.name = "ボタン".to_string();
            b.actions = Some(Vec::new());
            Node::Button(ButtonNode {
                base: b,
                label: "ボタン".to_string(),
                fill: "#111111".to_string(),
                text_color: "#ffffff".to_string(),
                corner_radius: 24.0,
                font_size: 16.0,
            })
        }
        NewNodeKind::View3d => {
            let mut b = base("view3d", x, y, 310.0, 380.0);
            b.name = "3Dビュー".to_string();
            let cameras = default_cameras();
            let active = cameras[0].id.clone();
            Node::View3d(View3DNode {
                base: b,
                asset_id: None,
                active_camera_id: active,
                cameras,
                auto_rotate: Some(false),
            })
        }
    }
}

/// Web 版 defaultAction に対応する、種別ごとの既定アクション。
pub fn default_action(kind: &str, doc: &ProjectDoc, frame: &Frame) -> Option<Action> {
    match kind {
        "setCamera" => {
            let view = frame
                .nodes
                .iter()
                .find_map(|n| match n {
                    Node::View3d(v) => Some(v),
                    _ => None,
                })
                .or_else(|| {
                    doc.frames.iter().flat_map(|f| f.nodes.iter()).find_map(|n| match n {
                        Node::View3d(v) => Some(v),
                        _ => None,
                    })
                });
            Some(Action::SetCamera {
                view_node_id: view.map(|v| v.base.id.clone()).unwrap_or_default(),
                camera_id: view
                    .and_then(|v| v.cameras.first().map(|c| c.id.clone()))
                    .unwrap_or_default(),
            })
        }
        "navigate" => Some(Action::Navigate {
            frame_id: doc.frames.first().map(|f| f.id.clone()).unwrap_or_default(),
        }),
        "toggleVisible" => Some(Action::ToggleVisible {
            node_id: frame.nodes.first().map(|n| n.base().id.clone()).unwrap_or_default(),
        }),
        "openUrl" => Some(Action::OpenUrl {
            url: "https://example.com".to_string(),
        }),
        _ => None,
    }
}
