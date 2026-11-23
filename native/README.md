# tokagetype Native 版

Web 版と文書モデル（doc.json）を共有する Rust 製デスクトップ版。仕様は `../docs/SPEC.md`「Native 版」を参照。

## 構成

- `tokage-core` — 文書モデル（serde）と検証、編集操作・履歴、`.tokage` の読み書き。Web 版 doc.json と互換。
- `tokage-app` — egui / eframe による UI。開く・一覧・2Dプレビューに加え、編集（移動/リサイズ/プロパティ）、Undo/Redo、保存、再生モードを持つ。

## 使い方（tokage-app）

- 開く: 引数でパス指定、またはツールバーのパス欄から `.json` / `.tokage` を開く。「サンプル」「新規」ボタンもある。
- 編集: キャンバス上でノードをクリックして選択、ドラッグで移動、四隅・辺のハンドルでリサイズ。右パネルでプロパティとアクションを編集する。
- ショートカット: Ctrl+S = 保存、Ctrl+Z / Ctrl+Shift+Z（Ctrl+Y）= Undo/Redo、Delete = ノード削除。
- 保存: 開いたファイルへ上書き（`.tokage` で開いた場合は zip 形式のまま保存）。検証エラーがある場合は保存しない。
- 再生: ツールバーの「▶ 再生」で開始フレームから表示し、クリックでアクション（遷移・表示切替・カメラ切替・URL）を実行する。

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
