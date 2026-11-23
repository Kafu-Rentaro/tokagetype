//! Web 版サンプルプロジェクトの doc.json（実物フィクスチャ）との互換テスト。
//! フィクスチャは Web 版アプリが IndexedDB に保存したものをそのまま抽出した。

use tokage_core::{validate_doc, Action, Level, Node, ProjectDoc};

const FIXTURE: &str = include_str!("fixtures/sample-project.json");

#[test]
fn web_sample_fixture_parses() {
    let doc = ProjectDoc::from_json(FIXTURE).expect("Web 版サンプル doc.json が読めること");
    assert_eq!(doc.version, 1);
    assert!(!doc.frames.is_empty(), "フレームがある");
    assert!(!doc.assets.is_empty(), "GLB アセット参照がある");
    assert!(
        doc.frames.iter().any(|f| f.id == doc.start_frame_id),
        "開始フレームが実在する"
    );
}

#[test]
fn web_sample_fixture_validates_clean() {
    let doc = ProjectDoc::from_json(FIXTURE).unwrap();
    let issues = validate_doc(&doc);
    let errors: Vec<_> = issues.iter().filter(|i| i.level == Level::Error).collect();
    assert!(errors.is_empty(), "エラー級の問題がないこと: {errors:?}");
    assert!(issues.is_empty(), "警告もないこと: {issues:?}");
}

#[test]
fn roundtrip_preserves_document() {
    let doc = ProjectDoc::from_json(FIXTURE).unwrap();
    let json = doc.to_json().unwrap();
    let doc2 = ProjectDoc::from_json(&json).unwrap();
    assert_eq!(doc, doc2, "読み込み→再シリアライズ→再読み込みで文書が一致する");

    // JSON 値としても一致する（フィールドの欠落・余分がない）
    let v1: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    let v2: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v1, v2, "再シリアライズ結果が元の JSON と値として一致する");
}

#[test]
fn node_and_action_kinds_survive() {
    let doc = ProjectDoc::from_json(FIXTURE).unwrap();
    let kinds: Vec<&str> = doc
        .frames
        .iter()
        .flat_map(|f| f.nodes.iter().map(|n| n.kind_name()))
        .collect();
    for expected in ["text", "view3d", "button"] {
        assert!(kinds.contains(&expected), "ノード種別 {expected} を含む: {kinds:?}");
    }
    // ボタンの setCamera アクションが型付きで読める
    let has_set_camera = doc.frames.iter().flat_map(|f| f.nodes.iter()).any(|n| {
        n.base()
            .actions
            .as_ref()
            .map(|acts| acts.iter().any(|a| matches!(a, Action::SetCamera { .. })))
            .unwrap_or(false)
    });
    assert!(has_set_camera, "setCamera アクションを含む");
    // view3d のカメラプリセットが配列タプルとして読める
    let view = doc
        .frames
        .iter()
        .flat_map(|f| f.nodes.iter())
        .find_map(|n| match n {
            Node::View3d(v) => Some(v),
            _ => None,
        })
        .expect("view3d ノードがある");
    assert_eq!(view.cameras.len(), 2);
    assert_eq!(view.cameras[0].position, [0.0, 0.8, 3.2]);
}

#[test]
fn rejects_unsupported_version() {
    let mut v: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    v["version"] = serde_json::json!(2);
    let err = ProjectDoc::from_json(&v.to_string()).unwrap_err();
    assert!(err.to_string().contains("バージョン"), "エラー内容: {err}");
}

#[test]
fn validation_detects_duplicate_ids_and_bad_refs() {
    let mut v: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    // 2つ目のフレームの ID を 1つ目と重複させる
    let first_id = v["frames"][0]["id"].clone();
    if v["frames"].as_array().unwrap().len() > 1 {
        v["frames"][1]["id"] = first_id;
    }
    // 開始フレームを存在しない ID にする
    v["startFrameId"] = serde_json::json!("frame_missing");
    let doc = ProjectDoc::from_json(&v.to_string()).unwrap();
    let issues = validate_doc(&doc);
    assert!(
        issues.iter().any(|i| i.level == Level::Error && i.message.contains("開始フレーム")),
        "開始フレーム不在を検出する: {issues:?}"
    );
    if v["frames"].as_array().unwrap().len() > 1 {
        assert!(
            issues.iter().any(|i| i.level == Level::Error && i.message.contains("重複")),
            "ID 重複を検出する: {issues:?}"
        );
    }
}
