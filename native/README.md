# tokagetype Native 版

Web 版と文書モデル（doc.json）を共有する Rust 製デスクトップ版。仕様は `../docs/SPEC.md`「Native 版」を参照。

## 構成

- `tokage-core` — 文書モデル（serde）と検証。Web 版 doc.json と互換。
- `tokage-app` — egui / eframe による UI（現時点は「開く・一覧・2Dプレビュー」の土台）。

## ビルドと実行

```sh
cd native
cargo build
cargo test -p tokage-core   # Web 版フィクスチャとの互換テスト
cargo run -p tokage-app     # 起動（同梱サンプルが開く）
cargo run -p tokage-app -- /path/to/doc.json
```

Linux では eframe の実行に libxkbcommon-x11 等のシステムライブラリが必要な場合がある。
日本語表示はシステムの Noto Sans CJK フォントがあれば自動で読み込む。
