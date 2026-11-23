//! .tokage（zip）の往復テスト。Web 版 src/lib/tokage.ts と同一レイアウトであることを確認する。

use std::collections::HashMap;
use tokage_core::{read_tokage, validate_doc, write_tokage, Level, ProjectDoc};

const FIXTURE: &str = include_str!("fixtures/sample-project.json");

#[test]
fn tokage_roundtrip_preserves_doc_and_assets() {
    let doc = ProjectDoc::from_json(FIXTURE).unwrap();
    let asset_id = doc.assets[0].id.clone();
    // GLB らしいダミーバイナリ（"glTF" マジック + 適当なバイト列）
    let mut glb = b"glTF".to_vec();
    glb.extend_from_slice(&[2, 0, 0, 0, 100, 0, 0, 0]);
    glb.extend((0u8..=255).cycle().take(1024));
    let mut assets = HashMap::new();
    assets.insert(asset_id.clone(), glb.clone());

    let bytes = write_tokage(&doc, &assets).unwrap();
    assert_eq!(&bytes[0..2], b"PK", "zip 形式である");

    let (doc2, assets2) = read_tokage(&bytes).unwrap();
    assert_eq!(doc, doc2, "文書が往復で一致する");
    assert_eq!(assets2.get(&asset_id), Some(&glb), "アセット実体が往復で一致する");
    assert!(validate_doc(&doc2).iter().all(|i| i.level != Level::Error));
}

#[test]
fn tokage_layout_matches_web_version() {
    // Web 版 importTokage が読めるよう、エントリ名が doc.json / assets/<id>.glb であること
    let doc = ProjectDoc::from_json(FIXTURE).unwrap();
    let asset_id = doc.assets[0].id.clone();
    let mut assets = HashMap::new();
    assets.insert(asset_id.clone(), vec![1, 2, 3]);
    let bytes = write_tokage(&doc, &assets).unwrap();
    let zr = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let names: Vec<&str> = zr.file_names().collect();
    assert!(names.contains(&"doc.json"), "doc.json を含む: {names:?}");
    let expect = format!("assets/{asset_id}.glb");
    assert!(names.contains(&expect.as_str()), "アセットエントリを含む: {names:?}");
}

#[test]
fn tokage_without_doc_json_is_rejected() {
    // doc.json の無い zip はエラー
    let mut buf = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts = zip::write::SimpleFileOptions::default();
        use std::io::Write;
        zw.start_file("other.txt", opts).unwrap();
        zw.write_all(b"hello").unwrap();
        zw.finish().unwrap();
    }
    let err = read_tokage(&buf).unwrap_err();
    assert!(err.to_string().contains("doc.json"), "エラー内容: {err}");
}

#[test]
fn tokage_missing_asset_blob_is_tolerated() {
    // アセット実体が zip に無い場合も、文書だけは読める（Web 版と同じ）
    let doc = ProjectDoc::from_json(FIXTURE).unwrap();
    let bytes = write_tokage(&doc, &HashMap::new()).unwrap();
    let (doc2, assets2) = read_tokage(&bytes).unwrap();
    assert_eq!(doc, doc2);
    assert!(assets2.is_empty());
}
