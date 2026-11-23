This is the prototype !
nothing at this time

---

# tokagetype（実装）

2D・3Dプロトタイピングツール（ブラウザ版）。詳細仕様は [docs/SPEC.md](docs/SPEC.md)。

## 開発手順

```bash
npm install
npm run dev        # 開発サーバー（http://localhost:5173）
npm run build      # 型チェック + 本番ビルド（dist/）
npm test           # Vitest 単体テスト（文書モデル / Action 検証 / 履歴）
npm run preview    # ビルド結果のプレビュー
```

## 使い方の概要

- 初回起動時にサンプルプロジェクト（スマホ商品紹介画面）が自動で用意されます。
- 編集モード: 左でフレーム/レイヤー管理、中央の Konva キャンバスでノードを編集、
  右のパネルでプロパティ・カメラ・クリック時操作（Action）を設定します。
  GLB はファイル選択またはキャンバスへのドラッグ&ドロップで取り込めます。
  3Dビューの視点を回すには、プロパティパネルで「3D操作」を有効にしてください。
- 再生モード: 「▶ 再生」で開始フレームから実行。ボタンの setCamera / navigate /
  toggleVisible / openUrl が動作します（再生状態は文書に保存されません）。
- スカルプト: 3Dビュー選択中、右パネルの「🗿 スカルプト」から開きます。
  push / pull / smooth ブラシ、ストローク単位 Undo、適用すると GLB 化して
  アセットを置き換え保存します（他のメッシュは保持されます）。
- 保存は IndexedDB へ自動（編集後 500ms デバウンス）。
  プロジェクトの受け渡しは `.tokage`（zip: doc.json + assets/*.glb）の
  export / import で行えます。アセット単体の GLB 書き出しも可能です。
