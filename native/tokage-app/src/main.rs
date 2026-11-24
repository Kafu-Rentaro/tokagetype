//! tokagetype Native 版（tokage-app）。
//!
//! docs/SPEC.md「Native 版」参照。Web 版と文書モデル（doc.json）を共有する。
//! フェーズ4: 2D 編集（選択/移動/リサイズ/プロパティ編集）、ノード・フレーム操作、
//! Undo/Redo、JSON / .tokage の読み書き、再生モード（Action 実行）を持つ。
//! フェーズ5: view3d の実描画（wgpu 自前レンダラ + GLB 読込 + カメラプリセット /
//! オービット / 自動回転、再生モードの setCamera 反映）。

mod glb;
mod math3d;
mod render3d;
mod sample_glb;

use eframe::egui;
use egui::{Color32, FontId, Pos2, Rect, Rounding, Stroke, Vec2};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokage_core::{
    edit, validate_doc, Action, AssetBlobs, Frame, History, Level, Node, ProjectDoc, View3DNode,
};

const SAMPLE_JSON: &str = include_str!("../../tokage-core/tests/fixtures/sample-project.json");
const ACCENT: Color32 = Color32::from_rgb(0xff, 0x4d, 0x1c);
const HANDLE_SIZE: f32 = 9.0;
const MIN_NODE_SIZE: f64 = 10.0;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 760.0])
            .with_title("tokagetype (native)"),
        ..Default::default()
    };
    eframe::run_native(
        "tokagetype",
        options,
        Box::new(|cc| {
            install_cjk_font(&cc.egui_ctx);
            Ok(Box::new(TokageApp::default()))
        }),
    )
}

/// 日本語フォント（Noto Sans CJK）がシステムにあれば読み込む。無ければ既定フォントのまま。
fn install_cjk_font(ctx: &egui::Context) {
    const CANDIDATES: &[&str] = &[
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    ];
    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".to_owned(), egui::FontData::from_owned(bytes));
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                if let Some(list) = fonts.families.get_mut(&family) {
                    list.insert(0, "cjk".to_owned());
                }
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Edit,
    Play,
}

/// 再生モードの実行時状態（文書には保存しない。Web 版 playStore に対応）。
struct PlayState {
    frame_id: String,
    visible_override: HashMap<String, bool>,
    camera_override: HashMap<String, String>,
}

impl PlayState {
    fn new(start_frame_id: &str) -> Self {
        PlayState {
            frame_id: start_frame_id.to_string(),
            visible_override: HashMap::new(),
            camera_override: HashMap::new(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Handle {
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

#[derive(Clone, Copy)]
struct RectF {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

struct DragState {
    node_id: String,
    handle: Option<Handle>,
    start_pointer: Pos2, // 文書座標
    start_rect: RectF,
    /// 3D 操作モード中の view3d ドラッグ（文書は変更せず視点だけ回す）
    orbit: bool,
}

/// view3d ノードごとのオービット状態（編集モード）。
#[derive(Clone, Copy)]
struct OrbitState {
    yaw: f32,
    pitch: f32,
    dist: f32,
}

impl Default for OrbitState {
    fn default() -> Self {
        OrbitState {
            yaw: 0.0,
            pitch: 0.0,
            dist: 1.0,
        }
    }
}

struct TokageApp {
    history: History,
    assets: AssetBlobs,
    current_path: Option<PathBuf>,
    source_label: String,
    saved_snapshot: ProjectDoc,
    open_path_input: String,
    save_as_input: String,
    frame_idx: usize,
    selected_node: Option<String>,
    mode: Mode,
    play: Option<PlayState>,
    drag: Option<DragState>,
    /// アセット ID → (モデルキャッシュキー, 実体バイト)（3D 描画用）
    asset_arcs: HashMap<String, (String, Arc<Vec<u8>>)>,
    /// 3D 読込エラー（モデルキャッシュキー → 理由）
    render_errors: render3d::RenderErrors,
    /// wgpu サーフェス形式（初回フレームで取得）
    surface_format: Option<wgpu::TextureFormat>,
    /// 3D 操作（オービット）を有効にした view3d ノード ID
    orbit_node: Option<String>,
    /// ノードごとのオービット状態
    orbits: HashMap<String, OrbitState>,
    /// 自動回転の累積角（ノード ID → ラジアン）
    spin: HashMap<String, f32>,
    /// 連続編集（ドラッグ/入力）の開始時スナップショット。終了時に History::record する。
    edit_base: Option<ProjectDoc>,
    status: String,
}

impl Default for TokageApp {
    fn default() -> Self {
        let doc = ProjectDoc::from_json(SAMPLE_JSON).expect("同梱サンプルが読めること");
        let mut app = TokageApp {
            history: History::new(doc.clone()),
            assets: AssetBlobs::new(),
            current_path: None,
            source_label: String::new(),
            saved_snapshot: doc,
            open_path_input: String::new(),
            save_as_input: String::new(),
            frame_idx: 0,
            selected_node: None,
            mode: Mode::Edit,
            play: None,
            drag: None,
            asset_arcs: HashMap::new(),
            render_errors: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            surface_format: None,
            orbit_node: None,
            orbits: HashMap::new(),
            spin: HashMap::new(),
            edit_base: None,
            status: String::new(),
        };
        app.fill_sample_assets();
        app.refresh_asset_arcs();
        if let Some(path) = std::env::args().nth(1) {
            app.open_path(&PathBuf::from(&path));
            if app.current_path.is_none() && app.source_label.is_empty() {
                app.source_label = "bundled sample (Web version fixture)".into();
            }
        } else {
            app.source_label = "bundled sample (Web version fixture)".into();
            app.refresh_validation_status();
        }
        app.frame_idx = app
            .doc()
            .frames
            .iter()
            .position(|f| f.id == app.doc().start_frame_id)
            .unwrap_or(0);
        app
    }
}

impl TokageApp {
    fn doc(&self) -> &ProjectDoc {
        self.history.present()
    }

    fn doc_mut(&mut self) -> &mut ProjectDoc {
        self.history.present_mut()
    }

    fn current_frame_id(&self) -> Option<String> {
        self.doc().frames.get(self.frame_idx).map(|f| f.id.clone())
    }

    fn current_frame(&self) -> Option<&Frame> {
        self.doc().frames.get(self.frame_idx)
    }

    fn is_dirty(&self) -> bool {
        self.doc() != &self.saved_snapshot
    }

    fn refresh_validation_status(&mut self) {
        let issues = validate_doc(self.doc());
        let errors = issues.iter().filter(|i| i.level == Level::Error).count();
        let warnings = issues.iter().filter(|i| i.level == Level::Warning).count();
        self.status = format!("検証: エラー {errors} 件 / 警告 {warnings} 件");
    }

    // ---- 履歴つき編集の基本操作 ----

    /// 離散変更: クローンした文書に関数を適用し、履歴に push する。
    fn mutate(&mut self, f: impl FnOnce(&mut ProjectDoc)) {
        self.end_edit();
        let mut next = self.doc().clone();
        f(&mut next);
        self.history.push(next);
        self.fix_selection();
    }

    /// 連続編集の開始（開始時スナップショットを保持）。
    fn begin_edit(&mut self) {
        if self.edit_base.is_none() {
            self.edit_base = Some(self.doc().clone());
        }
    }

    /// 連続編集の終了（変化があれば履歴に記録）。
    fn end_edit(&mut self) {
        if let Some(base) = self.edit_base.take() {
            self.history.record(base);
            self.fix_selection();
        }
    }

    fn undo(&mut self) {
        self.end_edit();
        if self.history.undo() {
            self.drag = None;
            self.fix_selection();
            self.status = "元に戻しました".into();
        }
    }

    fn redo(&mut self) {
        self.end_edit();
        if self.history.redo() {
            self.drag = None;
            self.fix_selection();
            self.status = "やり直しました".into();
        }
    }

    /// frame_idx と選択ノードが現在の文書に対して有効な範囲に収まるよう調整する。
    fn fix_selection(&mut self) {
        let frame_count = self.doc().frames.len();
        if frame_count == 0 {
            self.frame_idx = 0;
            self.selected_node = None;
            return;
        }
        if self.frame_idx >= frame_count {
            self.frame_idx = frame_count - 1;
        }
        if let Some(sel) = self.selected_node.clone() {
            let exists = self
                .doc()
                .frames
                .get(self.frame_idx)
                .map(|f| f.nodes.iter().any(|n| n.base().id == sel))
                .unwrap_or(false);
            if !exists {
                self.selected_node = None;
            }
        }
    }

    // ---- 開く / 保存 ----

    fn load_doc(&mut self, doc: ProjectDoc, assets: AssetBlobs, label: String, path: Option<PathBuf>) {
        let path_for_assets = path.clone();
        self.history.reset(doc.clone());
        self.assets = assets;
        self.saved_snapshot = doc;
        self.current_path = path;
        self.source_label = label;
        self.frame_idx = self
            .doc()
            .frames
            .iter()
            .position(|f| f.id == self.doc().start_frame_id)
            .unwrap_or(0);
        self.selected_node = None;
        self.drag = None;
        self.edit_base = None;
        self.mode = Mode::Edit;
        self.play = None;
        self.orbit_node = None;
        self.orbits.clear();
        self.spin.clear();
        // JSON を開いた場合は同フォルダの外部 GLB を探す（SPEC「Native 版アセット解決」）
        if let Some(p) = &path_for_assets {
            let is_tokage = p
                .extension()
                .map(|e| e.eq_ignore_ascii_case("tokage"))
                .unwrap_or(false);
            if !is_tokage {
                self.load_sibling_assets(p);
            }
        }
        self.fill_sample_assets();
        self.refresh_asset_arcs();
        self.refresh_validation_status();
    }

    /// JSON と同じフォルダにある外部 GLB をアセット実体として取り込む。
    /// 探索順: `assets/<assetId>.glb` → `<アセット名>`（.tokage の展開形と同じ規則）。
    fn load_sibling_assets(&mut self, json_path: &Path) {
        let Some(dir) = json_path.parent() else {
            return;
        };
        for asset in self.doc().assets.clone() {
            if self.assets.contains_key(&asset.id) {
                continue;
            }
            let candidates = [
                dir.join("assets").join(format!("{}.glb", asset.id)),
                dir.join(&asset.name),
            ];
            for cand in candidates {
                if let Ok(bytes) = std::fs::read(&cand) {
                    self.assets.insert(asset.id.clone(), bytes);
                    break;
                }
            }
        }
    }

    /// 同梱サンプル用 GLB が不足していれば手続き生成で補完する。
    fn fill_sample_assets(&mut self) {
        let missing: Vec<String> = self
            .doc()
            .assets
            .iter()
            .filter(|a| a.name == "sample-phone.glb" && !self.assets.contains_key(&a.id))
            .map(|a| a.id.clone())
            .collect();
        if missing.is_empty() {
            return;
        }
        let bytes = sample_glb::build_sample_phone_glb();
        for id in missing {
            self.assets.insert(id, bytes.clone());
        }
    }

    /// 3D 描画用にアセット実体を Arc 化し、内容ハッシュつきキャッシュキーを作る。
    fn refresh_asset_arcs(&mut self) {
        self.asset_arcs = self
            .assets
            .iter()
            .map(|(id, bytes)| {
                let mut h: u64 = 0xcbf2_9ce4_8422_2325;
                for b in bytes {
                    h ^= u64::from(*b);
                    h = h.wrapping_mul(0x0000_0100_0000_01b3);
                }
                (
                    id.clone(),
                    (format!("{id}:{h:016x}"), Arc::new(bytes.clone())),
                )
            })
            .collect();
    }

    /// view3d ノードの有効カメラ（再生モードでは setCamera の上書きを優先）。
    fn view3d_camera(&self, n: &View3DNode) -> (math3d::Vec3, math3d::Vec3) {
        let cam_id = if self.mode == Mode::Play {
            self.play
                .as_ref()
                .and_then(|p| p.camera_override.get(&n.base.id))
                .cloned()
                .unwrap_or_else(|| n.active_camera_id.clone())
        } else {
            n.active_camera_id.clone()
        };
        let cam = n
            .cameras
            .iter()
            .find(|c| c.id == cam_id)
            .or_else(|| n.cameras.first());
        match cam {
            Some(c) => (
                [
                    c.position[0] as f32,
                    c.position[1] as f32,
                    c.position[2] as f32,
                ],
                [c.target[0] as f32, c.target[1] as f32, c.target[2] as f32],
            ),
            None => ([2.6, 1.8, 3.2], [0.0, 0.8, 0.0]),
        }
    }

    fn open_sample(&mut self) {
        match ProjectDoc::from_json(SAMPLE_JSON) {
            Ok(doc) => self.load_doc(
                doc,
                AssetBlobs::new(),
                "bundled sample (Web version fixture)".into(),
                None,
            ),
            Err(e) => self.status = format!("サンプルの読み込みに失敗: {e}"),
        }
    }

    fn new_project(&mut self) {
        let doc = tokage_core::create_project("新規プロジェクト");
        self.load_doc(doc, AssetBlobs::new(), "新規プロジェクト".into(), None);
    }

    fn open_path(&mut self, path: &PathBuf) {
        let is_tokage = path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("tokage"))
            .unwrap_or(false);
        if is_tokage {
            match std::fs::read(path) {
                Ok(bytes) => match tokage_core::read_tokage(&bytes) {
                    Ok((doc, assets)) => {
                        let n = assets.len();
                        self.load_doc(doc, assets, path.display().to_string(), Some(path.clone()));
                        self.status = format!("{} を開きました（アセット {n} 件）", path.display());
                    }
                    Err(e) => self.status = format!("読み込み失敗: {e}"),
                },
                Err(e) => self.status = format!("ファイルを開けません: {e}"),
            }
        } else {
            match std::fs::read_to_string(path) {
                Ok(json) => match ProjectDoc::from_json(&json) {
                    Ok(doc) => {
                        self.load_doc(doc, AssetBlobs::new(), path.display().to_string(), Some(path.clone()))
                    }
                    Err(e) => self.status = format!("読み込み失敗: {e}"),
                },
                Err(e) => self.status = format!("ファイルを開けません: {e}"),
            }
        }
    }

    /// 指定パスへ保存する。検証エラーがある場合は保存しない。
    fn save_to(&mut self, path: &PathBuf) -> Result<(), String> {
        let errors: Vec<String> = self
            .doc()
            .errors()
            .iter()
            .map(|i| i.message.clone())
            .collect();
        if !errors.is_empty() {
            return Err(format!(
                "検証エラーのため保存しません: {}",
                errors.join(" / ")
            ));
        }
        let is_tokage = path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("tokage"))
            .unwrap_or(false);
        if is_tokage {
            let bytes = tokage_core::write_tokage(self.doc(), &self.assets)
                .map_err(|e| e.to_string())?;
            std::fs::write(path, bytes).map_err(|e| e.to_string())?;
        } else {
            let json = self.doc().to_json_pretty().map_err(|e| e.to_string())?;
            std::fs::write(path, json).map_err(|e| e.to_string())?;
        }
        self.saved_snapshot = self.doc().clone();
        self.current_path = Some(path.clone());
        self.source_label = path.display().to_string();
        Ok(())
    }

    fn save(&mut self) {
        match self.current_path.clone() {
            Some(path) => match self.save_to(&path) {
                Ok(()) => self.status = format!("保存しました: {}", path.display()),
                Err(e) => self.status = e,
            },
            None => {
                self.status = "保存先が未設定です。「名前を付けて保存」のパスを入力してください".into()
            }
        }
    }

    // ---- 再生モード ----

    fn enter_play(&mut self) {
        self.end_edit();
        self.play = Some(PlayState::new(&self.doc().start_frame_id));
        self.mode = Mode::Play;
        self.drag = None;
    }

    fn exit_play(&mut self) {
        self.play = None;
        self.mode = Mode::Edit;
    }

    fn effective_visible(&self, node_id: &str) -> bool {
        if let Some(play) = &self.play {
            if let Some(v) = play.visible_override.get(node_id) {
                return *v;
            }
        }
        self.doc()
            .frames
            .iter()
            .flat_map(|f| f.nodes.iter())
            .find(|n| n.base().id == node_id)
            .map(|n| n.base().visible_or_default())
            .unwrap_or(true)
    }

    fn run_actions(&mut self, actions: &[Action]) {
        for action in actions {
            match action {
                Action::Navigate { frame_id } => {
                    if self.doc().frames.iter().any(|f| &f.id == frame_id) {
                        if let Some(play) = &mut self.play {
                            play.frame_id = frame_id.clone();
                        }
                    }
                }
                Action::ToggleVisible { node_id } => {
                    let cur = self.effective_visible(node_id);
                    if let Some(play) = &mut self.play {
                        play.visible_override.insert(node_id.clone(), !cur);
                    }
                }
                Action::SetCamera { view_node_id, camera_id } => {
                    let ok = self
                        .doc()
                        .frames
                        .iter()
                        .flat_map(|f| f.nodes.iter())
                        .find_map(|n| match n {
                            Node::View3d(v) if v.base.id == *view_node_id => Some(v),
                            _ => None,
                        })
                        .map(|v| v.cameras.iter().any(|c| &c.id == camera_id))
                        .unwrap_or(false);
                    if ok {
                        if let Some(play) = &mut self.play {
                            play.camera_override.insert(view_node_id.clone(), camera_id.clone());
                        }
                    }
                }
                Action::OpenUrl { url } => self.open_url(url),
            }
        }
    }

    fn open_url(&mut self, url: &str) {
        #[cfg(target_os = "macos")]
        let prog = "open";
        #[cfg(target_os = "windows")]
        let prog = "explorer";
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let prog = "xdg-open";
        match std::process::Command::new(prog).arg(url).spawn() {
            Ok(_) => self.status = format!("URL を開きました: {url}"),
            Err(e) => self.status = format!("URL を開けませんでした: {e}"),
        }
    }

    // ---- キーボード ----

    fn handle_keys(&mut self, ctx: &egui::Context) {
        let typing = ctx.memory(|m| m.focused().is_some());
        ctx.input(|i| {
            if i.key_pressed(egui::Key::S) && i.modifiers.ctrl {
                self.save();
            }
            if typing {
                return;
            }
            if i.key_pressed(egui::Key::Z) && i.modifiers.ctrl && !i.modifiers.shift {
                self.undo();
            } else if (i.key_pressed(egui::Key::Z) && i.modifiers.ctrl && i.modifiers.shift)
                || (i.key_pressed(egui::Key::Y) && i.modifiers.ctrl)
            {
                self.redo();
            }
            if self.mode == Mode::Edit && i.key_pressed(egui::Key::Delete) {
                if let (Some(fid), Some(nid)) =
                    (self.current_frame_id(), self.selected_node.clone())
                {
                    self.mutate(|doc| {
                        edit::remove_node(doc, &fid, &nid);
                    });
                    self.selected_node = None;
                    self.status = "ノードを削除しました".into();
                }
            }
        });
    }
}

fn parse_color(hex: &str, opacity: f64) -> Color32 {
    let h = hex.trim_start_matches('#');
    let (r, g, b) = if h.len() == 6 {
        (
            u8::from_str_radix(&h[0..2], 16).unwrap_or(0),
            u8::from_str_radix(&h[2..4], 16).unwrap_or(0),
            u8::from_str_radix(&h[4..6], 16).unwrap_or(0),
        )
    } else {
        (0xdd, 0xdd, 0xdd)
    };
    let a = (255.0 * opacity.clamp(0.0, 1.0)) as u8;
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

fn format_color(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

impl eframe::App for TokageApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // 3D 描画用に wgpu サーフェス形式を一度だけ取得する
        if self.surface_format.is_none() {
            self.surface_format = frame.wgpu_render_state().map(|s| s.target_format);
        }
        // 連続編集の終了検知: ポインタを離し、入力欄にもフォーカスが無い時点で確定する
        if self.edit_base.is_some()
            && !ctx.input(|i| i.pointer.any_down())
            && ctx.memory(|m| m.focused().is_none())
            && self.drag.is_none()
        {
            self.end_edit();
        }
        self.fix_selection();
        self.handle_keys(ctx);
        self.toolbar(ctx);
        self.left_panel(ctx);
        if self.mode == Mode::Edit {
            self.right_panel(ctx);
        }
        self.status_bar(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            self.canvas(ui);
        });
    }
}

impl TokageApp {
    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("tokagetype");
                ui.separator();
                if ui.button("サンプル").clicked() {
                    self.open_sample();
                }
                if ui.button("新規").clicked() {
                    self.new_project();
                }
                ui.label("開く:");
                ui.add(egui::TextEdit::singleline(&mut self.open_path_input).desired_width(220.0));
                if ui.button("開く").clicked() {
                    let path = self.open_path_input.trim().to_string();
                    if !path.is_empty() {
                        self.open_path(&PathBuf::from(path));
                    }
                }
                ui.separator();
                let save_label = if self.is_dirty() { "保存 ●" } else { "保存" };
                if ui.button(save_label).clicked() {
                    self.save();
                }
                ui.label("保存先:");
                ui.add(egui::TextEdit::singleline(&mut self.save_as_input).desired_width(220.0));
                if ui.button("名前を付けて保存").clicked() {
                    let path = self.save_as_input.trim().to_string();
                    if !path.is_empty() {
                        match self.save_to(&PathBuf::from(&path)) {
                            Ok(()) => self.status = format!("保存しました: {path}"),
                            Err(e) => self.status = e,
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.add_enabled_ui(self.history.can_undo(), |ui| {
                    if ui.button("↩ Undo").clicked() {
                        self.undo();
                    }
                });
                ui.add_enabled_ui(self.history.can_redo(), |ui| {
                    if ui.button("↪ Redo").clicked() {
                        self.redo();
                    }
                });
                ui.separator();
                if ui
                    .selectable_label(self.mode == Mode::Edit, "編集")
                    .clicked()
                    && self.mode != Mode::Edit
                {
                    self.exit_play();
                }
                if ui
                    .selectable_label(self.mode == Mode::Play, "▶ 再生")
                    .clicked()
                    && self.mode != Mode::Play
                {
                    self.enter_play();
                }
                if !self.source_label.is_empty() {
                    ui.separator();
                    ui.label(&self.source_label);
                }
            });
        });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let issues = validate_doc(self.doc());
                let errors = issues.iter().filter(|i| i.level == Level::Error).count();
                let warnings = issues.iter().filter(|i| i.level == Level::Warning).count();
                if errors > 0 {
                    ui.colored_label(
                        Color32::from_rgb(0xd0, 0x30, 0x30),
                        format!("検証エラー {errors} 件"),
                    );
                }
                if warnings > 0 {
                    ui.colored_label(
                        Color32::from_rgb(0xb0, 0x80, 0x10),
                        format!("警告 {warnings} 件"),
                    );
                }
                if errors == 0 && warnings == 0 {
                    ui.label("検証 OK");
                }
                if self.is_dirty() {
                    ui.separator();
                    ui.label("未保存の変更があります");
                }
                ui.separator();
                ui.label(&self.status);
            });
            // 検証の詳細（先頭数件）
            let issues = validate_doc(self.doc());
            for issue in issues.iter().take(4) {
                ui.small(&issue.message);
            }
        });
    }

    fn left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("frames")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                if self.mode == Mode::Play {
                    ui.strong("再生モード");
                    if let Some(play) = &self.play {
                        let name = self
                            .doc()
                            .frames
                            .iter()
                            .find(|f| f.id == play.frame_id)
                            .map(|f| f.name.clone())
                            .unwrap_or_default();
                        ui.label(format!("表示中: {name}"));
                    }
                    if ui.button("⤺ 開始フレームに戻る").clicked() {
                        let start = self.doc().start_frame_id.clone();
                        self.play = Some(PlayState::new(&start));
                    }
                    if ui.button("編集に戻る").clicked() {
                        self.exit_play();
                    }
                    ui.separator();
                    ui.label("フレームをクリックすると再生表示を切り替えます。");
                    ui.separator();
                }

                ui.horizontal(|ui| {
                    ui.label("プロジェクト:");
                    let mut name = self.doc().name.clone();
                    if ui.text_edit_singleline(&mut name).changed() {
                        self.begin_edit();
                        self.doc_mut().name = name;
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong("フレーム");
                    if self.mode == Mode::Edit && ui.button("＋ 追加").clicked() {
                        let n = self.doc().frames.len() + 1;
                        let mut new_id = String::new();
                        self.mutate(|doc| {
                            new_id = edit::add_frame(doc, &format!("フレーム{n}"));
                        });
                        if let Some(idx) = self.doc().frames.iter().position(|f| f.id == new_id) {
                            self.frame_idx = idx;
                            self.selected_node = None;
                        }
                    }
                });
                let doc = self.doc().clone();
                for (i, f) in doc.frames.iter().enumerate() {
                    let is_start = f.id == doc.start_frame_id;
                    let label = if is_start {
                        format!("★ {}", f.name)
                    } else {
                        f.name.clone()
                    };
                    let active = if self.mode == Mode::Play {
                        self.play.as_ref().map(|p| p.frame_id == f.id).unwrap_or(false)
                    } else {
                        self.frame_idx == i
                    };
                    if ui.selectable_label(active, label).clicked() {
                        if self.mode == Mode::Play {
                            if let Some(play) = &mut self.play {
                                play.frame_id = f.id.clone();
                            }
                        } else {
                            self.frame_idx = i;
                            self.selected_node = None;
                        }
                    }
                }
                if self.mode == Mode::Edit {
                    self.frame_props(ui);
                }
                ui.separator();
                ui.strong("ノード");
                if let Some(frame) = doc.frames.get(self.frame_idx).cloned() {
                    if self.mode == Mode::Edit {
                        ui.horizontal(|ui| {
                            for (label, kind) in [
                                ("＋矩形", tokage_core::NewNodeKind::Rect),
                                ("＋文字", tokage_core::NewNodeKind::Text),
                                ("＋ボタン", tokage_core::NewNodeKind::Button),
                                ("＋3D", tokage_core::NewNodeKind::View3d),
                            ] {
                                if ui.button(label).clicked() {
                                    let fid = frame.id.clone();
                                    let mut nid = String::new();
                                    self.mutate(|doc| {
                                        nid = edit::add_node(doc, &fid, kind, 40.0, 40.0)
                                            .unwrap_or_default();
                                    });
                                    if !nid.is_empty() {
                                        self.selected_node = Some(nid);
                                    }
                                }
                            }
                        });
                    }
                    // Web 版レイヤー表示と同じく上が手前 = 配列の逆順
                    for node in frame.nodes.iter().rev() {
                        let base = node.base();
                        let text = format!("[{}] {}", node.kind_name(), base.name);
                        if ui
                            .selectable_label(
                                self.selected_node.as_deref() == Some(base.id.as_str()),
                                text,
                            )
                            .clicked()
                        {
                            self.selected_node = Some(base.id.clone());
                        }
                    }
                    if frame.nodes.is_empty() {
                        ui.label("(ノードなし)");
                    }
                    if self.mode == Mode::Edit {
                        if let Some(nid) = self.selected_node.clone() {
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("複製").clicked() {
                                    let fid = frame.id.clone();
                                    let mut new_id = String::new();
                                    self.mutate(|doc| {
                                        new_id = edit::duplicate_node(doc, &fid, &nid)
                                            .unwrap_or_default();
                                    });
                                    if !new_id.is_empty() {
                                        self.selected_node = Some(new_id);
                                    }
                                }
                                if ui.button("削除").clicked() {
                                    let fid = frame.id.clone();
                                    self.mutate(|doc| {
                                        edit::remove_node(doc, &fid, &nid);
                                    });
                                    self.selected_node = None;
                                }
                                if ui.button("前面へ").clicked() {
                                    let fid = frame.id.clone();
                                    self.mutate(|doc| {
                                        edit::move_node_layer(doc, &fid, &nid, 1);
                                    });
                                }
                                if ui.button("背面へ").clicked() {
                                    let fid = frame.id.clone();
                                    self.mutate(|doc| {
                                        edit::move_node_layer(doc, &fid, &nid, -1);
                                    });
                                }
                            });
                        }
                    }
                }
            });
    }

    /// 選択中フレームのプロパティ（名前・サイズ・背景・開始フレーム指定）。
    fn frame_props(&mut self, ui: &mut egui::Ui) {
        let Some(frame) = self.current_frame().cloned() else {
            return;
        };
        ui.separator();
        ui.strong("フレーム設定");
        let fid = frame.id.clone();
        let mut local = frame.clone();
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("名前");
            changed |= ui.text_edit_singleline(&mut local.name).changed();
        });
        ui.horizontal(|ui| {
            ui.label("幅");
            changed |= ui
                .add(egui::DragValue::new(&mut local.width).speed(1.0).range(1.0..=10000.0))
                .changed();
            ui.label("高さ");
            changed |= ui
                .add(egui::DragValue::new(&mut local.height).speed(1.0).range(1.0..=10000.0))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("背景");
            changed |= color_field(ui, &mut local.background);
        });
        if changed {
            self.begin_edit();
            if let Some(f) = edit::frame_mut(self.doc_mut(), &fid) {
                *f = local.clone();
            }
        }
        let is_start = self.doc().start_frame_id == fid;
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!is_start, |ui| {
                if ui.button("開始フレームにする").clicked() {
                    self.mutate(|doc| {
                        edit::set_start_frame(doc, &fid);
                    });
                }
            });
            let can_delete = self.doc().frames.len() > 1;
            ui.add_enabled_ui(can_delete, |ui| {
                if ui.button("フレーム削除").clicked() {
                    self.mutate(|doc| {
                        edit::remove_frame(doc, &fid);
                    });
                    self.selected_node = None;
                }
            });
            if is_start {
                ui.label("（開始フレーム）");
            }
        });
    }
}

/// 色フィールド（カラーボタン＋16進テキスト）。変更があれば true。
fn color_field(ui: &mut egui::Ui, hex: &mut String) -> bool {
    let mut changed = false;
    let mut col = parse_color(hex, 1.0);
    if ui.color_edit_button_srgba(&mut col).changed() {
        *hex = format_color(col);
        changed = true;
    }
    let resp = ui.add(egui::TextEdit::singleline(hex).desired_width(80.0));
    if resp.changed() {
        changed = true;
    }
    changed
}

impl TokageApp {
    fn right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("props")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.strong("プロパティ");
                ui.separator();
                let (Some(fid), Some(nid)) =
                    (self.current_frame_id(), self.selected_node.clone())
                else {
                    ui.label("ノードを選択してください。");
                    return;
                };
                let Some(node) = self
                    .doc()
                    .frames
                    .iter()
                    .find(|f| f.id == fid)
                    .and_then(|f| f.nodes.iter().find(|n| n.base().id == nid))
                    .cloned()
                else {
                    ui.label("ノードを選択してください。");
                    return;
                };
                self.node_props(ui, &fid, node);
            });
    }

    /// 選択ノードのプロパティ編集。ローカルコピーを編集し、変化があれば文書へ書き戻す
    /// （連続編集として履歴に coalesce される）。
    fn node_props(&mut self, ui: &mut egui::Ui, frame_id: &str, node: Node) {
        let before = node.clone();
        let mut local = node;
        let nid = local.base().id.clone();

        ui.horizontal(|ui| {
            ui.label("種別");
            ui.monospace(local.kind_name());
        });
        ui.horizontal(|ui| {
            ui.label("名前");
            ui.text_edit_singleline(&mut local.base_mut().name);
        });
        // 位置・サイズ
        {
            let b = local.base_mut();
            egui::Grid::new("geom").num_columns(4).show(ui, |ui| {
                ui.label("x");
                ui.add(egui::DragValue::new(&mut b.x).speed(1.0));
                ui.label("y");
                ui.add(egui::DragValue::new(&mut b.y).speed(1.0));
                ui.end_row();
                ui.label("幅");
                ui.add(
                    egui::DragValue::new(&mut b.width)
                        .speed(1.0)
                        .range(MIN_NODE_SIZE..=10000.0),
                );
                ui.label("高さ");
                ui.add(
                    egui::DragValue::new(&mut b.height)
                        .speed(1.0)
                        .range(MIN_NODE_SIZE..=10000.0),
                );
                ui.end_row();
                ui.label("回転");
                let mut rot = b.rotation_or_default();
                if ui
                    .add(egui::DragValue::new(&mut rot).speed(1.0).range(-360.0..=360.0))
                    .changed()
                {
                    b.rotation = Some(rot);
                }
                ui.label("不透明度");
                let mut op = b.opacity_or_default();
                if ui
                    .add(egui::DragValue::new(&mut op).speed(0.01).range(0.0..=1.0))
                    .changed()
                {
                    b.opacity = Some(op);
                }
                ui.end_row();
            });
            let mut vis = b.visible_or_default();
            if ui.checkbox(&mut vis, "表示する").changed() {
                b.visible = Some(vis);
            }
        }

        ui.separator();
        match &mut local {
            Node::Rect(n) => {
                ui.horizontal(|ui| {
                    ui.label("塗り");
                    color_field(ui, &mut n.fill);
                });
                ui.horizontal(|ui| {
                    ui.label("角丸");
                    ui.add(
                        egui::DragValue::new(&mut n.corner_radius)
                            .speed(1.0)
                            .range(0.0..=1000.0),
                    );
                });
                let mut has_stroke = n.stroke.is_some();
                if ui.checkbox(&mut has_stroke, "枠線").changed() {
                    n.stroke = if has_stroke {
                        Some("#000000".to_string())
                    } else {
                        None
                    };
                }
                if let Some(stroke) = &mut n.stroke {
                    ui.horizontal(|ui| {
                        ui.label("枠線色");
                        color_field(ui, stroke);
                    });
                    let mut w = n.stroke_width.unwrap_or(1.0);
                    ui.horizontal(|ui| {
                        ui.label("枠線幅");
                        if ui
                            .add(egui::DragValue::new(&mut w).speed(0.5).range(0.0..=100.0))
                            .changed()
                        {
                            n.stroke_width = Some(w);
                        }
                    });
                }
            }
            Node::Text(n) => {
                ui.label("テキスト");
                ui.add(
                    egui::TextEdit::multiline(&mut n.text)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );
                ui.horizontal(|ui| {
                    ui.label("文字サイズ");
                    ui.add(
                        egui::DragValue::new(&mut n.font_size)
                            .speed(1.0)
                            .range(1.0..=500.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("文字色");
                    color_field(ui, &mut n.fill);
                });
                ui.horizontal(|ui| {
                    ui.label("書体");
                    let mut style = n.font_style.clone().unwrap_or_else(|| "normal".into());
                    egui::ComboBox::from_id_salt("font_style")
                        .selected_text(style.clone())
                        .show_ui(ui, |ui| {
                            for s in ["normal", "bold"] {
                                ui.selectable_value(&mut style, s.to_string(), s);
                            }
                        });
                    n.font_style = Some(style);
                });
                ui.horizontal(|ui| {
                    ui.label("揃え");
                    let mut align = n.align.unwrap_or(tokage_core::TextAlign::Left);
                    egui::ComboBox::from_id_salt("text_align")
                        .selected_text(match align {
                            tokage_core::TextAlign::Left => "left",
                            tokage_core::TextAlign::Center => "center",
                            tokage_core::TextAlign::Right => "right",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut align, tokage_core::TextAlign::Left, "left");
                            ui.selectable_value(
                                &mut align,
                                tokage_core::TextAlign::Center,
                                "center",
                            );
                            ui.selectable_value(&mut align, tokage_core::TextAlign::Right, "right");
                        });
                    n.align = Some(align);
                });
            }
            Node::Button(n) => {
                ui.horizontal(|ui| {
                    ui.label("ラベル");
                    ui.text_edit_singleline(&mut n.label);
                });
                ui.horizontal(|ui| {
                    ui.label("塗り");
                    color_field(ui, &mut n.fill);
                });
                ui.horizontal(|ui| {
                    ui.label("文字色");
                    color_field(ui, &mut n.text_color);
                });
                ui.horizontal(|ui| {
                    ui.label("角丸");
                    ui.add(
                        egui::DragValue::new(&mut n.corner_radius)
                            .speed(1.0)
                            .range(0.0..=1000.0),
                    );
                    ui.label("文字サイズ");
                    ui.add(
                        egui::DragValue::new(&mut n.font_size)
                            .speed(1.0)
                            .range(1.0..=500.0),
                    );
                });
            }
            Node::View3d(n) => {
                ui.horizontal(|ui| {
                    ui.label("アセット");
                    let assets = self.doc().assets.clone();
                    let cur = n
                        .asset_id
                        .as_ref()
                        .and_then(|id| assets.iter().find(|a| &a.id == id))
                        .map(|a| a.name.clone())
                        .unwrap_or_else(|| "(なし)".to_string());
                    let mut asset_id = n.asset_id.clone();
                    egui::ComboBox::from_id_salt("asset_id")
                        .selected_text(cur)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut asset_id, None, "(なし)");
                            for a in &assets {
                                let has_blob = self.assets.contains_key(&a.id);
                                let label = if has_blob {
                                    a.name.clone()
                                } else {
                                    format!("{} (実体なし)", a.name)
                                };
                                ui.selectable_value(&mut asset_id, Some(a.id.clone()), label);
                            }
                        });
                    n.asset_id = asset_id;
                });
                ui.horizontal(|ui| {
                    ui.label("カメラ");
                    let mut cam = n.active_camera_id.clone();
                    let cur_name = n
                        .cameras
                        .iter()
                        .find(|c| c.id == cam)
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| cam.clone());
                    egui::ComboBox::from_id_salt("active_camera")
                        .selected_text(cur_name)
                        .show_ui(ui, |ui| {
                            for c in &n.cameras {
                                ui.selectable_value(&mut cam, c.id.clone(), c.name.clone());
                            }
                        });
                    n.active_camera_id = cam;
                });
                let mut auto = n.auto_rotate.unwrap_or(false);
                if ui.checkbox(&mut auto, "自動回転").changed() {
                    n.auto_rotate = Some(auto);
                }
                let mut orbit_on = self.orbit_node.as_deref() == Some(nid.as_str());
                if ui
                    .checkbox(&mut orbit_on, "3D操作（ドラッグで回転 / ホイールでズーム）")
                    .changed()
                {
                    self.orbit_node = if orbit_on { Some(nid.clone()) } else { None };
                }
                ui.label("カメラプリセット:");
                for c in &n.cameras {
                    ui.monospace(format!(
                        "{}: pos [{}, {}, {}] target [{}, {}, {}]",
                        c.name,
                        c.position[0],
                        c.position[1],
                        c.position[2],
                        c.target[0],
                        c.target[1],
                        c.target[2]
                    ));
                }
            }
        }

        ui.separator();
        self.actions_editor(ui, &mut local);

        if local != before {
            self.begin_edit();
            if let Some(target) = edit::node_mut(self.doc_mut(), frame_id, &nid) {
                *target = local;
            }
        }
    }

    /// アクション一覧の編集（追加・削除・パラメータ変更）。対象ノードのローカルコピーを書き換える。
    fn actions_editor(&mut self, ui: &mut egui::Ui, local: &mut Node) {
        ui.strong("アクション（クリック時）");
        let doc = self.doc().clone();
        let frame = self.current_frame().cloned();
        let mut actions = local.base().actions.clone().unwrap_or_default();
        let mut remove_idx: Option<usize> = None;

        for (i, action) in actions.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.monospace(format!("#{i}"));
                match action {
                    Action::Navigate { frame_id } => {
                        ui.label("navigate →");
                        let cur = doc
                            .frames
                            .iter()
                            .find(|f| &f.id == frame_id)
                            .map(|f| f.name.clone())
                            .unwrap_or_else(|| "(なし)".into());
                        egui::ComboBox::from_id_salt(format!("act_nav_{i}"))
                            .selected_text(cur)
                            .show_ui(ui, |ui| {
                                for f in &doc.frames {
                                    ui.selectable_value(frame_id, f.id.clone(), f.name.clone());
                                }
                            });
                    }
                    Action::ToggleVisible { node_id } => {
                        ui.label("toggleVisible →");
                        let label_of = |id: &str| {
                            doc.frames
                                .iter()
                                .flat_map(|f| f.nodes.iter().map(move |n| (f, n)))
                                .find(|(_, n)| n.base().id == id)
                                .map(|(f, n)| format!("{} / {}", f.name, n.base().name))
                                .unwrap_or_else(|| "(なし)".into())
                        };
                        let cur = label_of(node_id);
                        egui::ComboBox::from_id_salt(format!("act_tgl_{i}"))
                            .selected_text(cur)
                            .show_ui(ui, |ui| {
                                for f in &doc.frames {
                                    for n in &f.nodes {
                                        ui.selectable_value(
                                            node_id,
                                            n.base().id.clone(),
                                            format!("{} / {}", f.name, n.base().name),
                                        );
                                    }
                                }
                            });
                    }
                    Action::SetCamera { view_node_id, camera_id } => {
                        ui.label("setCamera →");
                        let views: Vec<&tokage_core::View3DNode> = doc
                            .frames
                            .iter()
                            .flat_map(|f| f.nodes.iter())
                            .filter_map(|n| match n {
                                Node::View3d(v) => Some(v),
                                _ => None,
                            })
                            .collect();
                        let cur_view = views
                            .iter()
                            .find(|v| &v.base.id == view_node_id)
                            .map(|v| v.base.name.clone())
                            .unwrap_or_else(|| "(なし)".into());
                        egui::ComboBox::from_id_salt(format!("act_cam_view_{i}"))
                            .selected_text(cur_view)
                            .show_ui(ui, |ui| {
                                for v in &views {
                                    ui.selectable_value(
                                        view_node_id,
                                        v.base.id.clone(),
                                        v.base.name.clone(),
                                    );
                                }
                            });
                        let cams: Vec<(String, String)> = views
                            .iter()
                            .find(|v| &v.base.id == view_node_id)
                            .map(|v| {
                                v.cameras
                                    .iter()
                                    .map(|c| (c.id.clone(), c.name.clone()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        let cur_cam = cams
                            .iter()
                            .find(|(id, _)| id == camera_id)
                            .map(|(_, name)| name.clone())
                            .unwrap_or_else(|| camera_id.clone());
                        egui::ComboBox::from_id_salt(format!("act_cam_{i}"))
                            .selected_text(cur_cam)
                            .show_ui(ui, |ui| {
                                for (id, name) in &cams {
                                    ui.selectable_value(camera_id, id.clone(), name.clone());
                                }
                            });
                    }
                    Action::OpenUrl { url } => {
                        ui.label("openUrl →");
                        ui.add(egui::TextEdit::singleline(url).desired_width(140.0));
                    }
                }
                if ui.button("削除").clicked() {
                    remove_idx = Some(i);
                }
            });
        }
        if let Some(i) = remove_idx {
            actions.remove(i);
        }
        ui.horizontal(|ui| {
            ui.label("追加:");
            for (label, kind) in [
                ("navigate", "navigate"),
                ("setCamera", "setCamera"),
                ("toggleVisible", "toggleVisible"),
                ("openUrl", "openUrl"),
            ] {
                if ui.button(label).clicked() {
                    if let Some(f) = &frame {
                        if let Some(a) = tokage_core::default_action(kind, &doc, f) {
                            actions.push(a);
                        }
                    }
                }
            }
        });
        local.base_mut().actions = if actions.is_empty() && local.base().actions.is_none() {
            None
        } else {
            Some(actions)
        };
    }
}

impl TokageApp {
    /// 中央キャンバス。編集モードでは選択/移動/リサイズ、再生モードではクリックで Action 実行。
    fn canvas(&mut self, ui: &mut egui::Ui) {
        let frame = if self.mode == Mode::Play {
            let play_frame = self
                .play
                .as_ref()
                .and_then(|p| {
                    self.doc()
                        .frames
                        .iter()
                        .find(|f| f.id == p.frame_id)
                        .cloned()
                });
            play_frame.or_else(|| self.current_frame().cloned())
        } else {
            self.current_frame().cloned()
        };
        let Some(frame) = frame else {
            ui.centered_and_justified(|ui| {
                ui.label("フレームがありません。");
            });
            return;
        };
        ui.horizontal(|ui| {
            ui.strong(&frame.name);
            ui.monospace(format!("{} x {}", frame.width, frame.height));
            if self.mode == Mode::Play {
                ui.label("（再生中: ノードをクリックするとアクションが実行されます）");
            }
        });
        ui.separator();

        let avail = ui.available_size();
        let (resp, painter) = ui.allocate_painter(avail, egui::Sense::click_and_drag());
        let area = resp.rect;
        let scale = ((area.width() / frame.width as f32).min(area.height() / frame.height as f32))
            .min(1.5)
            .max(0.05);
        let size = Vec2::new(frame.width as f32 * scale, frame.height as f32 * scale);
        let origin = area.center() - size / 2.0;
        let to_rect = |x: f64, y: f64, w: f64, h: f64| {
            Rect::from_min_size(
                Pos2::new(origin.x + x as f32 * scale, origin.y + y as f32 * scale),
                Vec2::new(w as f32 * scale, h as f32 * scale),
            )
        };
        let to_doc = |p: Pos2| {
            Pos2::new(
                (p.x - origin.x) / scale,
                (p.y - origin.y) / scale,
            )
        };

        // フレーム地
        let frame_rect = Rect::from_min_size(origin, size);
        painter.rect_filled(frame_rect, Rounding::ZERO, parse_color(&frame.background, 1.0));
        painter.rect_stroke(
            frame_rect,
            Rounding::ZERO,
            Stroke::new(1.0_f32, Color32::from_gray(90)),
        );

        // ---- インタラクション ----
        let pointer = resp.interact_pointer_pos();
        if self.mode == Mode::Edit {
            if resp.drag_started() {
                if let Some(p) = pointer {
                    let dp = to_doc(p);
                    // 選択中ノードのハンドルを優先
                    let mut started = false;
                    if let Some(sel_id) = self.selected_node.clone() {
                        if let Some(node) =
                            frame.nodes.iter().find(|n| n.base().id == sel_id)
                        {
                            let b = node.base();
                            let r = to_rect(b.x, b.y, b.width, b.height);
                            if let Some(h) = handle_at(p, r) {
                                self.drag = Some(DragState {
                                    node_id: sel_id,
                                    handle: Some(h),
                                    start_pointer: dp,
                                    start_rect: RectF {
                                        x: b.x,
                                        y: b.y,
                                        w: b.width,
                                        h: b.height,
                                    },
                                    orbit: false,
                                });
                                self.begin_edit();
                                started = true;
                            }
                        }
                    }
                    if !started {
                        if let Some(node) = hit_node(&frame, dp, true) {
                            let b = node.base();
                            self.selected_node = Some(b.id.clone());
                            // 3D 操作モード中の view3d は移動ではなくオービットにする
                            let orbit = matches!(node, Node::View3d(_))
                                && self.orbit_node.as_deref() == Some(b.id.as_str());
                            self.drag = Some(DragState {
                                node_id: b.id.clone(),
                                handle: None,
                                start_pointer: dp,
                                start_rect: RectF {
                                    x: b.x,
                                    y: b.y,
                                    w: b.width,
                                    h: b.height,
                                },
                                orbit,
                            });
                            if !orbit {
                                self.begin_edit();
                            }
                        }
                    }
                }
            }
            if resp.dragged() {
                let orbit_drag = self
                    .drag
                    .as_ref()
                    .filter(|d| d.orbit)
                    .map(|d| d.node_id.clone());
                if let Some(nid) = orbit_drag {
                    let delta = resp.drag_delta();
                    let st = self.orbits.entry(nid).or_default();
                    st.yaw -= delta.x * 0.008;
                    st.pitch = (st.pitch - delta.y * 0.008).clamp(-1.35, 1.35);
                } else if let (Some(d), Some(p)) = (&self.drag, pointer) {
                    let dp = to_doc(p);
                    let dx = (dp.x - d.start_pointer.x) as f64;
                    let dy = (dp.y - d.start_pointer.y) as f64;
                    let s = d.start_rect;
                    let (nx, ny, nw, nh) = match d.handle {
                        None => (s.x + dx, s.y + dy, s.w, s.h),
                        Some(h) => resize_rect(s, h, dx, dy),
                    };
                    let fid = frame.id.clone();
                    let nid = d.node_id.clone();
                    if let Some(node) = edit::node_mut(self.doc_mut(), &fid, &nid) {
                        let b = node.base_mut();
                        b.x = nx;
                        b.y = ny;
                        b.width = nw;
                        b.height = nh;
                    }
                }
            }
            if resp.drag_stopped() {
                self.drag = None;
                self.end_edit();
            }
            if resp.clicked() {
                if let Some(p) = pointer {
                    let dp = to_doc(p);
                    self.selected_node =
                        hit_node(&frame, dp, true).map(|n| n.base().id.clone());
                }
            }
        } else if resp.clicked() {
            // 再生モード: 最前面の表示中ノードのアクションを実行
            if let Some(p) = pointer {
                let dp = to_doc(p);
                let hit = frame
                    .nodes
                    .iter()
                    .rev()
                    .find(|n| {
                        let b = n.base();
                        self.effective_visible(&b.id)
                            && dp.x >= b.x as f32
                            && dp.x <= (b.x + b.width) as f32
                            && dp.y >= b.y as f32
                            && dp.y <= (b.y + b.height) as f32
                    })
                    .cloned();
                if let Some(node) = hit {
                    self.selected_node = Some(node.base().id.clone());
                    if let Some(actions) = node.base().actions.clone() {
                        if !actions.is_empty() {
                            self.run_actions(&actions);
                        }
                    }
                }
            }
        }

        // 3D 操作モード中の view3d 上でのホイールズーム
        if self.mode == Mode::Edit {
            if let (Some(oid), Some(p)) = (self.orbit_node.clone(), pointer) {
                if let Some(node) = frame.nodes.iter().find(|n| n.base().id == oid) {
                    let b = node.base();
                    let r = to_rect(b.x, b.y, b.width, b.height);
                    if r.contains(p) {
                        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                        if scroll.abs() > 0.01 {
                            let st = self.orbits.entry(oid).or_default();
                            st.dist = (st.dist * (1.0 - scroll * 0.002)).clamp(0.3, 3.0);
                        }
                    }
                }
            }
        }

        // ---- 描画 ----
        for node in &frame.nodes {
            let base = node.base();
            let visible = if self.mode == Mode::Play {
                self.effective_visible(&base.id)
            } else {
                base.visible_or_default()
            };
            if !visible {
                continue;
            }
            let r = to_rect(base.x, base.y, base.width, base.height);
            let opacity = base.opacity_or_default();
            match node {
                Node::Rect(n) => {
                    let rounding = Rounding::same((n.corner_radius as f32 * scale).min(24.0));
                    painter.rect_filled(r, rounding, parse_color(&n.fill, opacity));
                    if let Some(stroke) = &n.stroke {
                        let w = n.stroke_width.unwrap_or(1.0) as f32 * scale;
                        if w > 0.0 {
                            painter.rect_stroke(
                                r,
                                rounding,
                                Stroke::new(w, parse_color(stroke, opacity)),
                            );
                        }
                    }
                }
                Node::Text(n) => {
                    let font = FontId::proportional(n.font_size as f32 * scale);
                    let color = parse_color(&n.fill, opacity);
                    let galley = painter.layout(n.text.clone(), font, color, r.width());
                    let x = match n.align.unwrap_or(tokage_core::TextAlign::Left) {
                        tokage_core::TextAlign::Left => r.min.x,
                        tokage_core::TextAlign::Center => r.center().x - galley.size().x / 2.0,
                        tokage_core::TextAlign::Right => r.max.x - galley.size().x,
                    };
                    let mut pos = Pos2::new(x, r.min.y);
                    if n.font_style.as_deref() == Some("bold") {
                        // egui の painter では太字指定が限られるため、僅かにずらして二重描きする
                        painter.galley(pos, galley.clone(), color);
                        pos.x += 0.7;
                    }
                    painter.galley(pos, galley, color);
                }
                Node::Button(n) => {
                    let rounding = Rounding::same((n.corner_radius as f32 * scale).min(24.0));
                    painter.rect_filled(r, rounding, parse_color(&n.fill, opacity));
                    let font = FontId::proportional(n.font_size as f32 * scale);
                    let color = parse_color(&n.text_color, opacity);
                    let galley = painter.layout_no_wrap(n.label.clone(), font, color);
                    painter.galley(r.center() - galley.size() / 2.0, galley, color);
                }
                Node::View3d(n) => {
                    painter.rect_filled(r, Rounding::ZERO, Color32::from_rgb(0xdd, 0xe1, 0xe6));
                    // 実描画できる場合は wgpu コールバックを差し込み、
                    // できない場合は理由つきプレースホルダにする
                    let mut label: Option<String> = None;
                    match &n.asset_id {
                        None => {
                            label = Some(format!("3D VIEW\n{}\nGLB 未設定", base.name));
                        }
                        Some(asset_id) => match self.asset_arcs.get(asset_id) {
                            None => {
                                label = Some(format!(
                                    "3D VIEW\n{}\nアセット実体なし",
                                    base.name
                                ));
                            }
                            Some((model_key, bytes)) => {
                                let load_err = self.render_errors.lock().ok().and_then(|m| {
                                    m.get(model_key).cloned()
                                });
                                if let Some(e) = load_err {
                                    label = Some(format!(
                                        "3D VIEW\n{}\n読み込み失敗: {}",
                                        base.name, e
                                    ));
                                } else if let Some(surface_format) = self.surface_format {
                                    let (eye0, target) = self.view3d_camera(n);
                                    let mut yaw = 0.0f32;
                                    let mut pitch = 0.0f32;
                                    let mut dist = 1.0f32;
                                    if n.auto_rotate.unwrap_or(false) {
                                        let dt = ui.input(|i| i.unstable_dt).min(0.1);
                                        let e =
                                            self.spin.entry(base.id.clone()).or_insert(0.0);
                                        *e += dt * 0.6;
                                        yaw += *e;
                                        ui.ctx().request_repaint();
                                    } else if let Some(s) = self.spin.get(&base.id) {
                                        yaw += *s;
                                    }
                                    if self.mode == Mode::Edit {
                                        if let Some(o) = self.orbits.get(&base.id) {
                                            yaw += o.yaw;
                                            pitch += o.pitch;
                                            dist = o.dist;
                                        }
                                    }
                                    let eye =
                                        math3d::apply_orbit(target, eye0, yaw, pitch, dist);
                                    let aspect = (r.width() / r.height().max(1.0)).max(0.05);
                                    let mvp = math3d::perspective(
                                        40f32.to_radians(),
                                        aspect,
                                        0.01,
                                        100.0,
                                    )
                                    .mul(&math3d::look_at(eye, target, [0.0, 1.0, 0.0]));
                                    let ppp = ui.ctx().pixels_per_point();
                                    painter.add(egui_wgpu::Callback::new_paint_callback(
                                        r,
                                        render3d::View3dCallback {
                                            target_key: base.id.clone(),
                                            model_key: model_key.clone(),
                                            bytes: bytes.clone(),
                                            mvp,
                                            size_px: [
                                                (r.width() * ppp).round() as u32,
                                                (r.height() * ppp).round() as u32,
                                            ],
                                            surface_format,
                                            errors: self.render_errors.clone(),
                                        },
                                    ));
                                } else {
                                    label = Some(format!("3D VIEW\n{}", base.name));
                                }
                            }
                        },
                    }
                    painter.rect_stroke(
                        r,
                        Rounding::ZERO,
                        Stroke::new(1.0_f32, Color32::from_gray(120)),
                    );
                    if let Some(label) = label {
                        let galley = painter.layout(
                            label,
                            FontId::proportional(13.0 * scale.max(0.6)),
                            Color32::from_rgb(0x3a, 0x3b, 0x42),
                            r.width(),
                        );
                        painter.galley(
                            r.center() - galley.size() / 2.0,
                            galley,
                            Color32::from_gray(60),
                        );
                    }
                }
            }
            if self.mode == Mode::Edit
                && self.selected_node.as_deref() == Some(base.id.as_str())
            {
                painter.rect_stroke(r, Rounding::ZERO, Stroke::new(2.0_f32, ACCENT));
                // リサイズハンドル
                for c in handle_points(r) {
                    let hr = Rect::from_center_size(c, Vec2::splat(HANDLE_SIZE));
                    painter.rect_filled(hr, Rounding::ZERO, Color32::WHITE);
                    painter.rect_stroke(hr, Rounding::ZERO, Stroke::new(1.0_f32, ACCENT));
                }
            }
        }
    }
}

/// 文書座標でのヒットテスト（最前面優先）。edit=true のとき非表示ノードも対象に含める。
fn hit_node(frame: &Frame, p: Pos2, _edit: bool) -> Option<&Node> {
    frame.nodes.iter().rev().find(|n| {
        let b = n.base();
        p.x >= b.x as f32
            && p.x <= (b.x + b.width) as f32
            && p.y >= b.y as f32
            && p.y <= (b.y + b.height) as f32
    })
}

fn handle_points(r: Rect) -> [Pos2; 8] {
    let c = r.center();
    [
        r.min,                                    // NW
        Pos2::new(c.x, r.min.y),                  // N
        Pos2::new(r.max.x, r.min.y),              // NE
        Pos2::new(r.max.x, c.y),                  // E
        r.max,                                    // SE
        Pos2::new(c.x, r.max.y),                  // S
        Pos2::new(r.min.x, r.max.y),              // SW
        Pos2::new(r.min.x, c.y),                  // W
    ]
}

fn handle_at(p: Pos2, r: Rect) -> Option<Handle> {
    let pts = handle_points(r);
    let handles = [
        Handle::NW,
        Handle::N,
        Handle::NE,
        Handle::E,
        Handle::SE,
        Handle::S,
        Handle::SW,
        Handle::W,
    ];
    for (i, c) in pts.iter().enumerate() {
        let hr = Rect::from_center_size(*c, Vec2::splat(HANDLE_SIZE + 4.0));
        if hr.contains(p) {
            return Some(handles[i]);
        }
    }
    None
}

/// ハンドル方向に応じて開始矩形へドラッグ量を適用する（最小サイズでクランプ）。
fn resize_rect(s: RectF, h: Handle, dx: f64, dy: f64) -> (f64, f64, f64, f64) {
    let (mut x, mut y, mut w, mut hh) = (s.x, s.y, s.w, s.h);
    let right = s.x + s.w;
    let bottom = s.y + s.h;
    match h {
        Handle::E | Handle::NE | Handle::SE => w = (s.w + dx).max(MIN_NODE_SIZE),
        _ => {}
    }
    match h {
        Handle::S | Handle::SE | Handle::SW => hh = (s.h + dy).max(MIN_NODE_SIZE),
        _ => {}
    }
    match h {
        Handle::W | Handle::NW | Handle::SW => {
            w = (s.w - dx).max(MIN_NODE_SIZE);
            x = right - w;
        }
        _ => {}
    }
    match h {
        Handle::N | Handle::NE | Handle::NW => {
            hh = (s.h - dy).max(MIN_NODE_SIZE);
            y = bottom - hh;
        }
        _ => {}
    }
    (x, y, w, hh)
}
