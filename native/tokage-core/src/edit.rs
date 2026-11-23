//! 文書の編集操作（純粋なデータ操作）。Web 版 `src/model/mutate.ts` の考え方に対応し、
//! アプリ側はこれらを履歴（History）と組み合わせて使う。

use crate::factory::{create_frame, create_node, NewNodeKind};
use crate::model::{Frame, Node, ProjectDoc};

pub fn frame_mut<'a>(doc: &'a mut ProjectDoc, frame_id: &str) -> Option<&'a mut Frame> {
    doc.frames.iter_mut().find(|f| f.id == frame_id)
}

pub fn node_mut<'a>(doc: &'a mut ProjectDoc, frame_id: &str, node_id: &str) -> Option<&'a mut Node> {
    frame_mut(doc, frame_id)?
        .nodes
        .iter_mut()
        .find(|n| n.base().id == node_id)
}

/// フレームに既定ノードを追加し、新しいノード ID を返す。
pub fn add_node(doc: &mut ProjectDoc, frame_id: &str, kind: NewNodeKind, x: f64, y: f64) -> Option<String> {
    let node = create_node(kind, x, y);
    let id = node.base().id.clone();
    frame_mut(doc, frame_id)?.nodes.push(node);
    Some(id)
}

/// ノードを複製する（新しい ID、+16px オフセット、元の直後に挿入）。
/// カメラやアクションなどノード内部の ID・参照は元のまま共有する（Web 版の複製と同じ）。
pub fn duplicate_node(doc: &mut ProjectDoc, frame_id: &str, node_id: &str) -> Option<String> {
    let frame = frame_mut(doc, frame_id)?;
    let idx = frame.nodes.iter().position(|n| n.base().id == node_id)?;
    let mut copy = frame.nodes[idx].clone();
    let new_id = crate::factory::new_id("node");
    let b = copy.base_mut();
    b.id = new_id.clone();
    b.x += 16.0;
    b.y += 16.0;
    frame.nodes.insert(idx + 1, copy);
    Some(new_id)
}

/// ノードを削除する。削除できたら true。
pub fn remove_node(doc: &mut ProjectDoc, frame_id: &str, node_id: &str) -> bool {
    let Some(frame) = frame_mut(doc, frame_id) else {
        return false;
    };
    let before = frame.nodes.len();
    frame.nodes.retain(|n| n.base().id != node_id);
    frame.nodes.len() != before
}

/// ノードの描画順を1つ前/後ろへ（delta: -1=背面へ, +1=前面へ）。Web 版 moveNodeLayer に対応。
pub fn move_node_layer(doc: &mut ProjectDoc, frame_id: &str, node_id: &str, delta: i32) -> bool {
    let Some(frame) = frame_mut(doc, frame_id) else {
        return false;
    };
    let Some(idx) = frame.nodes.iter().position(|n| n.base().id == node_id) else {
        return false;
    };
    let target = idx as i32 + delta;
    if target < 0 || target as usize >= frame.nodes.len() {
        return false;
    }
    frame.nodes.swap(idx, target as usize);
    true
}

/// フレームを追加し、その ID を返す。
pub fn add_frame(doc: &mut ProjectDoc, name: &str) -> String {
    let frame = create_frame(name);
    let id = frame.id.clone();
    doc.frames.push(frame);
    id
}

/// フレームを削除する。最後の1つは削除しない。
/// 開始フレームを消した場合は先頭フレームを開始フレームにする。
pub fn remove_frame(doc: &mut ProjectDoc, frame_id: &str) -> bool {
    if doc.frames.len() <= 1 {
        return false;
    }
    let before = doc.frames.len();
    doc.frames.retain(|f| f.id != frame_id);
    if doc.frames.len() == before {
        return false;
    }
    if doc.start_frame_id == frame_id {
        doc.start_frame_id = doc.frames[0].id.clone();
    }
    true
}

/// 開始フレームを設定する（存在しない ID は無視）。
pub fn set_start_frame(doc: &mut ProjectDoc, frame_id: &str) -> bool {
    if doc.frames.iter().any(|f| f.id == frame_id) {
        doc.start_frame_id = frame_id.to_string();
        true
    } else {
        false
    }
}
