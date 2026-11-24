# tokagetype 詳細仕様 v1

2D・3Dプロトタイピングツール。プロジェクトの目標は「3D・2D・Web UI を一つのソフトウェアで完結させる」ことであり、Web 版に加えて Rust による Native UI 版を持つ。Figmaのように画面をデザインして操作を試せること、Blender / Unity と GLB で受け渡しできること、ツール内でスカルプト造形できることを目標とする。文書モデル（doc.json）は Web 版 / Native 版で共有する。

## 技術スタック

### Web 版
- React 19 / TypeScript / Vite
- 2D編集: Konva + react-konva
- 3D: three + @react-three/fiber + @react-three/drei
- スカルプト補助: three-mesh-bvh
- 保存: IndexedDB（idb-keyval 等の薄いラッパ可）
- テスト: Vitest（文書モデルの単体テスト）

### Native 版
- Rust（Cargo ワークスペース `native/`）
- 文書モデル: `tokage-core` クレート（serde、Web 版 doc.json と互換）
- UI: `tokage-app` クレート（egui / eframe、wgpu バックエンド）
- 3D: 自前 wgpu レンダラ（egui_wgpu コールバック）+ gltf クレートで GLB 読込
- 詳細は「Native 版」の節を参照

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
- アセット欠落への対処（フェーズ5）: 起動時に文書が参照するアセット Blob の存在をチェックする。3D ビューは状態（未設定 / 読込中 / 読込失敗 / 実体なし）をオーバーレイで明示し、無言のプレースホルダで固まらない。読込失敗時はエラー内容と再試行ボタンを出す。サンプルプロジェクトについては、Blob が欠けていればサンプル GLB を再生成して自己修復する（初回投入は Blob → 文書の順に保存し、中断しても壊れた状態が残らないようにする）。

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

## モバイル UI（Web 版）

Web 版は PC UI とスマホ UI の2形態を持ち、ヘッダーの「PC / スマホ」切替ボタンで切り替える。

- 既定は画面幅で自動判定する（768px 未満はスマホ UI）。手動切替が優先され、選択は localStorage（`tokagetype:uiMode`）に保持される。手動選択がある場合、リサイズでは自動切替しない。
- スマホ UI の編集モード: キャンバスを主画面とする。左パネル（フレーム/レイヤー）と右パネル（プロパティ）はボトムシートとして下から開閉する。ツールバーは画面下部の横スクロール可能なバーに移し、タップ対象は 44px 以上を確保する。編集/再生の切替と UI 切替はヘッダーに常駐させる。
- スマホ UI の再生モード: フレームを画面幅にフィット（縮小表示）させる。
- キャンバスの初期ズームはスマホ UI ではフレーム幅フィットとする。2本指ピンチでズームできる（0.15〜3倍）。Konva のノードドラッグ・選択はタッチでも動作する（onTap 併用）。

## デザイン方針（Web 版 UI）

方向: 「製図台」— 精密な計測器・製図道具の性格を持たせる。AI 生成にありがちな見た目（紫グラデーション、グロー、絵文字ラベル、不揃いな余白）を排除する（unslop の考え方）。

- 地色は無彩色チャコール（`--bg #16171a` / `--panel #1e1f23` / `--inset #101114`）。UI クロームに青・紫の色味を持たせない。
- アクセントは1色のみ: バーミリオン（`--accent #ff4d1c`、赤鉛筆・校正記号の朱）。選択状態、アクティブなトグル、Transformer の枠に使う。
- 罫線は 1px ヘアライン（`--line #34353b`）。影は使わない（再生フレームの外形も罫線で示す）。
- 角丸は小さく（`--radius 4px`）。余白は 4px グリッドで揃える。
- 数値・寸法（座標、ズーム率、フレームサイズ）は等幅フォント（`--mono`）で表示する。
- アイコンは線画 SVG（stroke: currentColor）に統一し、絵文字をラベルに使わない。レイヤーの種別バッジは等幅・大文字で示す。
- キャンバス周囲の地はドットグリッド（方眼）とする。
- 色トークンは `src/styles.css` の CSS 変数に集約し、共有スタイルは `src/ui/theme.ts`、アイコンは `src/ui/icons.tsx` に置く。
- 文書コンテンツ側の色（ノードの fill 等、ユーザーが決める色）はこの方針の対象外。

## Native 版（Rust）

Web 版と文書モデルを共有するデスクトップ版。`native/` に Cargo ワークスペースを置く。

### 構成
- `tokage-core`: 文書モデル（ProjectDoc / Frame / Node / Action）の serde 実装と検証。Web 版の doc.json をそのまま読み書きできる（フィールド名・構造・任意フィールドの省略規則まで互換）。数値は JS の JSON 表記に合わせ、整数値の f64 は小数点なしでシリアライズする。検証は Web 版 `validate.ts` の validateDoc / validateAction に対応。
  - `factory`（Web 版 factory.ts に対応する生成・既定値）、`edit`（ノード/フレームの追加・複製・削除・並び替え）、`history`（文書スナップショット方式の Undo/Redo、Web 版 history.ts に対応）、`archive`（`.tokage` の読み書き、Web 版 tokage.ts と同一レイアウト）を含む。
- `tokage-app`: egui / eframe による UI。現在の機能:
  - プロジェクトを開く（同梱サンプル＝Web 版フィクスチャ、新規作成、パス指定。`.json` / `.tokage` 両対応、引数での指定も可）
  - 読み込み時に検証結果（エラー/警告件数と詳細）をステータスバーに表示する
  - フレーム一覧とノード一覧（Web 版と同じく上が手前の順）を表示する
  - 2D 編集: プレビュー上でノードのクリック選択、ドラッグ移動、8方向ハンドルでのリサイズ。右パネルでプロパティ（位置・サイズ・回転・不透明度・表示、種別ごとの塗り/文字/ラベル/角丸/枠線、アセット割当、アクティブカメラ、自動回転）とアクション（追加・削除・パラメータ）を編集できる
  - ノード操作: rect / text / button / view3d の追加、複製、削除、前面/背面への並び替え。フレームの追加・削除・切り替え、開始フレーム指定、フレーム名/サイズ/背景の編集、プロジェクト名の編集
  - 履歴: Undo / Redo（Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y、ボタン）。連続編集（ドラッグ・入力）は開始時スナップショットで1ステップにまとめる
  - 保存: 開いたファイルへの上書きと「名前を付けて保存」（Ctrl+S）。拡張子で JSON / .tokage を判別する。保存前に検証し、エラーがあれば表示して保存しない。未保存の変更はツールバーとステータスバーに示す
  - 再生モード: 編集/再生を切り替える。開始フレームから表示し、クリックで Action を実行する（navigate=フレーム遷移、toggleVisible=表示切替、openUrl=OS のブラウザで開く、setCamera=対象ビューの実カメラ切替。状態は保持する）。開始フレームへ戻るボタンを持つ。再生状態は文書に保存しない
  - 2D プレビュー描画（rect / text / button を実描画）
  - 3D 実描画（フェーズ5）: eframe の wgpu バックエンド + `egui_wgpu::Callback` による自前レンダラで view3d に GLB を描画する。gltf クレートで読み込み、Web 版 fitTransform と同じ規則（最大寸法 1.6 正規化・底面 y=0）で自動フィット。baseColor テクスチャ / emissive テクスチャ / 頂点色 / baseColorFactor・emissiveFactor に対応し、方向光源+環境光の簡易ライティングで描く（PBR の金属・粗さ、法線マップ、オクルージョンは未対応）。カメラは View3DNode のプリセット（position/target）を使用し、編集モードではプロパティの「3D操作」を有効にしたノードでドラッグ=オービット、ホイール=ズームができる。自動回転にも対応。アセット未割当・実体なし・読込失敗の場合は理由をプレースホルダに表示する
  - アセット解決（Native 版）: `.tokage` は内蔵 assets が正。JSON を開いた場合は同じフォルダの `assets/<assetId>.glb` → `<アセット名>` の順に外部ファイルを探して取り込む。同梱サンプルの GLB は実体を同梱しないため、`sample-phone.glb` が不足する場合は Web 版サンプルと同じ形状を手続き生成して補完する。JSON で保存し直すと実体は含まれない（実体を残す場合は .tokage で保存する）
  - スカルプト（フェーズ6）: view3d ノードのプロパティからスカルプトモード（全画面モーダル）へ入る。対象は Web 版と同じく GLB の最初のメッシュ（走査順で最初に現れる頂点を持つプリミティブ）。アセット未割当・実体なしの場合は Web 版と同じ球プリミティブを生成して対象にする。ブラシは pull（法線方向へ引き出す）/ push（押し込む）/ smooth（平均位置へ寄せる）の3種で、半径 0.05〜0.8・強さ 0.05〜1.5 を調整できる。変形式とフォールオフは Web 版 brush.ts と同一。頂点検索は BVH の代わりに空間グリッド（セル = 半径）を使う。ヒット判定はポインタからのレイキャスト（Möller–Trumbore、両面）。変形はレンダラと同じフィット空間で行い、頂点バッファの動的差し替えで即時表示する。ブラシカーソル（ヒット点と半径）を重ねて表示する。1ストローク = 1 Undo（ストローク開始時点の頂点スナップショット、上限30、スカルプト内 Undo/Redo）。「適用して保存」で変形を GLB へ書き戻してアセット実体を差し替える。書き戻しは元 GLB の JSON/BIN を保持したまま対象プリミティブの POSITION / NORMAL アクセサだけを新しいデータに差し替える方式で、他メッシュ・テクスチャ・マテリアルは保持される。法線はローカル空間で再計算する。未割当だった場合は新規アセット（sculpted.glb）を作成してノードへ割り当て、既存アセットは名前に「 (sculpt)」を付与する（Web 版と同じ）。文書側の変更は通常の履歴つき編集として記録し、アセット実体の差し替えは未保存フラグで示す。視点回転モード（オービット/ズーム）も持つ。制限: 対象は三角形プリミティブ前提。スキニング・モーフィング・スパースアクセサのあるモデル、外部バッファ参照の GLB は書き戻し対象外
  - 日本語表示のため、システムの Noto Sans CJK があれば読み込む

### ロードマップ（Native 版の今後）
1. ~~ノードの選択ハイライトとプロパティ表示（読み取り専用）~~ → 完了（フェーズ3で表示、フェーズ4で編集可能に）
2. ~~ノードの移動・リサイズ等の編集と、履歴（Undo/Redo）~~ → 完了（フェーズ4）
3. ~~.tokage（zip）の読み書きとアセット GLB の保持~~ → 完了（フェーズ4。アセット実体は保持のみ、3D 表示はプレースホルダ）
4. ~~3D プレビュー（wgpu 描画）とカメラプリセット切替~~ → 完了（フェーズ5。自前 wgpu レンダラ + GLB 読込 + テクスチャ対応）
5. ~~再生モードの 3D 連携（setCamera の実カメラ切替）とスカルプト~~ → 完了（setCamera はフェーズ5、スカルプトはフェーズ6）
6. Web 版との相互運用テストの拡充（.tokage 往復、フィクスチャ追加） ← 次の到達点

### 互換性の担保
- Web 版アプリが実際に保存したサンプル doc.json を `tokage-core/tests/fixtures/sample-project.json` にフィクスチャとして置き、読み込み→検証→再シリアライズ→JSON 値一致を単体テストで担保する。

## 非目標（v1）

- リアルタイム共同編集（Yjs 等）
- 動的リメッシュ、頂点数の増減を伴うスカルプト
- Web 版の Rust / WASM 化（Native 版は別系統として開発する）
- クラウド保存
