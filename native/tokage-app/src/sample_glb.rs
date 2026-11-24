//! 同梱サンプル用スマホ GLB の手続き生成。
//! Web 版 `buildSamplePhoneGLB`（three.js）と同じ形状・色を GLB バイナリとして組み立てる。
//! Native 版はサンプルの GLB 実体を同梱していないため、不足時にここで補完する。

struct Part {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
    material: usize,
}

fn translate(mut part: Part, dx: f32, dy: f32, dz: f32) -> Part {
    for p in &mut part.positions {
        p[0] += dx;
        p[1] += dy;
        p[2] += dz;
    }
    part
}

/// 直方体（面ごとに頂点を分けた 24 頂点）。
fn make_box(w: f32, h: f32, d: f32, material: usize) -> Part {
    let (x, y, z) = (w / 2.0, h / 2.0, d / 2.0);
    // (法線, 4隅) を面ごとに定義する
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        ([0.0, 0.0, 1.0], [[-x, -y, z], [x, -y, z], [x, y, z], [-x, y, z]]), // +z
        ([0.0, 0.0, -1.0], [[x, -y, -z], [-x, -y, -z], [-x, y, -z], [x, y, -z]]), // -z
        ([1.0, 0.0, 0.0], [[x, -y, z], [x, -y, -z], [x, y, -z], [x, y, z]]), // +x
        ([-1.0, 0.0, 0.0], [[-x, -y, -z], [-x, -y, z], [-x, y, z], [-x, y, -z]]), // -x
        ([0.0, 1.0, 0.0], [[-x, y, z], [x, y, z], [x, y, -z], [-x, y, -z]]), // +y
        ([0.0, -1.0, 0.0], [[-x, -y, -z], [x, -y, -z], [x, -y, z], [-x, -y, z]]), // -y
    ];
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    for (n, corners) in faces {
        let base = positions.len() as u32;
        for c in corners {
            positions.push(c);
            normals.push(n);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Part {
        positions,
        normals,
        indices,
        material,
    }
}

/// Z 軸方向の円柱（three.js の CylinderGeometry を X に 90° 回したものと等価）。
fn make_cylinder_z(r: f32, h: f32, segments: usize, material: usize) -> Part {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let hz = h / 2.0;
    // 側面
    for i in 0..segments {
        let a0 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
        let (c0, s0) = (a0.cos(), a0.sin());
        let (c1, s1) = (a1.cos(), a1.sin());
        let base = positions.len() as u32;
        positions.extend_from_slice(&[
            [r * c0, r * s0, -hz],
            [r * c1, r * s1, -hz],
            [r * c1, r * s1, hz],
            [r * c0, r * s0, hz],
        ]);
        normals.extend_from_slice(&[[c0, s0, 0.0], [c1, s1, 0.0], [c1, s1, 0.0], [c0, s0, 0.0]]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    // 蓋（+z / -z）
    for (z, n, flip) in [(hz, [0.0, 0.0, 1.0], false), (-hz, [0.0, 0.0, -1.0], true)] {
        let center = positions.len() as u32;
        positions.push([0.0, 0.0, z]);
        normals.push(n);
        let ring_start = positions.len() as u32;
        for i in 0..segments {
            let a = (i as f32 / segments as f32) * std::f32::consts::TAU;
            positions.push([r * a.cos(), r * a.sin(), z]);
            normals.push(n);
        }
        for i in 0..segments {
            let a = ring_start + i as u32;
            let b = ring_start + ((i + 1) % segments) as u32;
            if flip {
                indices.extend_from_slice(&[center, b, a]);
            } else {
                indices.extend_from_slice(&[center, a, b]);
            }
        }
    }
    Part {
        positions,
        normals,
        indices,
        material,
    }
}

/// スカルプト用: 細分化した球プリミティブの GLB を生成する。
/// Web 版 `buildSphereGLB`（SphereGeometry(0.8, 96, 72) を y+0.8 へ移動）に対応する。
pub fn build_sphere_glb() -> Vec<u8> {
    let radius = 0.8f32;
    let (wseg, hseg) = (96usize, 72usize);
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    for iy in 0..=hseg {
        let phi = std::f32::consts::PI * (iy as f32 / hseg as f32);
        for ix in 0..=wseg {
            let theta = std::f32::consts::TAU * (ix as f32 / wseg as f32);
            let n = [
                phi.sin() * theta.cos(),
                phi.cos(),
                phi.sin() * theta.sin(),
            ];
            normals.push(n);
            positions.push([radius * n[0], radius * n[1] + 0.8, radius * n[2]]);
        }
    }
    let stride = wseg + 1;
    let mut indices: Vec<u32> = Vec::new();
    for iy in 0..hseg {
        for ix in 0..wseg {
            let a = (iy * stride + ix) as u32;
            let b = a + 1;
            let c = a + stride as u32;
            let d = c + 1;
            // 外向き（CCW）に揃える
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }

    let pos_bytes: Vec<u8> = positions
        .iter()
        .flat_map(|p| p.iter().flat_map(|v| v.to_le_bytes()))
        .collect();
    let nor_bytes: Vec<u8> = normals
        .iter()
        .flat_map(|p| p.iter().flat_map(|v| v.to_le_bytes()))
        .collect();
    let idx_bytes: Vec<u8> = indices.iter().flat_map(|i| i.to_le_bytes()).collect();

    let mut bin: Vec<u8> = Vec::new();
    let mut buffer_views: Vec<serde_json::Value> = Vec::new();
    let mut add_view = |bin: &mut Vec<u8>, data: &[u8], target: u64| -> usize {
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let offset = bin.len();
        bin.extend_from_slice(data);
        buffer_views.push(serde_json::json!({
            "buffer": 0, "byteOffset": offset, "byteLength": data.len(), "target": target,
        }));
        buffer_views.len() - 1
    };
    let pos_view = add_view(&mut bin, &pos_bytes, 34962);
    let nor_view = add_view(&mut bin, &nor_bytes, 34962);
    let idx_view = add_view(&mut bin, &idx_bytes, 34963);

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &positions {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    let json = serde_json::json!({
        "asset": { "version": "2.0", "generator": "tokagetype native sculpt sphere" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0, "name": "SculptSphere" }],
        "meshes": [{
            "name": "SculptSphere",
            "primitives": [{
                "attributes": { "POSITION": 0, "NORMAL": 1 },
                "indices": 2, "material": 0, "mode": 4,
            }],
        }],
        "materials": [
            { "name": "sculpt", "pbrMetallicRoughness": { "baseColorFactor": [0.722, 0.769, 0.831, 1.0], "metallicFactor": 0.05, "roughnessFactor": 0.6 } },
        ],
        "accessors": [
            { "bufferView": pos_view, "componentType": 5126, "count": positions.len(), "type": "VEC3", "min": min, "max": max },
            { "bufferView": nor_view, "componentType": 5126, "count": normals.len(), "type": "VEC3" },
            { "bufferView": idx_view, "componentType": 5125, "count": indices.len(), "type": "SCALAR" },
        ],
        "bufferViews": buffer_views,
        "buffers": [{ "byteLength": bin.len() }],
    });

    let mut json_bytes = serde_json::to_vec(&json).unwrap_or_default();
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }
    let mut bin_padded = bin;
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

/// サンプルスマホの GLB バイト列を生成する。
pub fn build_sample_phone_glb() -> Vec<u8> {
    let parts = vec![
        // 本体
        translate(make_box(0.75, 1.6, 0.08, 0), 0.0, 0.8, 0.0),
        // 画面
        translate(make_box(0.68, 1.5, 0.005, 1), 0.0, 0.8, 0.043),
        // 背面カメラ台座 / レンズ / ロゴ
        translate(make_cylinder_z(0.11, 0.02, 32, 2), -0.18, 1.38, -0.05),
        translate(make_cylinder_z(0.06, 0.025, 32, 3), -0.18, 1.38, -0.055),
        translate(make_cylinder_z(0.09, 0.006, 32, 4), 0.0, 0.75, -0.043),
    ];

    // BIN バッファと accessor / bufferView を組み立てる
    let mut bin: Vec<u8> = Vec::new();
    let mut buffer_views: Vec<serde_json::Value> = Vec::new();
    let mut accessors: Vec<serde_json::Value> = Vec::new();
    let mut primitives: Vec<serde_json::Value> = Vec::new();

    let push_view = |bin: &mut Vec<u8>,
                     views: &mut Vec<serde_json::Value>,
                     data: &[u8],
                     target: u64|
     -> usize {
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let offset = bin.len();
        bin.extend_from_slice(data);
        views.push(serde_json::json!({
            "buffer": 0, "byteOffset": offset, "byteLength": data.len(), "target": target,
        }));
        views.len() - 1
    };

    for part in &parts {
        let pos_bytes: Vec<u8> = part
            .positions
            .iter()
            .flat_map(|p| p.iter().flat_map(|v| v.to_le_bytes()))
            .collect();
        let nor_bytes: Vec<u8> = part
            .normals
            .iter()
            .flat_map(|p| p.iter().flat_map(|v| v.to_le_bytes()))
            .collect();
        let idx_bytes: Vec<u8> = part
            .indices
            .iter()
            .flat_map(|i| i.to_le_bytes())
            .collect();
        let pos_view = push_view(&mut bin, &mut buffer_views, &pos_bytes, 34962);
        let nor_view = push_view(&mut bin, &mut buffer_views, &nor_bytes, 34962);
        let idx_view = push_view(&mut bin, &mut buffer_views, &idx_bytes, 34963);

        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for p in &part.positions {
            for i in 0..3 {
                min[i] = min[i].min(p[i]);
                max[i] = max[i].max(p[i]);
            }
        }
        let pos_acc = accessors.len();
        accessors.push(serde_json::json!({
            "bufferView": pos_view, "componentType": 5126, "count": part.positions.len(),
            "type": "VEC3", "min": min, "max": max,
        }));
        let nor_acc = accessors.len();
        accessors.push(serde_json::json!({
            "bufferView": nor_view, "componentType": 5126, "count": part.normals.len(), "type": "VEC3",
        }));
        let idx_acc = accessors.len();
        accessors.push(serde_json::json!({
            "bufferView": idx_view, "componentType": 5125, "count": part.indices.len(), "type": "SCALAR",
        }));
        primitives.push(serde_json::json!({
            "attributes": { "POSITION": pos_acc, "NORMAL": nor_acc },
            "indices": idx_acc, "material": part.material, "mode": 4,
        }));
    }

    let json = serde_json::json!({
        "asset": { "version": "2.0", "generator": "tokagetype native sample" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0, "name": "sample-phone" }],
        "meshes": [{ "name": "sample-phone", "primitives": primitives }],
        "materials": [
            { "name": "body", "pbrMetallicRoughness": { "baseColorFactor": [0.169, 0.227, 0.333, 1.0], "metallicFactor": 0.6, "roughnessFactor": 0.35 } },
            { "name": "screen", "pbrMetallicRoughness": { "baseColorFactor": [0.043, 0.082, 0.149, 1.0], "metallicFactor": 0.0, "roughnessFactor": 0.4 }, "emissiveFactor": [0.498, 0.831, 1.0] },
            { "name": "cameraBase", "pbrMetallicRoughness": { "baseColorFactor": [0.063, 0.078, 0.094, 1.0], "metallicFactor": 0.4, "roughnessFactor": 0.5 } },
            { "name": "cameraLens", "pbrMetallicRoughness": { "baseColorFactor": [0.227, 0.431, 0.647, 1.0], "metallicFactor": 0.2, "roughnessFactor": 0.25 } },
            { "name": "backLogo", "pbrMetallicRoughness": { "baseColorFactor": [0.843, 0.890, 0.957, 1.0], "metallicFactor": 0.3, "roughnessFactor": 0.4 } },
        ],
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{ "byteLength": bin.len() }],
    });

    let mut json_bytes = serde_json::to_vec(&json).unwrap_or_default();
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }
    let mut bin_padded = bin;
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
