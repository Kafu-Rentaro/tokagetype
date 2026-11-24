//! view3d の実描画: egui_wgpu::Callback で GLB をオフスクリーンへ描き、
//! そのテクスチャをノード矩形へ貼る自前 wgpu レンダラ。
//!
//! - シーン描画は prepare で行い、paint でテクスチャ付き三角形を描く（1フレーム遅延なし）。
//! - マテリアルは baseColorFactor × 頂点色 × baseColor テクスチャ + emissive、
//!   方向光源1灯 + 環境光の簡易ライティング。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use wgpu::util::DeviceExt;

use crate::glb::{parse_glb, CpuModel, CpuTexture, Vertex};
use crate::math3d::Mat4;

/// CPU テクスチャを GPU へアップロードし、ビューを返す（ビューが本体への参照を保持する）。
fn upload_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    tex: &CpuTexture,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("tokage-glb-texture"),
        size: wgpu::Extent3d {
            width: tex.width,
            height: tex.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SCENE_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &tex.rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * tex.width),
            rows_per_image: Some(tex.height),
        },
        wgpu::Extent3d {
            width: tex.width,
            height: tex.height,
            depth_or_array_layers: 1,
        },
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// アセットキー（asset_id + 内容ハッシュ）→ 読込エラー。アプリ側の表示切替に使う。
pub type RenderErrors = Arc<Mutex<HashMap<String, String>>>;

pub struct View3dCallback {
    /// ノード単位の描画先キー（ノード ID）
    pub target_key: String,
    /// モデルキャッシュキー（asset_id + 内容ハッシュ）
    pub model_key: String,
    pub bytes: Arc<Vec<u8>>,
    pub mvp: Mat4,
    /// 描画先の物理ピクセルサイズ
    pub size_px: [u32; 2],
    pub surface_format: wgpu::TextureFormat,
    pub errors: RenderErrors,
}

struct GpuPrimitive {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    bind_group: wgpu::BindGroup,
}

struct GpuModel {
    primitives: Vec<GpuPrimitive>,
}

struct Target {
    w: u32,
    h: u32,
    _color_tex: wgpu::Texture,
    color_view: wgpu::TextureView,
    _depth_tex: wgpu::Texture,
    depth_view: wgpu::TextureView,
    quad_bind_group: wgpu::BindGroup,
    frame_bind_group: wgpu::BindGroup,
    frame_uniform: wgpu::Buffer,
}

struct Resources {
    scene_pipeline: wgpu::RenderPipeline,
    quad_pipeline: wgpu::RenderPipeline,
    scene_layout: wgpu::BindGroupLayout,
    frame_layout: wgpu::BindGroupLayout,
    quad_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    dummy_tex: wgpu::Texture,
    models: HashMap<String, Result<GpuModel, String>>,
    targets: HashMap<String, Target>,
}

const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

const SHADER: &str = r#"
struct FrameU { mvp: mat4x4<f32> };
@group(1) @binding(0) var<uniform> frame: FrameU;

struct PrimU {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    has_texture: u32,
    has_emissive: u32,
    _pad1: u32, _pad2: u32,
};
@group(0) @binding(0) var<uniform> prim: PrimU;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
@group(0) @binding(3) var etex: texture_2d<f32>;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};
@vertex fn vs_scene(in: VsIn) -> VsOut {
    var o: VsOut;
    o.clip = frame.mvp * vec4<f32>(in.pos, 1.0);
    o.normal = in.normal;
    o.uv = in.uv;
    o.color = in.color;
    return o;
}
@fragment fn fs_scene(in: VsOut) -> @location(0) vec4<f32> {
    var base = prim.base_color * in.color;
    if (prim.has_texture == 1u) {
        base = base * textureSample(tex, samp, in.uv);
    }
    let n = normalize(in.normal);
    let l = normalize(vec3<f32>(0.45, 0.8, 0.55));
    let diff = max(dot(n, l), 0.0);
    var emis = prim.emissive.rgb;
    if (prim.has_emissive == 1u) {
        emis = emis * textureSample(etex, samp, in.uv).rgb;
    }
    let rgb = base.rgb * (0.38 + 0.62 * diff) + emis;
    return vec4<f32>(rgb, base.a);
}

@group(0) @binding(0) var qtex: texture_2d<f32>;
@group(0) @binding(1) var qsamp: sampler;
struct QOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
@vertex fn vs_quad(@builtin(vertex_index) idx: u32) -> QOut {
    var pos = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    let p = pos[idx];
    var o: QOut;
    o.clip = vec4<f32>(p, 0.0, 1.0);
    o.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return o;
}
@fragment fn fs_quad(in: QOut) -> @location(0) vec4<f32> {
    return textureSample(qtex, qsamp, in.uv);
}
"#;

impl Resources {
    fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tokage-3d"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("tokage-3d-sampler"),
            // glTF のデフォルトは REPEAT。UV が 0..1 を超えるモデル（DamagedHelmet 等）で必須
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        // テクスチャ無しプリミティブ用の 1x1 白テクスチャ
        let dummy = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("tokage-dummy"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SCENE_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let dummy_tex = dummy;

        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tokage-prim-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tokage-frame-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let scene_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tokage-scene-layout"),
            bind_group_layouts: &[&scene_layout, &frame_layout],
            push_constant_ranges: &[],
        });
        let vertex_attrs = wgpu::vertex_attr_array![
            0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x4
        ];
        let scene_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tokage-scene-pipeline"),
            layout: Some(&scene_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: "vs_scene",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &vertex_attrs,
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: "fs_scene",
                targets: &[Some(wgpu::ColorTargetState {
                    format: SCENE_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let quad_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tokage-quad-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let quad_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tokage-quad-layout-pl"),
            bind_group_layouts: &[&quad_layout],
            push_constant_ranges: &[],
        });
        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tokage-quad-pipeline"),
            layout: Some(&quad_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: "vs_quad",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: "fs_quad",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        Resources {
            scene_pipeline,
            quad_pipeline,
            scene_layout,
            frame_layout,
            quad_layout,
            sampler,
            dummy_tex,
            models: HashMap::new(),
            targets: HashMap::new(),
        }
    }

    fn ensure_target(&mut self, device: &wgpu::Device, key: &str, w: u32, h: u32) {
        let w = w.max(1);
        let h = h.max(1);
        if let Some(t) = self.targets.get(key) {
            if t.w == w && t.h == h {
                return;
            }
        }
        let color_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("tokage-3d-target"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SCENE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("tokage-3d-depth"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let quad_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tokage-quad-bg"),
            layout: &self.quad_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let frame_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tokage-frame-uniform"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tokage-frame-bg"),
            layout: &self.frame_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame_uniform.as_entire_binding(),
            }],
        });
        self.targets.insert(
            key.to_string(),
            Target {
                w,
                h,
                _color_tex: color_tex,
                color_view,
                _depth_tex: depth_tex,
                depth_view,
                quad_bind_group,
                frame_bind_group,
                frame_uniform,
            },
        );
    }

    fn ensure_model(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: &str,
        bytes: &[u8],
        errors: &RenderErrors,
    ) {
        if self.models.contains_key(key) {
            return;
        }
        let result = parse_glb(bytes).map(|cpu| self.upload(device, queue, &cpu));
        if let Err(e) = &result {
            if let Ok(mut map) = errors.lock() {
                map.insert(key.to_string(), e.clone());
            }
        }
        self.models.insert(key.to_string(), result);
    }

    fn upload(&self, device: &wgpu::Device, queue: &wgpu::Queue, model: &CpuModel) -> GpuModel {
        let mut primitives = Vec::new();
        for prim in &model.primitives {
            if prim.vertices.is_empty() || prim.indices.is_empty() {
                continue;
            }
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tokage-vb"),
                contents: bytemuck::cast_slice(&prim.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tokage-ib"),
                contents: bytemuck::cast_slice(&prim.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            // プリミティブ uniform (48 bytes): base_color vec4 / emissive vec4 /
            // has_texture u32（offset 32）/ padding
            let mut uniform = [0f32; 12];
            uniform[0..4].copy_from_slice(&prim.base_color);
            uniform[4] = prim.emissive[0];
            uniform[5] = prim.emissive[1];
            uniform[6] = prim.emissive[2];
            let has_tex_u32: u32 = if prim.texture.is_some() { 1 } else { 0 };
            let has_emis_u32: u32 = if prim.emissive_texture.is_some() { 1 } else { 0 };
            let mut uniform_bytes = Vec::with_capacity(48);
            uniform_bytes.extend_from_slice(bytemuck::cast_slice(&uniform[0..8]));
            uniform_bytes.extend_from_slice(&has_tex_u32.to_le_bytes());
            uniform_bytes.extend_from_slice(&has_emis_u32.to_le_bytes());
            uniform_bytes.extend_from_slice(&[0u8; 8]);
            let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tokage-prim-uniform"),
                contents: &uniform_bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });

            let tex_view = match &prim.texture {
                Some(tex) => upload_texture(device, queue, tex),
                None => self
                    .dummy_tex
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            };
            let etex_view = match &prim.emissive_texture {
                Some(tex) => upload_texture(device, queue, tex),
                None => self
                    .dummy_tex
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            };

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tokage-prim-bg"),
                layout: &self.scene_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&tex_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&etex_view),
                    },
                ],
            });
            primitives.push(GpuPrimitive {
                vertex_buffer,
                index_buffer,
                index_count: prim.indices.len() as u32,
                bind_group,
            });
        }
        GpuModel { primitives }
    }
}

impl egui_wgpu::CallbackTrait for View3dCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if !callback_resources.contains::<Resources>() {
            callback_resources.insert(Resources::new(device, self.surface_format));
        }
        // 初回 insert 直後は get_mut が必要なので取り直す
        let resources: &mut Resources = callback_resources.get_mut().unwrap();
        resources.ensure_target(device, &self.target_key, self.size_px[0], self.size_px[1]);
        resources.ensure_model(device, queue, &self.model_key, &self.bytes, &self.errors);

        let target = resources.targets.get(&self.target_key).unwrap();
        queue.write_buffer(
            &target.frame_uniform,
            0,
            bytemuck::cast_slice(&self.mvp.0),
        );
        let model_ok = matches!(
            resources.models.get(&self.model_key),
            Some(Ok(_))
        );
        {
            let mut pass = egui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("tokage-3d-scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.867,
                            g: 0.882,
                            b: 0.902,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if model_ok {
                let model = match resources.models.get(&self.model_key) {
                    Some(Ok(m)) => m,
                    _ => unreachable!(),
                };
                pass.set_pipeline(&resources.scene_pipeline);
                pass.set_bind_group(1, &target.frame_bind_group, &[]);
                for prim in &model.primitives {
                    pass.set_bind_group(0, &prim.bind_group, &[]);
                    pass.set_vertex_buffer(0, prim.vertex_buffer.slice(..));
                    pass.set_index_buffer(prim.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..prim.index_count, 0, 0..1);
                }
            }
        }
        Vec::new()
    }

    fn paint(
        &self,
        info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let Some(resources) = callback_resources.get::<Resources>() else {
            return;
        };
        let Some(target) = resources.targets.get(&self.target_key) else {
            return;
        };
        // viewport と scissor は egui_wgpu がコールバック矩形に合わせて設定済み。
        let _ = info;
        render_pass.set_pipeline(&resources.quad_pipeline);
        render_pass.set_bind_group(0, &target.quad_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
