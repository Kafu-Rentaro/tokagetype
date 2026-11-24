//! スカルプト: Web 版 `src/sculpt/brush.ts` + `SculptView.tsx` に対応する Native 実装。
//!
//! - 対象は GLB の最初のメッシュ（走査順で最初に現れる、頂点を持つプリミティブ）。
//!   Web 版 GLTFLoader が作る最初の Mesh と同じ定義。
//! - 変形はレンダラと同じフィット空間（最大寸法 1.6 正規化・底面 y=0）で行う。
//!   Web 版はワールド空間で変形するが、フィットは一様スケール+平行移動なので
//!   ブラシの振る舞い（法線方向・フォールオフ）は等価になる。
//! - ブラシ範囲の頂点検索は BVH の代わりに空間グリッド（セル = 半径）を使う。
//! - 書き戻しは元 GLB の JSON/BIN を保持したまま、対象プリミティブの
//!   POSITION / NORMAL アクセサを新しいデータで差し替える（他メッシュ・
//!   テクスチャ・マテリアルはバイト単位で保持される）。

use std::collections::HashMap;

use crate::glb::{compute_normals, Vertex};
use crate::math3d::{v_add, v_cross, v_dot, v_len, v_norm, v_scale, v_sub, Mat4, Vec3};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BrushMode {
    /// 法線方向へ引き出す（膨らませる）
    Pull,
    /// 法線と逆方向へ押し込む（へこませる）
    Push,
    /// 範囲内頂点の平均位置へ寄せる
    Smooth,
}

/// スカルプト対象プリミティブの作業データ（位置・法線はフィット空間）。
pub struct SculptTarget {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    /// 書き戻し先（元 GLB の meshes[mesh_index].primitives[prim_index]）
    pub mesh_index: usize,
    pub prim_index: usize,
    /// フィット変換: fitted = (world - [center_x, min_y, center_z]) * scale
    pub fit_scale: f32,
    pub fit_center_x: f32,
    pub fit_min_y: f32,
    pub fit_center_z: f32,
    /// ローカル → ワールド変換の逆行列（書き戻し時にローカル空間へ戻す）
    pub world_inverse: Mat4,
}

impl SculptTarget {
    /// レンダラ（動的頂点差し替え）へ渡す頂点列を作る。
    pub fn vertices(&self) -> Vec<Vertex> {
        (0..self.positions.len())
            .map(|i| Vertex {
                pos: self.positions[i],
                normal: self.normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                uv: self.uvs.get(i).copied().unwrap_or([0.0, 0.0]),
                color: self.colors.get(i).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]),
            })
            .collect()
    }

    /// Undo/Redo 用: 位置を差し替え、法線を再計算する。
    pub fn set_positions(&mut self, positions: Vec<Vec3>) {
        debug_assert_eq!(positions.len(), self.positions.len());
        self.positions = positions;
        self.normals = compute_normals(&self.positions, &self.indices);
    }

    /// フィット空間 → ワールド空間。
    fn to_world(&self, p: Vec3) -> Vec3 {
        [
            p[0] / self.fit_scale + self.fit_center_x,
            p[1] / self.fit_scale + self.fit_min_y,
            p[2] / self.fit_scale + self.fit_center_z,
        ]
    }

    /// フィット空間の現在位置を元 GLB のローカル空間へ戻す。
    pub fn local_positions(&self) -> Vec<Vec3> {
        self.positions
            .iter()
            .map(|p| self.world_inverse.transform_point(self.to_world(*p)))
            .collect()
    }
}

struct FirstPrim {
    local_positions: Vec<Vec3>,
    indices: Vec<u32>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    world: Mat4,
    mesh_index: usize,
    prim_index: usize,
}

/// GLB からスカルプト対象（最初のメッシュ）を抽出する。
/// フィット規則は glb.rs の parse_glb と同一（モデル全体の bbox で正規化）。
pub fn extract_target(bytes: &[u8]) -> Result<SculptTarget, String> {
    let (doc, buffers, _images) =
        gltf::import_slice(bytes).map_err(|e| format!("GLB の読み込みに失敗: {e}"))?;
    let scene = doc
        .default_scene()
        .or_else(|| doc.scenes().next())
        .ok_or_else(|| "シーンがありません".to_string())?;

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let mut first: Option<FirstPrim> = None;
    for node in scene.nodes() {
        walk_extract(
            &node,
            &Mat4::identity(),
            &buffers,
            &mut min,
            &mut max,
            &mut first,
        );
    }
    let first = first.ok_or_else(|| "メッシュが含まれていません".to_string())?;

    let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    let max_dim = size[0].max(size[1]).max(size[2]);
    let scale = if max_dim > 1e-6 { 1.6 / max_dim } else { 1.0 };
    let center_x = (min[0] + max[0]) * 0.5;
    let center_z = (min[2] + max[2]) * 0.5;
    let fit = |p: Vec3| -> Vec3 {
        [
            (p[0] - center_x) * scale,
            (p[1] - min[1]) * scale,
            (p[2] - center_z) * scale,
        ]
    };
    let positions: Vec<Vec3> = first
        .local_positions
        .iter()
        .map(|p| fit(first.world.transform_point(*p)))
        .collect();
    let normals = compute_normals(&positions, &first.indices);
    Ok(SculptTarget {
        positions,
        normals,
        uvs: first.uvs,
        colors: first.colors,
        indices: first.indices,
        mesh_index: first.mesh_index,
        prim_index: first.prim_index,
        fit_scale: scale,
        fit_center_x: center_x,
        fit_min_y: min[1],
        fit_center_z: center_z,
        world_inverse: first.world.inverse(),
    })
}

fn walk_extract(
    node: &gltf::Node,
    parent: &Mat4,
    buffers: &[gltf::buffer::Data],
    min: &mut [f32; 3],
    max: &mut [f32; 3],
    first: &mut Option<FirstPrim>,
) {
    let world = parent.mul(&Mat4::from_gltf(node.transform().matrix()));
    if let Some(mesh) = node.mesh() {
        for (prim_index, prim) in mesh.primitives().enumerate() {
            let reader = prim.reader(|b| buffers.get(b.index()).map(|d| &d[..]));
            let Some(local_positions): Option<Vec<Vec3>> =
                reader.read_positions().map(|it| it.collect())
            else {
                continue;
            };
            if local_positions.is_empty() {
                continue;
            }
            for p in &local_positions {
                let w = world.transform_point(*p);
                for i in 0..3 {
                    min[i] = min[i].min(w[i]);
                    max[i] = max[i].max(w[i]);
                }
            }
            if first.is_none() {
                let indices: Vec<u32> = match reader.read_indices() {
                    Some(idx) => idx.into_u32().collect(),
                    None => (0..local_positions.len() as u32).collect(),
                };
                let count = local_positions.len();
                let uvs: Vec<[f32; 2]> = match reader.read_tex_coords(0) {
                    Some(tc) => tc.into_f32().collect(),
                    None => vec![[0.0, 0.0]; count],
                };
                let colors: Vec<[f32; 4]> = match reader.read_colors(0) {
                    Some(c) => c.into_rgba_f32().collect(),
                    None => vec![[1.0, 1.0, 1.0, 1.0]; count],
                };
                *first = Some(FirstPrim {
                    local_positions,
                    indices,
                    uvs,
                    colors,
                    world,
                    mesh_index: mesh.index(),
                    prim_index,
                });
            }
        }
    }
    for child in node.children() {
        walk_extract(&child, &world, buffers, min, max, first);
    }
}

// ---- ブラシ範囲検索（空間グリッド） ----

fn grid_cell(p: Vec3, cell: f32) -> [i32; 3] {
    [
        (p[0] / cell).floor() as i32,
        (p[1] / cell).floor() as i32,
        (p[2] / cell).floor() as i32,
    ]
}

/// 半径内の頂点インデックスを返す。セルサイズ = 半径とし、隣接 27 セルを調べる
/// （Web 版 BVH shapecast + 距離フィルタと同じ結果集合になる）。
pub fn find_vertices_in_radius(positions: &[Vec3], center: Vec3, radius: f32) -> Vec<usize> {
    if positions.is_empty() || radius <= 0.0 {
        return Vec::new();
    }
    let cell = radius;
    let mut grid: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
    for (i, p) in positions.iter().enumerate() {
        grid.entry(grid_cell(*p, cell)).or_default().push(i as u32);
    }
    let c = grid_cell(center, cell);
    let r2 = radius * radius;
    let mut out = Vec::new();
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                if let Some(list) = grid.get(&[c[0] + dx, c[1] + dy, c[2] + dz]) {
                    for &i in list {
                        let p = positions[i as usize];
                        let d = v_sub(p, center);
                        if v_dot(d, d) <= r2 {
                            out.push(i as usize);
                        }
                    }
                }
            }
        }
    }
    out.sort_unstable();
    out
}

// ---- ブラシ適用（Web 版 applyBrush と同一式） ----

/// ブラシを1回適用し、影響した頂点数を返す。center はフィット空間。
pub fn apply_brush(
    target: &mut SculptTarget,
    center: Vec3,
    radius: f32,
    strength: f32,
    mode: BrushMode,
) -> usize {
    let indices = find_vertices_in_radius(&target.positions, center, radius);
    if indices.is_empty() {
        return 0;
    }
    match mode {
        BrushMode::Smooth => {
            let mut avg = [0.0f32; 3];
            for &i in &indices {
                avg = v_add(avg, target.positions[i]);
            }
            avg = v_scale(avg, 1.0 / indices.len() as f32);
            for &i in &indices {
                let v = target.positions[i];
                let d = v_len(v_sub(v, center));
                let falloff = (1.0 - d / radius).max(0.0);
                let t = (strength * falloff).min(1.0);
                target.positions[i] = v_add(v, v_scale(v_sub(avg, v), t));
            }
        }
        BrushMode::Pull | BrushMode::Push => {
            let sign = if mode == BrushMode::Pull { 1.0 } else { -1.0 };
            for &i in &indices {
                let v = target.positions[i];
                let d = v_len(v_sub(v, center));
                let falloff = (1.0 - d / radius).max(0.0);
                let smooth = falloff * falloff * (3.0 - 2.0 * falloff);
                let n = target
                    .normals
                    .get(i)
                    .copied()
                    .unwrap_or_else(|| v_norm(v_sub(v, center)));
                let amount = strength * smooth * sign * radius * 0.35;
                target.positions[i] = v_add(v, v_scale(n, amount));
            }
        }
    }
    target.normals = compute_normals(&target.positions, &target.indices);
    indices.len()
}

// ---- レイキャスト ----

pub struct RayHit {
    pub point: Vec3,
    /// ヒット面の法線（将来のブラシ向き表示用に保持）
    #[allow(dead_code)]
    pub normal: Vec3,
    pub dist: f32,
}

/// 対象メッシュへのレイキャスト（Möller–Trumbore、両面対象）。最も近いヒットを返す。
pub fn raycast(target: &SculptTarget, origin: Vec3, dir: Vec3) -> Option<RayHit> {
    let mut best: Option<RayHit> = None;
    for tri in target.indices.chunks_exact(3) {
        let a = target.positions[tri[0] as usize];
        let b = target.positions[tri[1] as usize];
        let c = target.positions[tri[2] as usize];
        let e1 = v_sub(b, a);
        let e2 = v_sub(c, a);
        let p = v_cross(dir, e2);
        let det = v_dot(e1, p);
        if det.abs() < 1e-9 {
            continue;
        }
        let inv = 1.0 / det;
        let tvec = v_sub(origin, a);
        let u = v_dot(tvec, p) * inv;
        if !(0.0..=1.0).contains(&u) {
            continue;
        }
        let q = v_cross(tvec, e1);
        let v = v_dot(dir, q) * inv;
        if v < 0.0 || u + v > 1.0 {
            continue;
        }
        let t = v_dot(e2, q) * inv;
        if t <= 1e-6 {
            continue;
        }
        if best.as_ref().map(|h| t < h.dist).unwrap_or(true) {
            best = Some(RayHit {
                point: v_add(origin, v_scale(dir, t)),
                normal: v_norm(v_cross(e1, e2)),
                dist: t,
            });
        }
    }
    best
}

// ---- GLB 書き戻し ----

/// GLB コンテナを (JSON ルート, BIN バイト列) に分解する。
fn split_glb(bytes: &[u8]) -> Result<(serde_json::Value, Vec<u8>), String> {
    if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
        return Err("GLB ではありません".into());
    }
    let read_u32 = |o: usize| -> u32 {
        u32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]])
    };
    let json_len = read_u32(12) as usize;
    if &bytes[16..20] != b"JSON" || 20 + json_len > bytes.len() {
        return Err("GLB の JSON チャンクが不正です".into());
    }
    let root: serde_json::Value =
        serde_json::from_slice(&bytes[20..20 + json_len]).map_err(|e| format!("JSON 読込失敗: {e}"))?;
    let mut bin = Vec::new();
    let bin_off = 20 + json_len;
    if bin_off + 8 <= bytes.len() && &bytes[bin_off + 4..bin_off + 8] == b"BIN\0" {
        let bin_len = read_u32(bin_off) as usize;
        let start = bin_off + 8;
        if start + bin_len <= bytes.len() {
            bin = bytes[start..start + bin_len].to_vec();
        }
    }
    Ok((root, bin))
}

/// GLB コンテナを組み立てる。
fn join_glb(root: &serde_json::Value, bin: &[u8]) -> Vec<u8> {
    let mut json_bytes = serde_json::to_vec(root).unwrap_or_default();
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }
    let mut bin_padded = bin.to_vec();
    while bin_padded.len() % 4 != 0 {
        bin_padded.push(0);
    }
    let total = 12 + 8 + json_bytes.len() + 8 + bin_padded.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json_bytes);
    out.extend_from_slice(&(bin_padded.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin_padded);
    out
}

fn append_f32x3(bin: &mut Vec<u8>, views: &mut Vec<serde_json::Value>, data: &[[f32; 3]]) -> usize {
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let offset = bin.len();
    for v in data {
        for c in v {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    views.push(serde_json::json!({
        "buffer": 0, "byteOffset": offset, "byteLength": data.len() * 12, "target": 34962,
    }));
    views.len() - 1
}

/// スカルプト結果を元 GLB へ書き戻す。対象プリミティブの POSITION / NORMAL を
/// 新しいアクセサに差し替え、他のデータは保持する。法線はローカル空間で再計算する。
pub fn write_back(source: &[u8], target: &SculptTarget) -> Result<Vec<u8>, String> {
    let (mut root, mut bin) = split_glb(source)?;
    if root
        .get("buffers")
        .and_then(|b| b.get(0))
        .and_then(|b| b.get("uri"))
        .is_some()
    {
        return Err("外部バッファ参照の GLB には書き戻せません".into());
    }
    let local_positions = target.local_positions();
    let local_normals = compute_normals(&local_positions, &target.indices);

    let mut views = root
        .get("bufferViews")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    let mut accessors = root
        .get("accessors")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();

    let pos_view = append_f32x3(&mut bin, &mut views, &local_positions);
    let nor_view = append_f32x3(&mut bin, &mut views, &local_normals);

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &local_positions {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    let pos_acc = accessors.len();
    accessors.push(serde_json::json!({
        "bufferView": pos_view, "componentType": 5126, "count": local_positions.len(),
        "type": "VEC3", "min": min, "max": max,
    }));
    let nor_acc = accessors.len();
    accessors.push(serde_json::json!({
        "bufferView": nor_view, "componentType": 5126, "count": local_normals.len(), "type": "VEC3",
    }));

    let prim = root
        .get_mut("meshes")
        .and_then(|m| m.get_mut(target.mesh_index))
        .and_then(|m| m.get_mut("primitives"))
        .and_then(|p| p.get_mut(target.prim_index))
        .ok_or_else(|| "書き戻し先のプリミティブが見つかりません".to_string())?;
    let attrs = prim
        .get_mut("attributes")
        .and_then(|a| a.as_object_mut())
        .ok_or_else(|| "attributes がありません".to_string())?;
    attrs.insert("POSITION".into(), serde_json::json!(pos_acc));
    attrs.insert("NORMAL".into(), serde_json::json!(nor_acc));

    root["bufferViews"] = serde_json::Value::Array(views);
    root["accessors"] = serde_json::Value::Array(accessors);
    if let Some(buf0) = root
        .get_mut("buffers")
        .and_then(|b| b.get_mut(0))
        .and_then(|b| b.as_object_mut())
    {
        buf0.insert("byteLength".into(), serde_json::json!(bin.len()));
    }
    Ok(join_glb(&root, &bin))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_glb::{build_sample_phone_glb, build_sphere_glb};

    /// 決定的な疑似乱数（LCG）。
    fn lcg(seed: &mut u64) -> f32 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((*seed >> 33) as f32) / (u32::MAX as f32)
    }

    #[test]
    fn grid_query_matches_brute_force() {
        let mut seed = 42u64;
        let positions: Vec<Vec3> = (0..3000)
            .map(|_| {
                [
                    lcg(&mut seed) * 2.0 - 1.0,
                    lcg(&mut seed) * 2.0 - 1.0,
                    lcg(&mut seed) * 2.0 - 1.0,
                ]
            })
            .collect();
        for (center, radius) in [
            ([0.0, 0.0, 0.0], 0.4f32),
            ([0.5, -0.3, 0.2], 0.25),
            ([-0.9, 0.9, -0.9], 0.6),
        ] {
            let got = find_vertices_in_radius(&positions, center, radius);
            let want: Vec<usize> = positions
                .iter()
                .enumerate()
                .filter(|(_, p)| v_len(v_sub(**p, center)) <= radius)
                .map(|(i, _)| i)
                .collect();
            assert_eq!(got, want, "center={center:?} radius={radius}");
        }
    }

    /// z=0 平面の 5x5 グリッド（法線 +z）を SculptTarget として作る。
    fn plane_target() -> SculptTarget {
        let mut positions = Vec::new();
        for y in 0..5 {
            for x in 0..5 {
                positions.push([x as f32 * 0.1, y as f32 * 0.1, 0.0]);
            }
        }
        let mut indices = Vec::new();
        for y in 0..4u32 {
            for x in 0..4u32 {
                let a = y * 5 + x;
                let b = a + 1;
                let c = a + 5;
                let d = c + 1;
                indices.extend_from_slice(&[a, b, c, b, d, c]);
            }
        }
        let normals = compute_normals(&positions, &indices);
        SculptTarget {
            positions,
            normals,
            uvs: Vec::new(),
            colors: Vec::new(),
            indices,
            mesh_index: 0,
            prim_index: 0,
            fit_scale: 1.0,
            fit_center_x: 0.0,
            fit_min_y: 0.0,
            fit_center_z: 0.0,
            world_inverse: Mat4::identity(),
        }
    }

    #[test]
    fn pull_moves_vertices_along_normal() {
        let mut t = plane_target();
        let before = t.positions.clone();
        // 中心頂点 (0.2, 0.2, 0) = index 12
        let n = apply_brush(&mut t, [0.2, 0.2, 0.0], 0.15, 1.0, BrushMode::Pull);
        assert!(n > 1);
        // 中心: falloff=1 → amount = 1.0 * 1 * 0.15 * 0.35 = 0.0525 (+z)
        assert!((t.positions[12][2] - 0.0525).abs() < 1e-5, "z={}", t.positions[12][2]);
        assert_eq!(t.positions[12][0], 0.2);
        assert_eq!(t.positions[12][1], 0.2);
        // 範囲外の頂点は不変
        assert_eq!(t.positions[0], before[0]);
        assert_eq!(t.positions[24], before[24]);
    }

    #[test]
    fn push_moves_vertices_against_normal() {
        let mut t = plane_target();
        apply_brush(&mut t, [0.2, 0.2, 0.0], 0.15, 1.0, BrushMode::Push);
        assert!((t.positions[12][2] + 0.0525).abs() < 1e-5);
    }

    #[test]
    fn smooth_pulls_outlier_toward_average() {
        let mut t = plane_target();
        t.positions[12][2] = 0.1; // 中心だけ飛び出させる
        t.normals = compute_normals(&t.positions, &t.indices);
        let before_center = t.positions[12][2];
        apply_brush(&mut t, [0.2, 0.2, 0.0], 0.15, 0.5, BrushMode::Smooth);
        assert!(
            t.positions[12][2] < before_center,
            "中心が平均へ寄るはず: {} -> {}",
            before_center,
            t.positions[12][2]
        );
        // 範囲外（距離 ≈0.28）は不変
        assert_eq!(t.positions[0], [0.0, 0.0, 0.0]);
        assert_eq!(t.positions[24], [0.4, 0.4, 0.0]);
    }

    #[test]
    fn raycast_hits_sphere_front() {
        let bytes = build_sphere_glb();
        let t = extract_target(&bytes).expect("球が抽出できること");
        // 球: 半径 0.8・中心 y=0.8、フィット後も scale=1 で不変。前面は z=+0.8
        let hit = raycast(&t, [0.0, 0.8, 5.0], [0.0, 0.0, -1.0]).expect("ヒットするはず");
        assert!((hit.point[2] - 0.8).abs() < 0.02, "z={}", hit.point[2]);
        assert!((hit.dist - 4.2).abs() < 0.02, "dist={}", hit.dist);
        assert!(hit.normal[2] > 0.9);
        // 外れたレイはヒットしない
        assert!(raycast(&t, [3.0, 0.8, 5.0], [0.0, 0.0, -1.0]).is_none());
    }

    /// 全プリミティブのローカル頂点位置を走査順に返す（書き戻し比較用）。
    /// フィット空間は書き戻しのたびに再計算されるため、保持の検証は
    /// フィットに依存しないローカル空間で行う。
    fn all_local_positions(bytes: &[u8]) -> Vec<Vec<Vec3>> {
        let (doc, buffers, _) = gltf::import_slice(bytes).expect("GLB として読めること");
        fn walk(
            node: &gltf::Node,
            buffers: &[gltf::buffer::Data],
            out: &mut Vec<Vec<Vec3>>,
        ) {
            if let Some(mesh) = node.mesh() {
                for prim in mesh.primitives() {
                    let reader = prim.reader(|b| buffers.get(b.index()).map(|d| &d[..]));
                    if let Some(pos) = reader.read_positions() {
                        out.push(pos.collect());
                    }
                }
            }
            for child in node.children() {
                walk(&child, buffers, out);
            }
        }
        let scene = doc
            .default_scene()
            .or_else(|| doc.scenes().next())
            .unwrap();
        let mut out = Vec::new();
        for n in scene.nodes() {
            walk(&n, &buffers, &mut out);
        }
        out
    }

    #[test]
    fn write_back_roundtrip_preserves_other_primitives() {
        let bytes = build_sample_phone_glb();
        let orig_prims = all_local_positions(&bytes);
        let mut t = extract_target(&bytes).expect("抽出");
        assert_eq!(t.mesh_index, 0);
        assert_eq!(t.prim_index, 0);
        // 本体は 24 頂点の直方体なので、最も手前の頂点そのものにブラシを当てる
        let front = *t
            .positions
            .iter()
            .max_by(|a, b| a[2].partial_cmp(&b[2]).unwrap())
            .unwrap();
        let n = apply_brush(&mut t, front, 0.3, 1.0, BrushMode::Pull);
        assert!(n > 0);
        let out = write_back(&bytes, &t).expect("書き戻し");
        // 出力が有効な GLB として読み直せる
        let new_model = crate::glb::parse_glb(&out).expect("書き戻し後 GLB");
        assert_eq!(new_model.primitives.len(), orig_prims.len());
        let new_prims = all_local_positions(&out);
        assert_eq!(new_prims.len(), orig_prims.len());
        // 対象外プリミティブ（画面・レンズ等）はローカル頂点が完全一致
        for pi in 1..orig_prims.len() {
            assert_eq!(orig_prims[pi], new_prims[pi], "prim {pi} が変わってはいけない");
        }
        // 対象プリミティブは少なくとも 1 頂点が変わっている
        let diffs = orig_prims[0]
            .iter()
            .zip(&new_prims[0])
            .filter(|(a, b)| a != b)
            .count();
        assert!(diffs >= 1, "対象プリミティブに変形が残るはず");
    }

    #[test]
    fn write_back_sphere_roundtrip() {
        let bytes = build_sphere_glb();
        let mut t = extract_target(&bytes).expect("抽出");
        let count = t.positions.len();
        // 前面の頂点 (0, 0.8, 0.8) のインデックス
        let front_idx = t
            .positions
            .iter()
            .position(|p| p[0].abs() < 1e-6 && (p[1] - 0.8).abs() < 1e-6 && (p[2] - 0.8).abs() < 1e-6)
            .expect("前面頂点があるはず");
        let back_idx = t
            .positions
            .iter()
            .position(|p| p[0].abs() < 1e-6 && (p[1] - 0.8).abs() < 1e-6 && (p[2] + 0.8).abs() < 1e-6)
            .expect("背面頂点があるはず");
        apply_brush(&mut t, [0.0, 0.8, 0.8], 0.4, 1.2, BrushMode::Pull);
        let out = write_back(&bytes, &t).expect("書き戻し");
        let t2 = extract_target(&out).expect("再抽出");
        assert_eq!(t2.positions.len(), count);
        // ローカル空間で比較する（フィットは再計算されるため）
        let local = t2.local_positions();
        assert!(
            local[front_idx][2] > 0.9,
            "前面が膨らんで残るはず: z={}",
            local[front_idx][2]
        );
        assert!(
            (local[back_idx][2] + 0.8).abs() < 1e-4,
            "背面は不変のはず: z={}",
            local[back_idx][2]
        );
    }

    #[test]
    fn mat4_inverse_roundtrip() {
        // 平行移動 + 回転 + 一様スケールの合成で往復を確認
        let bytes = build_sphere_glb();
        let t = extract_target(&bytes).expect("抽出");
        let _ = t; // フィット/逆行列の存在確認のみ（球は単位変換）
        let m = Mat4::from_gltf([
            [0.0, -2.0, 0.0, 0.0],
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 2.0, 0.0],
            [1.0, 2.0, 3.0, 1.0],
        ]);
        let p = [0.3, -0.7, 1.1];
        let q = m.inverse().transform_point(m.transform_point(p));
        for i in 0..3 {
            assert!((q[i] - p[i]).abs() < 1e-5, "i={i}: {} vs {}", q[i], p[i]);
        }
    }
}

