//! 文書モデル（docs/SPEC.md「文書モデル」の Rust 実装）。
//!
//! Web 版（TypeScript）の `src/model/types.ts` と同一の JSON 構造を持つ。
//! フィールド名は camelCase でシリアライズし、Web 版の doc.json をそのまま読み書きできる。
//! 任意フィールドは Web 版と同様、未設定（undefined）のとき JSON から省略される。

use serde::{Deserialize, Serialize};

/// JavaScript の JSON.stringify と同じ数値表記にするためのシリアライザ。
/// 整数値の f64 は `844` のように小数点なしで出す（Rust 標準では `844.0` になる）。
/// Web 版が読む分にはどちらでも同じ数値だが、doc.json の見た目と差分を Web 版に揃える。
pub(crate) mod js_num {
    use serde::{Serialize, Serializer};

    struct JsNum(f64);

    impl Serialize for JsNum {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let v = self.0;
            if v.is_finite() && v.fract() == 0.0 && v.abs() <= 9_007_199_254_740_992.0 {
                s.serialize_i64(v as i64)
            } else {
                s.serialize_f64(v)
            }
        }
    }

    pub fn ser<S: Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
        JsNum(*v).serialize(s)
    }

    pub fn ser_opt<S: Serializer>(v: &Option<f64>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(x) => JsNum(*x).serialize(s),
            None => s.serialize_none(),
        }
    }

    pub fn ser_arr3<S: Serializer>(v: &[f64; 3], s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = s.serialize_seq(Some(3))?;
        for x in v {
            seq.serialize_element(&JsNum(*x))?;
        }
        seq.end()
    }
}

/// プロジェクト文書（バージョン付き）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDoc {
    pub version: u32,
    pub id: String,
    pub name: String,
    pub frames: Vec<Frame>,
    pub assets: Vec<AssetRef>,
    pub start_frame_id: String,
}

/// 画面（フレーム）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub id: String,
    pub name: String,
    #[serde(serialize_with = "js_num::ser")]
    pub width: f64,
    #[serde(serialize_with = "js_num::ser")]
    pub height: f64,
    pub background: String,
    /// 描画順 = 配列順（後ろが手前）
    pub nodes: Vec<Node>,
}

/// ノード共通フィールド。各バリアントに平坦に埋め込まれる（Web 版の NodeBase 相当）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeBase {
    pub id: String,
    pub name: String,
    #[serde(serialize_with = "js_num::ser")]
    pub x: f64,
    #[serde(serialize_with = "js_num::ser")]
    pub y: f64,
    #[serde(serialize_with = "js_num::ser")]
    pub width: f64,
    #[serde(serialize_with = "js_num::ser")]
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "js_num::ser_opt")]
    pub rotation: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "js_num::ser_opt")]
    pub opacity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,
}

impl NodeBase {
    pub fn visible_or_default(&self) -> bool {
        self.visible.unwrap_or(true)
    }
    pub fn opacity_or_default(&self) -> f64 {
        self.opacity.unwrap_or(1.0)
    }
    pub fn rotation_or_default(&self) -> f64 {
        self.rotation.unwrap_or(0.0)
    }
}

/// ノード。JSON では "type" タグで判別する（Web 版の判別共用体と同一）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Node {
    Rect(RectNode),
    Text(TextNode),
    Button(ButtonNode),
    View3d(View3DNode),
}

impl Node {
    pub fn base(&self) -> &NodeBase {
        match self {
            Node::Rect(n) => &n.base,
            Node::Text(n) => &n.base,
            Node::Button(n) => &n.base,
            Node::View3d(n) => &n.base,
        }
    }
    pub fn kind_name(&self) -> &'static str {
        match self {
            Node::Rect(_) => "rect",
            Node::Text(_) => "text",
            Node::Button(_) => "button",
            Node::View3d(_) => "view3d",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RectNode {
    #[serde(flatten)]
    pub base: NodeBase,
    pub fill: String,
    #[serde(serialize_with = "js_num::ser")]
    pub corner_radius: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "js_num::ser_opt")]
    pub stroke_width: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextNode {
    #[serde(flatten)]
    pub base: NodeBase,
    pub text: String,
    #[serde(serialize_with = "js_num::ser")]
    pub font_size: f64,
    pub fill: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<TextAlign>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonNode {
    #[serde(flatten)]
    pub base: NodeBase,
    pub label: String,
    pub fill: String,
    pub text_color: String,
    #[serde(serialize_with = "js_num::ser")]
    pub corner_radius: f64,
    #[serde(serialize_with = "js_num::ser")]
    pub font_size: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct View3DNode {
    #[serde(flatten)]
    pub base: NodeBase,
    pub asset_id: Option<String>,
    pub active_camera_id: String,
    pub cameras: Vec<CameraDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_rotate: Option<bool>,
}

/// 名前付きカメラプリセット。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraDef {
    pub id: String,
    pub name: String,
    #[serde(serialize_with = "js_num::ser_arr3")]
    pub position: [f64; 3],
    #[serde(serialize_with = "js_num::ser_arr3")]
    pub target: [f64; 3],
}

/// アセット参照。実体（GLB バイナリ）は文書の外（Web 版では IndexedDB）に置く。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetRef {
    pub id: String,
    pub name: String,
    pub kind: AssetKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetKind {
    #[serde(rename = "glb")]
    Glb,
}

/// クリック時の操作。JSON では "type" タグで判別する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Action {
    SetCamera { view_node_id: String, camera_id: String },
    Navigate { frame_id: String },
    ToggleVisible { node_id: String },
    OpenUrl { url: String },
}
