//! `.tokage` 形式（zip: doc.json + assets/*.glb）の読み書き。
//! Web 版 `src/lib/tokage.ts` と同一のレイアウトにする。
//!
//! アセット実体はメモリ上のバイト列として保持するだけで、解釈はしない。

use crate::model::ProjectDoc;
use crate::DocError;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

/// アセット ID → GLB バイナリ。
pub type AssetBlobs = HashMap<String, Vec<u8>>;

#[derive(Debug)]
pub enum ArchiveError {
    Zip(zip::result::ZipError),
    Doc(DocError),
    Io(std::io::Error),
    /// doc.json が含まれていない等、形式として不正
    Invalid(String),
}

impl std::fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArchiveError::Zip(e) => write!(f, ".tokage の zip 処理に失敗しました: {e}"),
            ArchiveError::Doc(e) => write!(f, "{e}"),
            ArchiveError::Io(e) => write!(f, "入出力エラー: {e}"),
            ArchiveError::Invalid(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for ArchiveError {}

impl From<zip::result::ZipError> for ArchiveError {
    fn from(e: zip::result::ZipError) -> Self {
        ArchiveError::Zip(e)
    }
}
impl From<DocError> for ArchiveError {
    fn from(e: DocError) -> Self {
        ArchiveError::Doc(e)
    }
}
impl From<std::io::Error> for ArchiveError {
    fn from(e: std::io::Error) -> Self {
        ArchiveError::Io(e)
    }
}

/// 文書とアセット実体を .tokage バイト列に書き出す。
/// doc.assets に無いアセット実体は含めない（Web 版 exportTokage と同じ）。
pub fn write_tokage(doc: &ProjectDoc, assets: &AssetBlobs) -> Result<Vec<u8>, ArchiveError> {
    let mut buf = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(Cursor::new(&mut buf));
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zw.start_file("doc.json", opts)?;
        zw.write_all(doc.to_json()?.as_bytes())?;
        for asset in &doc.assets {
            if let Some(bytes) = assets.get(&asset.id) {
                zw.start_file(format!("assets/{}.glb", asset.id), opts)?;
                zw.write_all(bytes)?;
            }
        }
        zw.finish()?;
    }
    Ok(buf)
}

/// .tokage バイト列を読み、文書とアセット実体を返す。
/// doc.assets に無い zip 内エントリは無視する（Web 版 importTokage と同じ）。
pub fn read_tokage(bytes: &[u8]) -> Result<(ProjectDoc, AssetBlobs), ArchiveError> {
    let mut zr = zip::ZipArchive::new(Cursor::new(bytes))?;
    let doc = {
        let mut entry = zr
            .by_name("doc.json")
            .map_err(|_| ArchiveError::Invalid("doc.json が含まれていません".to_string()))?;
        let mut text = String::new();
        entry.read_to_string(&mut text)?;
        ProjectDoc::from_json(&text)?
    };
    let mut blobs = AssetBlobs::new();
    for asset in &doc.assets {
        if let Ok(mut entry) = zr.by_name(&format!("assets/{}.glb", asset.id)) {
            let mut data = Vec::new();
            entry.read_to_end(&mut data)?;
            blobs.insert(asset.id.clone(), data);
        }
    }
    Ok((doc, blobs))
}
