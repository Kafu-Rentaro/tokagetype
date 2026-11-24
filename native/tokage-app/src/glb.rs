//! GLB 読込: gltf クレートでパースし、CPU 側メッシュへ変換する。
//! ワールド変換を焼き込んだうえで、Web 版 fitTransform と同じ規則で自動フィットする
//! （最大寸法を 1.6 に正規化、底面 y=0、x/z 中心を 0）。

use crate::math3d::{v_norm, Mat4};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

pub struct CpuTexture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct CpuPrimitive {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub base_color: [f32; 4],
    pub emissive: [f32; 3],
    pub texture: Option<CpuTexture>,
    pub emissive_texture: Option<CpuTexture>,
}

pub struct CpuModel {
    pub primitives: Vec<CpuPrimitive>,
}

struct RawPrim {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
    base_color: [f32; 4],
    emissive: [f32; 3],
    texture: Option<CpuTexture>,
    emissive_texture: Option<CpuTexture>,
}

pub fn parse_glb(bytes: &[u8]) -> Result<CpuModel, String> {
    let (doc, buffers, images) =
        gltf::import_slice(bytes).map_err(|e| format!("GLB の読み込みに失敗: {e}"))?;

    let mut raw: Vec<RawPrim> = Vec::new();
    let scene = doc
        .default_scene()
        .or_else(|| doc.scenes().next())
        .ok_or_else(|| "シーンがありません".to_string())?;
    for node in scene.nodes() {
        walk_node(&node, &Mat4::identity(), &buffers, &images, &mut raw);
    }
    if raw.is_empty() {
        return Err("メッシュが含まれていません".to_string());
    }

    // 全頂点のバウンディングボックス（ワールド座標）→ Web 版と同じフィット
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &raw {
        for v in &p.positions {
            for i in 0..3 {
                min[i] = min[i].min(v[i]);
                max[i] = max[i].max(v[i]);
            }
        }
    }
    let size = [
        max[0] - min[0],
        max[1] - min[1],
        max[2] - min[2],
    ];
    let max_dim = size[0].max(size[1]).max(size[2]);
    let scale = if max_dim > 1e-6 { 1.6 / max_dim } else { 1.0 };
    let center_x = (min[0] + max[0]) * 0.5;
    let center_z = (min[2] + max[2]) * 0.5;
    let fit = |p: [f32; 3]| -> [f32; 3] {
        [
            (p[0] - center_x) * scale,
            (p[1] - min[1]) * scale,
            (p[2] - center_z) * scale,
        ]
    };

    let primitives = raw
        .into_iter()
        .map(|p| {
            let vertices = p
                .positions
                .iter()
                .enumerate()
                .map(|(i, pos)| Vertex {
                    pos: fit(*pos),
                    normal: p.normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                    uv: p.uvs.get(i).copied().unwrap_or([0.0, 0.0]),
                    color: p.colors.get(i).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]),
                })
                .collect();
            CpuPrimitive {
                vertices,
                indices: p.indices,
                base_color: p.base_color,
                emissive: p.emissive,
                texture: p.texture,
                emissive_texture: p.emissive_texture,
            }
        })
        .collect();
    Ok(CpuModel { primitives })
}

fn walk_node(
    node: &gltf::Node,
    parent: &Mat4,
    buffers: &[gltf::buffer::Data],
    images: &[gltf::image::Data],
    out: &mut Vec<RawPrim>,
) {
    let world = parent.mul(&Mat4::from_gltf(node.transform().matrix()));
    if let Some(mesh) = node.mesh() {
        for prim in mesh.primitives() {
            if let Some(p) = read_primitive(&prim, &world, buffers, images) {
                out.push(p);
            }
        }
    }
    for child in node.children() {
        walk_node(&child, &world, buffers, images, out);
    }
}

fn read_primitive(
    prim: &gltf::Primitive,
    world: &Mat4,
    buffers: &[gltf::buffer::Data],
    images: &[gltf::image::Data],
) -> Option<RawPrim> {
    let reader = prim.reader(|b| buffers.get(b.index()).map(|d| &d[..]));
    let positions: Vec<[f32; 3]> = reader.read_positions()?.collect();
    if positions.is_empty() {
        return None;
    }
    let indices: Vec<u32> = match reader.read_indices() {
        Some(idx) => idx.into_u32().collect(),
        None => (0..positions.len() as u32).collect(),
    };
    let normals: Vec<[f32; 3]> = match reader.read_normals() {
        Some(n) => n
            .map(|v| v_norm(world.transform_dir(v)))
            .collect(),
        None => compute_normals(&positions, &indices),
    };
    let uvs: Vec<[f32; 2]> = match reader.read_tex_coords(0) {
        Some(tc) => tc.into_f32().collect(),
        None => vec![[0.0, 0.0]; positions.len()],
    };
    let colors: Vec<[f32; 4]> = match reader.read_colors(0) {
        Some(c) => c.into_rgba_f32().collect(),
        None => vec![[1.0, 1.0, 1.0, 1.0]; positions.len()],
    };
    let world_positions: Vec<[f32; 3]> =
        positions.iter().map(|p| world.transform_point(*p)).collect();

    let material = prim.material();
    let pbr = material.pbr_metallic_roughness();
    let base_color = pbr.base_color_factor();
    let emissive = material.emissive_factor();
    let texture = pbr
        .base_color_texture()
        .and_then(|info| images.get(info.texture().source().index()))
        .and_then(convert_image);
    let emissive_texture = material
        .emissive_texture()
        .and_then(|info| images.get(info.texture().source().index()))
        .and_then(convert_image);

    Some(RawPrim {
        positions: world_positions,
        normals,
        uvs,
        colors,
        indices,
        base_color,
        emissive,
        texture,
        emissive_texture,
    })
}

/// 法線が無い場合に面法線の面積加重平均でスムーズ法線を作る。
fn compute_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0f32; 3]; positions.len()];
    for tri in indices.chunks_exact(3) {
        let (a, b, c) = (
            positions[tri[0] as usize],
            positions[tri[1] as usize],
            positions[tri[2] as usize],
        );
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        for &i in tri {
            let dst = &mut normals[i as usize];
            dst[0] += n[0];
            dst[1] += n[1];
            dst[2] += n[2];
        }
    }
    normals.iter().map(|n| v_norm(*n)).collect()
}

/// gltf の画像データを RGBA8 へ変換する。対応外フォーマットは None（単色で描く）。
fn convert_image(img: &gltf::image::Data) -> Option<CpuTexture> {
    use gltf::image::Format;
    let (w, h) = (img.width, img.height);
    let rgba: Vec<u8> = match img.format {
        Format::R8G8B8A8 => img.pixels.clone(),
        Format::R8G8B8 => img
            .pixels
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        Format::R8G8 => img
            .pixels
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[1], 0, 255])
            .collect(),
        Format::R8 => img
            .pixels
            .iter()
            .flat_map(|p| [*p, *p, *p, 255])
            .collect(),
        Format::R16G16B16A16 => img
            .pixels
            .chunks_exact(8)
            .flat_map(|p| [p[1], p[3], p[5], p[7]]) // 上位バイトを採用
            .collect(),
        Format::R16G16B16 => img
            .pixels
            .chunks_exact(6)
            .flat_map(|p| [p[1], p[3], p[5], 255])
            .collect(),
        _ => return None,
    };
    if rgba.len() != (w * h * 4) as usize {
        return None;
    }
    Some(CpuTexture {
        width: w,
        height: h,
        rgba,
    })
}

