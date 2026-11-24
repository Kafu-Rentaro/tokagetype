//! 最小限の 3D 数学（列優先 Mat4 / Vec3）。glTF と wgpu の規約に合わせる。

pub type Vec3 = [f32; 3];

pub fn v_sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn v_add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn v_scale(a: Vec3, s: f32) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
pub fn v_dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn v_cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn v_len(a: Vec3) -> f32 {
    v_dot(a, a).sqrt()
}
pub fn v_norm(a: Vec3) -> Vec3 {
    let l = v_len(a);
    if l > 1e-8 {
        v_scale(a, 1.0 / l)
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// 列優先 4x4 行列（m[col * 4 + row]）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4(pub [f32; 16]);

impl Mat4 {
    pub fn identity() -> Self {
        Mat4([
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    /// gltf クレートの transform matrix（列ごとの配列）から変換する。
    pub fn from_gltf(m: [[f32; 4]; 4]) -> Self {
        let mut out = [0.0f32; 16];
        for col in 0..4 {
            for row in 0..4 {
                out[col * 4 + row] = m[col][row];
            }
        }
        Mat4(out)
    }

    pub fn mul(&self, rhs: &Mat4) -> Mat4 {
        let mut out = [0.0f32; 16];
        for col in 0..4 {
            for row in 0..4 {
                let mut s = 0.0;
                for k in 0..4 {
                    s += self.0[k * 4 + row] * rhs.0[col * 4 + k];
                }
                out[col * 4 + row] = s;
            }
        }
        Mat4(out)
    }

    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        let m = &self.0;
        let w = m[3] * p[0] + m[7] * p[1] + m[11] * p[2] + m[15];
        let x = m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12];
        let y = m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13];
        let z = m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14];
        if w.abs() > 1e-8 && (w - 1.0).abs() > 1e-6 {
            [x / w, y / w, z / w]
        } else {
            [x, y, z]
        }
    }

    /// 方向ベクトル（平行移動なし）。正規化は呼び出し側で行う。
    pub fn transform_dir(&self, d: Vec3) -> Vec3 {
        let m = &self.0;
        [
            m[0] * d[0] + m[4] * d[1] + m[8] * d[2],
            m[1] * d[0] + m[5] * d[1] + m[9] * d[2],
            m[2] * d[0] + m[6] * d[1] + m[10] * d[2],
        ]
    }
}

/// 右手系 look-at（カメラは -Z 方向を見る）。
pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
    let f = v_norm(v_sub(target, eye));
    let s = v_norm(v_cross(f, up));
    let u = v_cross(s, f);
    let mut m = Mat4::identity();
    // 列優先: 列0 = s, 列1 = u, 列2 = -f
    m.0[0] = s[0];
    m.0[1] = u[0];
    m.0[2] = -f[0];
    m.0[4] = s[1];
    m.0[5] = u[1];
    m.0[6] = -f[1];
    m.0[8] = s[2];
    m.0[9] = u[2];
    m.0[10] = -f[2];
    m.0[12] = -v_dot(s, eye);
    m.0[13] = -v_dot(u, eye);
    m.0[14] = v_dot(f, eye);
    m
}

/// wgpu 向け透視投影（NDC z は 0..1）。
pub fn perspective(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov_y_rad * 0.5).tan();
    let mut m = Mat4([0.0; 16]);
    m.0[0] = f / aspect.max(0.01);
    m.0[5] = f;
    m.0[10] = far / (near - far);
    m.0[11] = -1.0;
    m.0[14] = (near * far) / (near - far);
    m
}

/// 視点オフセットにオービット（方位/仰角の追加回転と距離倍率）を適用する。
pub fn apply_orbit(target: Vec3, eye: Vec3, yaw: f32, pitch: f32, dist_mul: f32) -> Vec3 {
    let off = v_sub(eye, target);
    let r = v_len(off);
    if r < 1e-6 {
        return eye;
    }
    let theta = off[0].atan2(off[2]) + yaw;
    let phi = (off[1] / r).clamp(-1.0, 1.0).acos() + pitch;
    let phi = phi.clamp(0.05, std::f32::consts::PI - 0.05);
    let r = r * dist_mul;
    let sp = phi.sin();
    v_add(
        target,
        [r * sp * theta.sin(), r * phi.cos(), r * sp * theta.cos()],
    )
}
