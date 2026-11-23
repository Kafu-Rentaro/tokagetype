# tokagetype 詳細仕様 v1

2D・3Dプロトタイピングツール（ブラウザ版）。Figmaのように画面をデザインして操作を試せること、Blender / Unity と GLB で受け渡しできること、ツール内でスカルプト造形できることを目標とする。

## 技術スタック

- React 19 / TypeScript / Vite
- 2D編集: Konva + react-konva
- 3D: three + @react-three/fiber + @react-three/drei
- スカルプト補助: three-mesh-bvh
- 保存: IndexedDB（idb-keyval 等の薄いラッパ可）
- テスト: Vitest（文書モデルの単体テスト）

## 文書モデル（保存形式）

文書はバージョン付き JSON とする。`version: 1`。

```ts
interface ProjectDoc {
  version: 1;
  id: string;               // プロジェクトID
  name: string;
  frames: Frame[];          // 画面（フレーム）一覧
  assets: AssetRef[];       // アセット参照（実体は IndexedDB の Blob ストア）
  startFrameId: string;     // 再生開始フレーム
}

interface Frame {
  id: string;
  name: string;
  width: number;            // 既定 390（スマホ幅）
  height: number;           // 既定 844
  background: string;       // 背景色
  nodes: Node[];            // 描画順 = 配列順（後ろが手前）
}

type Node = RectNode | TextNode | ButtonNode | View3DNode;

interface NodeBase {
  id: string;
  name: string;
  x: number; y: number;     // フレーム左上原点
  width: number; height: number;
  rotation?: number;        // 度
  opacity?: number;         // 0..1
  visible?: boolean;
  actions?: Action[];       // クリック時の操作（ボタン以外にも付与可）
}

interface RectNode extends NodeBase {
  type: 'rect';
  fill: string;
  cornerRadius: number;
  stroke?: string;
  strokeWidth?: number;
}

interface TextNode extends NodeBase {
  type: 'text';
  text: string;
  fontSize: number;
  fill: string;
  fontStyle?: string;       // 'normal' | 'bold'
  align?: 'left' | 'center' | 'right';
}

interface ButtonNode extends NodeBase {
  type: 'button';
  label: string;
  fill: string;
  textColor: string;
  cornerRadius: number;
  fontSize: number;
}

interface View3DNode extends NodeBase {
  type: 'view3d';
  assetId: string | null;   // 表示する GLB アセット
  activeCameraId: string;   // 現在のカメラ
  cameras: CameraDef[];     // 名前付きカメラプリセット
  autoRotate?: boolean;
}

interface CameraDef {
  id: string;
  name: string;             // 例: 'front' | 'back' | 任意名
  position: [number, number, number];
  target: [number, number, number];
}

interface AssetRef {
  id: string;
  name: string;             // 元ファイル名
  kind: 'glb';
  // 実体（ArrayBuffer）は IndexedDB の assetBlobs ストアに id をキーとして保存
}
```

## 操作（Action）

操作は判別共用体で型付けし、文書内に保存する。再生モードのランタイムが解釈する。

```ts
type Action =
  | { type: 'setCamera'; viewNodeId: string; cameraId: string }
  | { type: 'navigate'; frameId: string }
  | { type: 'toggleVisible'; nodeId: string }
  | { type: 'openUrl'; url: string };
```

- `setCamera`: 指定 3D ビューのアクティブカメラを切り替える（補間アニメーション付き）。
- `navigate`: 再生中の表示フレームを切り替える。
- ID はすべて文書内で一意。操作の参照先が存在しない場合は実行時に無視し、編集時に警告表示する。

## 保存

- プロジェクト文書: IndexedDB `tokagetype` DB の `projects` ストア（キー = プロジェクトID）。
- アセット実体: 同 DB の `assetBlobs` ストア（キー = アセットID、値 = ArrayBuffer）。
- 自動保存: 編集後 500ms デバウンスで保存。保存状態をステータスバーに表示。
- 書き出し/読み込み: プロジェクトを `.tokage`（zip: doc.json + assets/*.glb）として export / import できる。

## 編集モード

- 左: フレーム一覧＋レイヤー（ノード）一覧パネル
- 中央: Konva キャンバス。フレームを描画し、ノードの選択・移動・リサイズ（Transformer）・回転ができる
- 右: プロパティパネル（選択ノードの位置・サイズ・色・テキスト等を編集）
- ツールバー: rect / text / button / view3d の追加、削除、複製、Undo / Redo（Ctrl+Z / Ctrl+Shift+Z）
- 3D ビュー: ノード内に R3F Canvas を重ね、GLB を表示。カメラプリセットの追加・編集（現在の視点を保存）ができる
- GLB 取り込み: ファイル選択またはドラッグ&ドロップでアセット化し、3D ビューに割り当てる
- テキスト編集: ダブルクリックで HTML textarea を重ねて編集（日本語入力対応）

## 再生モード

- startFrameId のフレームを実寸で表示し、ノードの actions を実行できる
- setCamera は 0.6 秒のイージングで補間する
- navigate でフレーム遷移。戻るボタンで開始フレームへ
- 編集モードと再生モードはトグルで切り替え、編集中の文書は壊さない（再生状態は文書に保存しない）

## スカルプト（段階3）

- 対象: 3D ビュー内の GLB、または組み込みの球プリミティブ
- ブラシ: push（引く）/ pull（押す）/ smooth（滑らか）。半径と強さを調整可能
- 実装: BufferGeometry の position 属性を直接更新し、three-mesh-bvh の BVH でブラシ範囲の頂点を検索する。初版は頂点数固定（リメッシュなし）
- 1 ストローク = 1 Undo 単位。スカルプト結果は GLB として書き出し、アセットを置き換えることで保存・復元できる

## 外部連携（段階4）

- GLB 読み込み: Blender / Unity から書き出した .glb をそのまま表示できる
- GLB 書き出し: 選択中アセット（スカルプト結果含む）を .glb でダウンロード（GLTFExporter）
- 座標系: glTF 標準（Y-up、メートル）。向き・スケールの変換は行わない

## マイルストーンと完了条件

### 試作1（必須・最初の関門）
1. 商品紹介画面を1枚作れる（フレーム＋GLB の 3D ビュー＋「背面を見る」ボタン）
2. 再生モードでボタンを押すとカメラが背面プリセットに切り替わる
3. 保存後にブラウザを再読み込みしても、モデル・ボタンの位置・操作の関連が復元される

### 段階2
- フレームを2枚以上作り、navigate アクションで再生中に遷移できる

### 段階3
- スカルプト3種ブラシ、ストローク単位 Undo、保存後も造形が残る

### 段階4
- Blender 出力 GLB の読み込み、造形後 GLB の書き出し

### 実用化仕上げ
- プロジェクトの新規作成・複数保存・切り替え
- .tokage での export / import
- サンプルプロジェクト（試作1の商品紹介画面）を初回起動時に用意する

## 非目標（v1）

- リアルタイム共同編集（Yjs 等）
- 動的リメッシュ、頂点数の増減を伴うスカルプト
- Rust / WASM 高速化
- クラウド保存
