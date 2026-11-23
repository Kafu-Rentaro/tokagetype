//! tokagetype Native 版（tokage-app）の土台。
//!
//! 最小機能（docs/SPEC.md「Native 版」参照）:
//! - プロジェクト JSON（Web 版 doc.json）を開く（同梱サンプル / パス指定）
//! - フレーム一覧とノード一覧の表示
//! - 選択フレームの 2D プレビュー描画（rect / text / button / view3d プレースホルダ）
//!
//! 編集機能はまだ持たない（ロードマップは SPEC 参照）。

use eframe::egui;
use egui::{Color32, FontId, Pos2, Rect, Rounding, Stroke, Vec2};
use tokage_core::{validate_doc, Frame, Level, Node, ProjectDoc};

const SAMPLE_JSON: &str = include_str!("../../tokage-core/tests/fixtures/sample-project.json");
const ACCENT: Color32 = Color32::from_rgb(0xff, 0x4d, 0x1c);

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
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

struct TokageApp {
    doc: Option<ProjectDoc>,
    source_label: String,
    path_input: String,
    frame_idx: usize,
    selected_node: Option<String>,
    status: String,
}

impl Default for TokageApp {
    /// 起動時は同梱サンプル（Web 版フィクスチャ）を開いた状態にする。
    /// 引数に JSON パスがあればそちらを優先する。
    fn default() -> Self {
        let mut app = TokageApp {
            doc: None,
            source_label: String::new(),
            path_input: String::new(),
            frame_idx: 0,
            selected_node: None,
            status: String::new(),
        };
        if let Some(path) = std::env::args().nth(1) {
            match std::fs::read_to_string(&path) {
                Ok(json) => app.load_json(&json, path),
                Err(e) => app.status = format!("ファイルを開けません: {e}"),
            }
        } else {
            app.load_json(SAMPLE_JSON, "bundled sample (Web version fixture)".into());
        }
        app
    }
}

impl TokageApp {
    fn load_json(&mut self, json: &str, label: String) {
        match ProjectDoc::from_json(json) {
            Ok(doc) => {
                let issues = validate_doc(&doc);
                let errors = issues.iter().filter(|i| i.level == Level::Error).count();
                let warnings = issues.iter().filter(|i| i.level == Level::Warning).count();
                self.status = format!(
                    "検証: エラー {errors} 件 / 警告 {warnings} 件（全 {} 件）",
                    issues.len()
                );
                self.frame_idx = doc
                    .frames
                    .iter()
                    .position(|f| f.id == doc.start_frame_id)
                    .unwrap_or(0);
                self.selected_node = None;
                self.source_label = label;
                self.doc = Some(doc);
            }
            Err(e) => {
                self.status = format!("読み込み失敗: {e}");
            }
        }
    }

    fn current_frame(&self) -> Option<&Frame> {
        self.doc.as_ref().and_then(|d| d.frames.get(self.frame_idx))
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

impl eframe::App for TokageApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("tokagetype");
                ui.separator();
                if ui.button("Open sample").clicked() {
                    self.load_json(SAMPLE_JSON, "bundled sample (Web version fixture)".into());
                }
                ui.label("JSON path:");
                ui.add(egui::TextEdit::singleline(&mut self.path_input).desired_width(280.0));
                if ui.button("Open").clicked() {
                    let path = self.path_input.trim().to_string();
                    match std::fs::read_to_string(&path) {
                        Ok(json) => self.load_json(&json, path),
                        Err(e) => self.status = format!("ファイルを開けません: {e}"),
                    }
                }
                if !self.source_label.is_empty() {
                    ui.separator();
                    ui.label(&self.source_label);
                }
            });
            if !self.status.is_empty() {
                ui.label(&self.status);
            }
        });

        egui::SidePanel::left("frames")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                let Some(doc) = self.doc.clone() else {
                    ui.label("No project loaded.\nUse \"Open sample\" or specify a doc.json path.");
                    return;
                };
                ui.label(format!("Project: {}", doc.name));
                ui.separator();
                ui.strong("Frames");
                for (i, f) in doc.frames.iter().enumerate() {
                    let label = if f.id == doc.start_frame_id {
                        format!("{} (start)", f.name)
                    } else {
                        f.name.clone()
                    };
                    if ui.selectable_label(self.frame_idx == i, label).clicked() {
                        self.frame_idx = i;
                        self.selected_node = None;
                    }
                }
                ui.separator();
                ui.strong("Nodes");
                if let Some(frame) = doc.frames.get(self.frame_idx) {
                    // Web 版レイヤー表示と同じく上が手前 = 配列の逆順
                    for node in frame.nodes.iter().rev() {
                        let base = node.base();
                        let text = format!("[{}] {}", node.kind_name(), base.name);
                        if ui
                            .selectable_label(self.selected_node.as_deref() == Some(base.id.as_str()), text)
                            .clicked()
                        {
                            self.selected_node = Some(base.id.clone());
                        }
                    }
                    if frame.nodes.is_empty() {
                        ui.label("(no nodes)");
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let Some(frame) = self.current_frame().cloned() else {
                ui.centered_and_justified(|ui| {
                    ui.label("Preview — load a project to begin.");
                });
                return;
            };
            ui.horizontal(|ui| {
                ui.strong(format!("{} ", frame.name));
                ui.monospace(format!("{} x {}", frame.width, frame.height));
            });
            ui.separator();
            self.draw_preview(ui, &frame);
        });
    }
}

impl TokageApp {
    fn draw_preview(&mut self, ui: &mut egui::Ui, frame: &Frame) {
        let avail = ui.available_size();
        let (resp, painter) = ui.allocate_painter(avail, egui::Sense::click());
        let area = resp.rect;
        let scale = ((area.width() / frame.width as f32)
            .min(area.height() / frame.height as f32))
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

        // フレーム地
        let frame_rect = Rect::from_min_size(origin, size);
        painter.rect_filled(frame_rect, Rounding::ZERO, parse_color(&frame.background, 1.0));
        painter.rect_stroke(frame_rect, Rounding::ZERO, Stroke::new(1.0_f32, Color32::from_gray(90)));

        let click_pos = resp.interact_pointer_pos();
        let mut clicked_node: Option<String> = None;

        for node in &frame.nodes {
            let base = node.base();
            if !base.visible_or_default() {
                continue;
            }
            let r = to_rect(base.x, base.y, base.width, base.height);
            let opacity = base.opacity_or_default();
            if resp.clicked() {
                if let Some(p) = click_pos {
                    if r.contains(p) {
                        clicked_node = Some(base.id.clone()); // 後ろ（手前）が優先されるよう上書き
                    }
                }
            }
            match node {
                Node::Rect(n) => {
                    let rounding = Rounding::same((n.corner_radius as f32 * scale).min(24.0));
                    painter.rect_filled(r, rounding, parse_color(&n.fill, opacity));
                    if let Some(stroke) = &n.stroke {
                        let w = n.stroke_width.unwrap_or(1.0) as f32 * scale;
                        if w > 0.0 {
                            painter.rect_stroke(r, rounding, Stroke::new(w, parse_color(stroke, opacity)));
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
                    painter.galley(Pos2::new(x, r.min.y), galley, color);
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
                    painter.rect_stroke(r, Rounding::ZERO, Stroke::new(1.0_f32, Color32::from_gray(120)));
                    let cam = n
                        .cameras
                        .iter()
                        .find(|c| c.id == n.active_camera_id)
                        .map(|c| c.name.as_str())
                        .unwrap_or("-");
                    let label = format!("3D VIEW\n{}\ncamera: {}", base.name, cam);
                    let galley = painter.layout(
                        label,
                        FontId::proportional(13.0 * scale.max(0.6)),
                        Color32::from_rgb(0x3a, 0x3b, 0x42),
                        r.width(),
                    );
                    painter.galley(r.center() - galley.size() / 2.0, galley, Color32::from_gray(60));
                }
            }
            if self.selected_node.as_deref() == Some(base.id.as_str()) {
                painter.rect_stroke(r, Rounding::ZERO, Stroke::new(2.0_f32, ACCENT));
            }
        }

        if let Some(id) = clicked_node {
            self.selected_node = Some(id);
        }
    }
}
