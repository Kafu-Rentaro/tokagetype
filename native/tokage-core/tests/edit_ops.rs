//! 編集操作（edit / factory / history）のテスト。

use tokage_core::edit;
use tokage_core::factory::NewNodeKind;
use tokage_core::{validate_doc, History, Level, ProjectDoc};

const FIXTURE: &str = include_str!("fixtures/sample-project.json");

fn load() -> ProjectDoc {
    ProjectDoc::from_json(FIXTURE).unwrap()
}

#[test]
fn add_and_remove_node() {
    let mut doc = load();
    let frame_id = doc.frames[0].id.clone();
    let before = doc.frames[0].nodes.len();
    let id = edit::add_node(&mut doc, &frame_id, NewNodeKind::Rect, 10.0, 20.0).unwrap();
    assert_eq!(doc.frames[0].nodes.len(), before + 1);
    let node = doc.frames[0].nodes.last().unwrap();
    assert_eq!(node.base().id, id);
    assert_eq!(node.kind_name(), "rect");
    assert_eq!((node.base().x, node.base().y), (10.0, 20.0));
    // 追加後も文書が検証を通る
    assert!(validate_doc(&doc).iter().all(|i| i.level != Level::Error));

    assert!(edit::remove_node(&mut doc, &frame_id, &id));
    assert_eq!(doc.frames[0].nodes.len(), before);
    assert!(!edit::remove_node(&mut doc, &frame_id, &id), "二重削除は false");
}

#[test]
fn duplicate_node_gets_new_id_and_offset() {
    let mut doc = load();
    let frame_id = doc.frames[0].id.clone();
    let orig = doc.frames[0].nodes[0].clone();
    let new_id = edit::duplicate_node(&mut doc, &frame_id, &orig.base().id).unwrap();
    assert_ne!(new_id, orig.base().id);
    let pos = doc.frames[0]
        .nodes
        .iter()
        .position(|n| n.base().id == new_id)
        .unwrap();
    let orig_pos = doc.frames[0]
        .nodes
        .iter()
        .position(|n| n.base().id == orig.base().id)
        .unwrap();
    assert_eq!(pos, orig_pos + 1, "元の直後に挿入される");
    let copy = &doc.frames[0].nodes[pos];
    assert_eq!(copy.base().x, orig.base().x + 16.0);
    assert_eq!(copy.base().y, orig.base().y + 16.0);
    // ID 重複なしで検証を通る
    assert!(validate_doc(&doc).iter().all(|i| i.level != Level::Error));
}

#[test]
fn frame_add_remove_and_start_frame() {
    let mut doc = load();
    let start = doc.start_frame_id.clone();
    let new_frame = edit::add_frame(&mut doc, "追加フレーム");
    assert_eq!(doc.frames.len(), 3);
    assert!(edit::set_start_frame(&mut doc, &new_frame));
    assert_eq!(doc.start_frame_id, new_frame);
    // 開始フレームを削除すると先頭が開始フレームになる
    assert!(edit::remove_frame(&mut doc, &new_frame));
    assert_eq!(doc.frames.len(), 2);
    assert_ne!(doc.start_frame_id, new_frame);
    // 残り2つ→1つまで消せるが、最後の1つは消せない
    let f0 = doc.frames[0].id.clone();
    assert!(edit::remove_frame(&mut doc, &f0));
    let last = doc.frames[0].id.clone();
    assert!(!edit::remove_frame(&mut doc, &last), "最後のフレームは削除不可");
    assert_eq!(doc.frames.len(), 1);
    let _ = start;
}

#[test]
fn history_undo_redo_round_trip() {
    let doc = load();
    let mut history = History::new(doc.clone());
    assert!(!history.can_undo());
    assert!(!history.can_redo());

    // 離散変更を push
    let mut next = doc.clone();
    let frame_id = next.frames[0].id.clone();
    edit::add_node(&mut next, &frame_id, NewNodeKind::Button, 0.0, 0.0);
    history.push(next.clone());
    assert!(history.can_undo());
    assert_eq!(history.present(), &next);

    assert!(history.undo());
    assert_eq!(history.present(), &doc, "undo で元の文書に戻る");
    assert!(history.can_redo());

    assert!(history.redo());
    assert_eq!(history.present(), &next, "redo で変更が戻る");
    assert!(!history.can_redo());

    // 同一文書の push は無視される
    let mut h2 = History::new(doc.clone());
    h2.push(doc.clone());
    assert!(!h2.can_undo());
}

#[test]
fn history_record_for_continuous_edit() {
    let doc = load();
    let mut history = History::new(doc.clone());
    let base = history.present().clone();
    // ドラッグ相当: present を直接書き換えてから record
    let frame_id = base.frames[0].id.clone();
    let node_id = base.frames[0].nodes[0].base().id.clone();
    {
        let node = edit::node_mut(history.present_mut(), &frame_id, &node_id).unwrap();
        node.base_mut().x += 50.0;
    }
    history.record(base.clone());
    assert!(history.can_undo());
    assert!(history.undo());
    assert_eq!(history.present(), &base);
    // 変化なしの record は履歴を増やさない
    let mut h2 = History::new(doc.clone());
    let b2 = h2.present().clone();
    h2.record(b2);
    assert!(!h2.can_undo());
}
