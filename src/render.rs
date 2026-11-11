//! Renderer: a moment in; a picture out. The drawing code draws into whatever target it
//! is handed, so a still and a window show the same thing. Nothing is carried from one
//! picture to the next: each is one cleared pass over buffers that never change.

use std::time::{Duration, Instant};

use wgpu::util::DeviceExt;

use crate::place::{FORM_FLOATS, LANTERN_FLOATS, Moment, Place, ROW_M, VERTEX_FLOATS, row_at};

/// Pictures are drawn in linear light into an sRGB target, for a still and a window alike.
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// Tuning: how far ahead the ground is drawn, and where the low sun stands (across, up,
/// along the route).
const DRAWN_AHEAD_M: f64 = 16_000.0;
const SUN_TOWARD: [f64; 3] = [0.3, 0.13, 1.0];
/// How far the air takes on the accent of the land around the listener, and how bright
/// that accent is at the zenith and along the horizon.
const AIR_TINT: (f32, f32, f32) = (0.55, 0.08, 0.4);
/// Samples to the pixel while drawing, so that edges and fine lines do not crawl.
const SAMPLES: u32 = 4;
const NEAR_M: f64 = 1.0;
const FAR_M: f64 = 60_000.0;
/// The most impulses drawn at once; the shader's array is this long. With them, the
/// shader is told this many floats of each moment: a matrix, nine vectors and the lights.
const MOST_LIGHTS: usize = 12;
const UNIFORM_FLOATS: usize = 16 + 4 * 9 + 4 * MOST_LIGHTS;
/// Positions reach the GPU relative to a point near the camera, on a grid this coarse,
/// so that tens of kilometres along the route cost no precision.
const ORIGIN_GRID_M: f64 = 256.0;

pub type Size = (u32, u32);

/// The most bytes the GPU takes in one buffer: the device is asked for the default limits.
pub const MOST_BUFFER_BYTES: u64 = wgpu::Limits::defaults().max_buffer_size;

/// The GPU, reached with no window and no display.
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// Which adapter drew: stills are comparable only on the same one.
    pub adapter: String,
}

impl Gpu {
    pub fn headless() -> Result<Gpu, String> {
        let instance = wgpu::InstanceDescriptor::new_without_display_handle();
        Ok(Gpu::reach(instance, None)?.0)
    }

    /// Reaches the GPU and, where there is a window, the surface that shows pictures in it.
    fn reach(
        mut instance: wgpu::InstanceDescriptor,
        window: Option<wgpu::SurfaceTarget<'static>>,
    ) -> Result<(Gpu, Option<wgpu::Surface<'static>>, wgpu::Adapter), String> {
        instance.backends = wgpu::Backends::VULKAN;
        let instance = wgpu::Instance::new(instance);
        let surface = window
            .map(|window| instance.create_surface(window))
            .transpose()
            .map_err(|error| format!("the window cannot be drawn in ({error})"))?;
        let options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: surface.as_ref(),
            apply_limit_buckets: false,
        };
        let adapter = pollster::block_on(instance.request_adapter(&options))
            .map_err(|error| format!("no graphics adapter can draw the place ({error})"))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|error| format!("the graphics adapter cannot be used ({error})"))?;
        let info = adapter.get_info();
        let adapter_said = format!(
            "{}, {:?}, driver {}",
            info.name, info.backend, info.driver_info
        );
        let gpu = Gpu {
            device,
            queue,
            adapter: adapter_said,
        };
        Ok((gpu, surface, adapter))
    }

    /// Draws one moment into a target and waits for it; returns how long the drawing took.
    pub fn draw(
        &self,
        renderer: &Renderer,
        target: &wgpu::TextureView,
        moment: &Moment,
    ) -> Result<Duration, String> {
        let started = Instant::now();
        let mut encoder = self.device.create_command_encoder(&Default::default());
        renderer.draw(&self.queue, &mut encoder, target, moment);
        let submitted = self.queue.submit([encoder.finish()]);
        self.wait(Some(submitted))?;
        Ok(started.elapsed())
    }

    fn wait(&self, submitted: Option<wgpu::SubmissionIndex>) -> Result<(), String> {
        let wait = wgpu::PollType::Wait {
            submission_index: submitted,
            timeout: None,
        };
        self.device.poll(wait).map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn bytes(floats: &[f32]) -> Vec<u8> {
    floats
        .iter()
        .flat_map(|float| float.to_ne_bytes())
        .collect()
}

/// A texture to draw into. One with a single sample to the pixel can be read back, and
/// copied to a window.
fn texture(
    device: &wgpu::Device,
    size: Size,
    format: wgpu::TextureFormat,
    sample_count: u32,
) -> wgpu::Texture {
    let read = if sample_count == 1 {
        wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING
    } else {
        wgpu::TextureUsages::empty()
    };
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | read,
        view_formats: &[],
    })
}

/// The one piece of drawing code: the whole place on the GPU, drawn a moment at a time.
pub struct Renderer {
    sky: wgpu::RenderPipeline,
    ground: wgpu::RenderPipeline,
    forms: wgpu::RenderPipeline,
    lanterns: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    /// The boxes of the forms and the lanterns, and how many there are of each.
    form_instances: wgpu::Buffer,
    lantern_instances: wgpu::Buffer,
    form_count: u32,
    /// Where the picture is drawn, several samples to the pixel, before it is resolved
    /// into the target; and the depth that goes with it.
    drawn: wgpu::TextureView,
    depth: wgpu::TextureView,
    lantern_count: u32,
    /// Rows of ground, and the indices that draw one of them.
    rows: u32,
    row_indices: u32,
    /// The part of every moment's uniforms that the place fixes: sky colours and the sun.
    place: [f32; 16],
    haze_per_m: f32,
    aspect: f64,
}

impl Renderer {
    /// Puts a place on the GPU, to be drawn into targets of `FORMAT` and of one size.
    pub fn new(device: &wgpu::Device, size: Size, place: &Place) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("place.wgsl"));
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (4 * UNIFORM_FLOATS) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        // The sky is behind everything; the ground and the forms hide what is behind them;
        // lanterns add their light on top of whatever does not hide them.
        let pipeline = |name: &str,
                        buffers: &[Option<wgpu::VertexBufferLayout>],
                        blend: Option<wgpu::BlendState>,
                        hides: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(name),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(&format!("{name}_vertex")),
                    compilation_options: Default::default(),
                    buffers,
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: Some(hides),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: SAMPLES,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(&format!("{name}_fragment")),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: FORMAT,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let adding = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let vertex = wgpu::VertexBufferLayout {
            array_stride: (4 * VERTEX_FLOATS) as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32, 4 => Float32x4],
        };
        let lantern = wgpu::VertexBufferLayout {
            array_stride: (4 * LANTERN_FLOATS) as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32],
        };
        let form = wgpu::VertexBufferLayout {
            array_stride: (4 * FORM_FLOATS) as u64,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
            ..lantern.clone()
        };

        let ground = place.ground();
        let (rows, columns) = (ground.rows as u32, ground.columns as u32);
        let indices: Vec<u32> = (0..rows - 1)
            .flat_map(|row| (0..columns - 1).map(move |column| row * columns + column))
            .flat_map(|corner| {
                let (beside, beyond) = (corner + 1, corner + columns);
                [corner, beyond, beside, beside, beyond, beyond + 1]
            })
            .collect();
        let (forms, lanterns) = (place.forms(), place.lanterns());
        let buffer = |contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents,
                usage,
            })
        };
        // A buffer may not be empty, so a place with no forms or lanterns still holds room for one.
        let instances = |floats: &[f32]| {
            let room = [floats, &[0.0; FORM_FLOATS]].concat();
            buffer(&bytes(&room), wgpu::BufferUsages::VERTEX)
        };
        let sky = &place.sky;
        let length = SUN_TOWARD
            .iter()
            .map(|part| part * part)
            .sum::<f64>()
            .sqrt();
        let sun = SUN_TOWARD.map(|part| (part / length) as f32);
        let mut fixed = [0.0; 16];
        for (slot, colour) in [sky.zenith, sky.horizon, sky.sun, sun].iter().enumerate() {
            fixed[slot * 4..slot * 4 + 3].copy_from_slice(colour);
        }
        Renderer {
            sky: pipeline("sky", &[], None, false),
            ground: pipeline("ground", &[Some(vertex)], None, true),
            forms: pipeline("form", &[Some(form)], None, true),
            lanterns: pipeline(
                "lantern",
                &[Some(lantern)],
                Some(wgpu::BlendState {
                    color: adding,
                    alpha: adding,
                }),
                false,
            ),
            uniforms,
            bind_group,
            vertices: buffer(&bytes(&ground.vertices), wgpu::BufferUsages::VERTEX),
            indices: buffer(
                &indices
                    .iter()
                    .flat_map(|index| index.to_ne_bytes())
                    .collect::<Vec<u8>>(),
                wgpu::BufferUsages::INDEX,
            ),
            form_instances: instances(&forms),
            lantern_instances: instances(&lanterns),
            form_count: (forms.len() / FORM_FLOATS) as u32,
            drawn: texture(device, size, FORMAT, SAMPLES).create_view(&Default::default()),
            depth: texture(device, size, DEPTH, SAMPLES).create_view(&Default::default()),
            lantern_count: (lanterns.len() / LANTERN_FLOATS) as u32,
            rows,
            row_indices: (columns - 1) * 6,
            place: fixed,
            haze_per_m: sky.haze_per_m,
            aspect: f64::from(size.0) / f64::from(size.1),
        }
    }

    /// Everything the shader is told about a moment: the camera, the light, the impulses
    /// in play, and the place's fixed colours.
    fn uniforms(&self, moment: &Moment) -> Vec<f32> {
        let camera = &moment.camera;
        let [right, up, forward] = camera.axes();

        let origin = [0, 2].map(|i| (camera.position_m[i] / ORIGIN_GRID_M).round() * ORIGIN_GRID_M);
        let eye = [
            camera.position_m[0] - origin[0],
            camera.position_m[1],
            camera.position_m[2] - origin[1],
        ];
        let tall = (camera.view_deg / 2.0).to_radians().tan();
        let half = (tall * self.aspect, tall);
        let (deep, shift) = (FAR_M / (FAR_M - NEAR_M), -NEAR_M * FAR_M / (FAR_M - NEAR_M));
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let column = |i: usize| {
            [
                right[i] / half.0,
                up[i] / half.1,
                forward[i] * deep,
                forward[i],
            ]
        };
        let last = [
            -dot(right, eye) / half.0,
            -dot(up, eye) / half.1,
            -dot(forward, eye) * deep + shift,
            -dot(forward, eye),
        ];

        let shown = &moment.impulses[moment.impulses.len().saturating_sub(MOST_LIGHTS)..];
        let mut uniforms: Vec<f64> = [column(0), column(1), column(2), last].concat();
        uniforms.extend([origin[0], 0.0, origin[1], moment.light]);
        let haze = f64::from(self.haze_per_m * moment.air);
        uniforms.extend([eye[0], eye[1], eye[2], haze]);
        uniforms.extend(right.map(|part| part * half.0));
        uniforms.push(shown.len() as f64);
        uniforms.extend(up.map(|part| part * half.1));
        uniforms.push(1.0 / moment.nearer);
        uniforms.extend(forward);
        uniforms.push(f64::from(moment.air));
        let mut uniforms: Vec<f32> = uniforms.iter().map(|value| *value as f32).collect();
        // The air takes on the accent of the land around the listener.
        let mut colours = self.place;
        for (channel, accent) in moment.accent.iter().enumerate() {
            colours[channel] += (AIR_TINT.1 * accent - colours[channel]) * AIR_TINT.0;
            colours[4 + channel] += (AIR_TINT.2 * accent - colours[4 + channel]) * AIR_TINT.0;
        }
        uniforms.extend(colours);
        for impulse in shown {
            // The side lit: -1 the left, 1 the right, and 0, which is both.
            let side: f64 = impulse.side.signs().iter().sum();
            uniforms.extend([side, 0.0, impulse.reach_m, impulse.level].map(|v| v as f32));
        }
        uniforms.resize(UNIFORM_FLOATS, 0.0);
        uniforms
    }

    /// Draws one moment into a target of the format and size this renderer was made for.
    pub fn draw(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        moment: &Moment,
    ) {
        queue.write_buffer(&self.uniforms, 0, &bytes(&self.uniforms(moment)));
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.drawn,
                depth_slice: None,
                resolve_target: Some(target),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_pipeline(&self.sky);
        pass.draw(0..3, 0..1);

        // Only the ground from just behind the camera to the far haze is drawn.
        let row = |route_m: f64| (row_at(route_m) as u32).min(self.rows - 1);
        let first = row(moment.route_position_m - 2.0 * ROW_M);
        let end = row(moment.route_position_m + DRAWN_AHEAD_M);
        pass.set_pipeline(&self.ground);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(first * self.row_indices..end * self.row_indices, 0, 0..1);

        // A form is a box of six faces, a lantern one face that is turned to the camera.
        let instanced = [
            (&self.forms, &self.form_instances, 36, self.form_count),
            (
                &self.lanterns,
                &self.lantern_instances,
                6,
                self.lantern_count,
            ),
        ];
        for (pipeline, instances, corners, count) in instanced {
            if count > 0 {
                pass.set_pipeline(pipeline);
                pass.set_vertex_buffer(0, instances.slice(..));
                pass.draw(0..corners, 0..count);
            }
        }
    }
}

/// The window a journey is shown in. A picture is drawn exactly as a still is and then
/// copied to the window, so the window shows what the still shows, whatever its own size.
pub struct Screen {
    surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    copy: wgpu::util::TextureBlitter,
}

impl Screen {
    /// Reaches the GPU through a window of the given size.
    pub fn open(
        instance: wgpu::InstanceDescriptor,
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        size: Size,
    ) -> Result<(Gpu, Screen), String> {
        let (gpu, surface, adapter) = Gpu::reach(instance, Some(window.into()))?;
        let surface = surface.ok_or("the window has nothing to draw in")?;
        let offered = surface.get_capabilities(&adapter);
        let srgb = offered.formats.iter().find(|format| format.is_srgb());
        let opaque = wgpu::CompositeAlphaMode::Opaque;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *srgb.ok_or("the display takes no sRGB picture")?,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.0.max(1),
            height: size.1.max(1),
            // One picture for each refresh of the display, in order, never torn.
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: if offered.alpha_modes.contains(&opaque) {
                opaque
            } else {
                wgpu::CompositeAlphaMode::Auto
            },
            view_formats: vec![],
        };
        surface.configure(&gpu.device, &config);
        let copy = wgpu::util::TextureBlitterBuilder::new(&gpu.device, config.format)
            .sample_type(wgpu::FilterMode::Linear)
            .build();
        Ok((
            gpu,
            Screen {
                surface,
                config,
                copy,
            },
        ))
    }

    /// The window has a new size, in pixels.
    pub fn resize(&mut self, gpu: &Gpu, size: Size) {
        if size.0 > 0 && size.1 > 0 && size != (self.config.width, self.config.height) {
            (self.config.width, self.config.height) = size;
            self.surface.configure(&gpu.device, &self.config);
        }
    }

    /// Room for the next picture, waiting for it if the display is still busy with the
    /// pictures before. None when the display has no room this time.
    pub fn room(&self, gpu: &Gpu) -> Result<Option<wgpu::SurfaceTexture>, String> {
        use wgpu::CurrentSurfaceTexture as Room;
        match self.surface.get_current_texture() {
            Room::Success(room) => Ok(Some(room)),
            Room::Timeout | Room::Occluded => Ok(None),
            // The window changed under the surface. Stale room must go before the
            // surface is set up again.
            Room::Suboptimal(stale) => {
                drop(stale);
                self.surface.configure(&gpu.device, &self.config);
                Ok(None)
            }
            Room::Outdated => {
                self.surface.configure(&gpu.device, &self.config);
                Ok(None)
            }
            Room::Lost | Room::Validation => Err("the display was lost".to_string()),
        }
    }

    /// Draws a moment into `picture` as a still is drawn, and shows it in the window.
    /// `about_to_show` is called just before the picture is handed to the display.
    pub fn show(
        &self,
        gpu: &Gpu,
        renderer: &Renderer,
        picture: &Still,
        moment: &Moment,
        room: wgpu::SurfaceTexture,
        about_to_show: impl FnOnce(),
    ) {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        renderer.draw(&gpu.queue, &mut encoder, &picture.view, moment);
        let shown = room.texture.create_view(&Default::default());
        self.copy
            .copy(&gpu.device, &mut encoder, &picture.view, &shown);
        gpu.queue.submit([encoder.finish()]);
        about_to_show();
        gpu.queue.present(room);
    }
}

/// A picture made with no display: a texture to draw into and the means to read it back.
pub struct Still {
    texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    readback: wgpu::Buffer,
    pub size: Size,
    /// Bytes from one row to the next in the readback, which pads rows.
    stride: u32,
}

impl Still {
    pub fn new(device: &wgpu::Device, size: Size) -> Still {
        let texture = texture(device, size, FORMAT, 1);
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let stride = (size.0 * 4).div_ceil(align) * align;
        Still {
            view: texture.create_view(&Default::default()),
            texture,
            readback: device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: u64::from(stride) * u64::from(size.1),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            size,
            stride,
        }
    }

    /// Brings the picture back from the GPU and hands over its rows, top row first: sRGB
    /// bytes, four to a pixel, each row `stride` bytes after the one before.
    pub fn read<R>(&self, gpu: &Gpu, with: impl FnOnce(&[u8], usize) -> R) -> Result<R, String> {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.stride),
                    rows_per_image: Some(self.size.1),
                },
            },
            self.texture.size(),
        );
        gpu.queue.submit([encoder.finish()]);
        let (sender, receiver) = std::sync::mpsc::channel();
        self.readback
            .map_async(wgpu::MapMode::Read, .., move |mapped| {
                let _ = sender.send(mapped);
            });
        gpu.wait(None)?;
        receiver
            .recv()
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;
        let result = {
            let rows = self
                .readback
                .get_mapped_range(..)
                .map_err(|error| error.to_string())?;
            with(&rows, self.stride as usize)
        };
        self.readback.unmap();
        Ok(result)
    }

    /// The picture as a PNG file.
    pub fn png(&self, gpu: &Gpu) -> Result<Vec<u8>, String> {
        let (width, height) = (self.size.0 as usize, self.size.1 as usize);
        let rgba = self.read(gpu, |rows, stride| {
            let row = |y: usize| &rows[y * stride..y * stride + width * 4];
            (0..height).flat_map(row).copied().collect::<Vec<u8>>()
        })?;
        Ok(png_file(self.size, png::ColorType::Rgba, &rgba))
    }
}

/// A picture as a PNG file, from its pixels as sRGB bytes, row by row from the top.
pub fn png_file(size: Size, colour: png::ColorType, pixels: &[u8]) -> Vec<u8> {
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, size.0, size.1);
    encoder.set_color(colour);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    encoder.set_compression(png::Compression::Fast);
    let written = "a PNG of pixels that match its size can be written to memory";
    let mut writer = encoder.write_header().expect(written);
    writer.write_image_data(pixels).expect(written);
    writer.finish().expect(written);
    file
}
