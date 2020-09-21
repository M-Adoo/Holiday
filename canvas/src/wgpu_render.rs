use crate::{
  canvas::{CanvasRender, RenderData, Vertex},
  mem_texture::MemTexture,
  DeviceRect, DeviceSize, LogicUnit, PhysicUnit, Primitive,
};
mod img_helper;
pub(crate) use img_helper::{bgra_texture_to_png, RgbaConvert};
pub mod surface;
use std::borrow::Borrow;
use surface::{PhysicSurface, Surface, TextureSurface};
use wgpu::util::DeviceExt;
use zerocopy::AsBytes;

enum PrimaryBindings {
  GlobalUniform = 0,
  TextureAtlas = 1,
  GlyphTexture = 2,
  Sampler = 3,
}

enum SecondBindings {
  Primitive = 0,
}

pub struct WgpuRender<S: Surface = PhysicSurface> {
  device: wgpu::Device,
  queue: wgpu::Queue,
  surface: S,
  pipeline: wgpu::RenderPipeline,
  primitives_layout: wgpu::BindGroupLayout,
  uniform_layout: wgpu::BindGroupLayout,
  uniforms: wgpu::BindGroup,
  rgba_converter: Option<RgbaConvert>,
  sampler: wgpu::Sampler,
  glyph: wgpu::Texture,
  atlas: wgpu::Texture,
  resized: bool,
}

impl WgpuRender<PhysicSurface> {
  /// Create a canvas and bind to a native window, its size is `width` and
  /// `height`. If you want to create a headless window, use
  /// [`from_window`](Canvas::window).
  pub async fn wnd_render<W: raw_window_handle::HasRawWindowHandle>(
    window: &W,
    size: DeviceSize,
    glyph_texture_size: DeviceSize,
    atlas_texture_size: DeviceSize,
  ) -> Self {
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

    let w_surface = unsafe { instance.create_surface(window) };

    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
      power_preference: wgpu::PowerPreference::Default,
      compatible_surface: Some(&w_surface),
    });

    Self::new(
      size,
      glyph_texture_size,
      atlas_texture_size,
      adapter,
      move |device| PhysicSurface::new(w_surface, &device, size),
    )
    .await
  }
}

impl WgpuRender<TextureSurface> {
  /// Create a WgpuRender, if you want to bind to a window, use
  /// [`wnd_render`](WgpuRender::wnd_render).
  pub async fn headless_render(
    size: DeviceSize,
    glyph_texture_size: DeviceSize,
    atlas_texture_size: DeviceSize,
  ) -> Self {
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
      power_preference: wgpu::PowerPreference::Default,
      compatible_surface: None,
    });

    WgpuRender::new(
      size,
      glyph_texture_size,
      atlas_texture_size,
      adapter,
      |device| {
        TextureSurface::new(
          &device,
          size,
          wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
        )
      },
    )
    .await
  }

  /// PNG encoded the canvas then write by `writer`.
  pub async fn write_png<W: std::io::Write>(&mut self, writer: W) -> Result<(), &'static str> {
    self.ensure_rgba_converter();
    let rect = DeviceRect::from_size(self.surface.size());

    let Self {
      surface,
      device,
      queue,
      rgba_converter,
      ..
    } = self;
    bgra_texture_to_png(
      &surface.raw_texture,
      rect,
      device,
      queue,
      rgba_converter.as_ref().unwrap(),
      writer,
    )
    .await
  }
}

impl<S: Surface> CanvasRender for WgpuRender<S> {
  fn draw(
    &mut self,
    data: &RenderData,
    mem_glyph: &mut MemTexture<u8>,
    mem_atlas: &mut MemTexture<u32>,
  ) {
    let mut encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
      });

    let tex_infos_bind_group = self.create_primitives_bind_group(&data.primitives);
    let Self {
      device,
      glyph,
      atlas,
      surface,
      queue,
      uniform_layout,
      ..
    } = self;
    let texture_resized = mem_glyph.is_resized() || mem_atlas.is_resized();
    type TF = wgpu::TextureFormat;
    Self::sync_texture(device, glyph, mem_glyph, TF::R8Unorm, &mut encoder);
    Self::sync_texture(device, atlas, mem_atlas, TF::Bgra8UnormSrgb, &mut encoder);

    if self.resized || texture_resized {
      self.resized = false;
      let size = surface.size();
      self.uniforms = create_uniforms(
        device,
        uniform_layout,
        mem_atlas.size(),
        &coordinate_2d_to_device_matrix(size.width, size.height),
        &self.sampler,
        &atlas.create_view(&wgpu::TextureViewDescriptor::default()),
        &glyph.create_view(&wgpu::TextureViewDescriptor::default()),
      )
    }

    let view = surface.get_next_view();

    let vb = &data.vertices_buffer;
    let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Vertices buffer"),
      contents: vb.vertices.as_bytes(),
      usage: wgpu::BufferUsage::VERTEX,
    });
    let indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      contents: vb.indices.as_bytes(),
      usage: wgpu::BufferUsage::INDEX,
      label: Some("Indices buffer"),
    });

    {
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
          attachment: view.borrow(),
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
            store: true,
          },
        }],
        depth_stencil_attachment: None,
      });
      render_pass.set_pipeline(&self.pipeline);
      render_pass.set_vertex_buffer(0, vertices_buffer.slice(..));
      render_pass.set_index_buffer(indices_buffer.slice(..));
      render_pass.set_bind_group(0, &self.uniforms, &[]);
      render_pass.set_bind_group(1, &tex_infos_bind_group, &[]);

      render_pass.draw_indexed(0..vb.indices.len() as u32, 0, 0..1);
    }

    queue.submit(Some(encoder.finish()));
  }

  #[inline]
  fn resize(&mut self, size: DeviceSize) {
    self
      .surface
      .resize(&self.device, &self.queue, size.width, size.height);
    self.resized = true;
  }
}

impl<S: Surface> WgpuRender<S> {
  async fn new<C>(
    size: DeviceSize,
    glyph_texture_size: DeviceSize,
    atlas_texture_size: DeviceSize,
    adapter: impl std::future::Future<Output = Option<wgpu::Adapter>> + Send,
    surface_ctor: C,
  ) -> WgpuRender<S>
  where
    C: FnOnce(&wgpu::Device) -> S,
  {
    let (device, queue) = adapter
      .await
      .unwrap()
      .request_device(
        &wgpu::DeviceDescriptor {
          features: wgpu::Features::empty(),
          limits: Default::default(),
          shader_validation: true,
        },
        None,
      )
      .await
      .unwrap();

    let surface = surface_ctor(&device);

    let sc_desc = wgpu::SwapChainDescriptor {
      usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      width: size.width,
      height: size.height,
      present_mode: wgpu::PresentMode::Fifo,
    };

    let [uniform_layout, primitives_layout] = create_uniform_layout(&device);
    let pipeline =
      create_render_pipeline(&device, &sc_desc, &[&uniform_layout, &primitives_layout]);

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Nearest,
      min_filter: wgpu::FilterMode::Nearest,
      mipmap_filter: wgpu::FilterMode::Nearest,
      lod_min_clamp: 0.0,
      lod_max_clamp: 0.0,
      label: Some("Texture atlas sampler"),
      ..Default::default()
    });

    let glyph_texture =
      Self::create_wgpu_texture(&device, glyph_texture_size, wgpu::TextureFormat::R8Unorm);
    let texture_atlas = Self::create_wgpu_texture(
      &device,
      atlas_texture_size,
      wgpu::TextureFormat::Bgra8UnormSrgb,
    );
    let uniforms = create_uniforms(
      &device,
      &uniform_layout,
      atlas_texture_size,
      &coordinate_2d_to_device_matrix(size.width, size.height),
      &sampler,
      &texture_atlas.create_view(&wgpu::TextureViewDescriptor::default()),
      &glyph_texture.create_view(&wgpu::TextureViewDescriptor::default()),
    );

    WgpuRender {
      sampler,
      device,
      surface,
      queue,
      pipeline,
      uniform_layout,
      primitives_layout,
      uniforms,
      glyph: glyph_texture,
      atlas: texture_atlas,
      rgba_converter: None,
      resized: false,
    }
  }

  pub(crate) fn ensure_rgba_converter(&mut self) {
    if self.rgba_converter.is_none() {
      self.rgba_converter = Some(RgbaConvert::new(&self.device));
    }
  }

  fn create_primitives_bind_group(&mut self, primitives: &[Primitive]) -> wgpu::BindGroup {
    let primitives_buffer = self
      .device
      .create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Primitive Buffer"),
        contents: primitives.as_bytes(),
        usage: wgpu::BufferUsage::STORAGE,
      });
    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &self.primitives_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: SecondBindings::Primitive as u32,
        resource: wgpu::BindingResource::Buffer(primitives_buffer.slice(..)),
      }],
      label: Some("texture infos bind group"),
    })
  }

  fn sync_texture<T: Copy + Default>(
    device: &wgpu::Device,
    wgpu_tex: &mut wgpu::Texture,
    mem_tex: &mut MemTexture<T>,
    format: wgpu::TextureFormat,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    if mem_tex.is_resized() {
      *wgpu_tex = Self::create_wgpu_texture(device, mem_tex.size(), format);
    }
    if mem_tex.is_updated() {
      let DeviceSize { width, height, .. } = mem_tex.size();
      let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Memory texture submit to wgpu render."),
        contents: mem_tex.as_bytes(),
        usage: wgpu::BufferUsage::COPY_SRC,
      });

      encoder.copy_buffer_to_texture(
        wgpu::BufferCopyView {
          buffer: &buffer,
          layout: wgpu::TextureDataLayout {
            offset: 0,
            bytes_per_row: width * std::mem::size_of::<T>() as u32,
            rows_per_image: height,
          },
        },
        wgpu::TextureCopyView {
          texture: wgpu_tex,
          mip_level: 0,
          origin: wgpu::Origin3d::ZERO,
        },
        wgpu::Extent3d {
          width,
          height,
          depth: 1,
        },
      )
    }
    mem_tex.data_synced();
  }
  fn create_wgpu_texture(
    device: &wgpu::Device,
    size: DeviceSize,
    format: wgpu::TextureFormat,
  ) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
      label: Some("Create texture for memory texture"),
      size: wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth: 1,
      },
      dimension: wgpu::TextureDimension::D2,
      format,
      usage: wgpu::TextureUsage::COPY_DST
        | wgpu::TextureUsage::SAMPLED
        | wgpu::TextureUsage::COPY_SRC,
      mip_level_count: 1,
      sample_count: 1,
    })
  }
}

fn create_render_pipeline(
  device: &wgpu::Device,
  sc_desc: &wgpu::SwapChainDescriptor,
  bind_group_layouts: &[&wgpu::BindGroupLayout; 2],
) -> wgpu::RenderPipeline {
  let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("render pipeline"),
    bind_group_layouts,
    push_constant_ranges: &[],
  });

  let vs_module = device.create_shader_module(wgpu::include_spirv!(
    "./wgpu_render/shaders/geometry.vert.spv"
  ));
  let fs_module = device.create_shader_module(wgpu::include_spirv!(
    "./wgpu_render/shaders/geometry.frag.spv"
  ));

  device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("render pipeline"),
    layout: Some(&render_pipeline_layout),
    vertex_stage: wgpu::ProgrammableStageDescriptor {
      module: &vs_module,
      entry_point: "main",
    },
    fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
      module: &fs_module,
      entry_point: "main",
    }),
    rasterization_state: Some(wgpu::RasterizationStateDescriptor {
      front_face: wgpu::FrontFace::Ccw,
      cull_mode: wgpu::CullMode::None,
      depth_bias: 0,
      depth_bias_slope_scale: 0.0,
      depth_bias_clamp: 0.0,
      clamp_depth: false,
    }),
    color_states: &[wgpu::ColorStateDescriptor {
      format: sc_desc.format,
      color_blend: wgpu::BlendDescriptor {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
      },
      alpha_blend: wgpu::BlendDescriptor {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
      },
      write_mask: wgpu::ColorWrite::ALL,
    }],
    primitive_topology: wgpu::PrimitiveTopology::TriangleList,
    depth_stencil_state: None,
    vertex_state: wgpu::VertexStateDescriptor {
      index_format: wgpu::IndexFormat::Uint32,
      vertex_buffers: &[Vertex::desc()],
    },
    sample_count: 1,
    sample_mask: !0,
    alpha_to_coverage_enabled: false,
  })
}

fn create_uniform_layout(device: &wgpu::Device) -> [wgpu::BindGroupLayout; 2] {
  let stable = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &[
      wgpu::BindGroupLayoutEntry {
        binding: PrimaryBindings::GlobalUniform as u32,
        visibility: wgpu::ShaderStage::VERTEX,
        ty: wgpu::BindingType::UniformBuffer {
          dynamic: false,
          min_binding_size: None,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: PrimaryBindings::TextureAtlas as u32,
        visibility: wgpu::ShaderStage::FRAGMENT,
        ty: wgpu::BindingType::SampledTexture {
          dimension: wgpu::TextureViewDimension::D2,
          component_type: wgpu::TextureComponentType::Float,
          multisampled: false,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: PrimaryBindings::Sampler as u32,
        visibility: wgpu::ShaderStage::FRAGMENT,
        ty: wgpu::BindingType::Sampler { comparison: false },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: PrimaryBindings::GlyphTexture as u32,
        visibility: wgpu::ShaderStage::FRAGMENT,
        ty: wgpu::BindingType::SampledTexture {
          dimension: wgpu::TextureViewDimension::D2,
          component_type: wgpu::TextureComponentType::Float,
          multisampled: false,
        },
        count: None,
      },
    ],
    label: Some("uniforms stable layout"),
  });

  let dynamic = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &[wgpu::BindGroupLayoutEntry {
      binding: SecondBindings::Primitive as u32,
      visibility: wgpu::ShaderStage::VERTEX,
      ty: wgpu::BindingType::StorageBuffer {
        dynamic: false,
        readonly: true,
        min_binding_size: None,
      },
      count: None,
    }],
    label: Some("uniform layout for texture infos (changed every draw)"),
  });
  [stable, dynamic]
}

/// Convert coordinate system from canvas 2d into wgpu.
pub fn coordinate_2d_to_device_matrix(
  width: u32,
  height: u32,
) -> euclid::Transform2D<f32, LogicUnit, PhysicUnit> {
  euclid::Transform2D::new(2. / width as f32, 0., 0., -2. / height as f32, -1., 1.)
}

fn create_uniforms(
  device: &wgpu::Device,
  layout: &wgpu::BindGroupLayout,
  atlas_size: DeviceSize,
  canvas_2d_to_device_matrix: &euclid::Transform2D<f32, LogicUnit, PhysicUnit>,
  sampler: &wgpu::Sampler,
  tex_atlas: &wgpu::TextureView,
  glyph_texture: &wgpu::TextureView,
) -> wgpu::BindGroup {
  let uniform = GlobalUniform {
    texture_atlas_size: atlas_size.to_array(),
    canvas_coordinate_map: canvas_2d_to_device_matrix.to_arrays(),
  };
  let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    contents: &uniform.as_bytes(),
    usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    label: Some("uniform buffer"),
  });
  device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout,
    entries: &[
      wgpu::BindGroupEntry {
        binding: PrimaryBindings::GlobalUniform as u32,
        resource: wgpu::BindingResource::Buffer(uniform_buffer.slice(..)),
      },
      wgpu::BindGroupEntry {
        binding: PrimaryBindings::TextureAtlas as u32,
        resource: wgpu::BindingResource::TextureView(tex_atlas),
      },
      wgpu::BindGroupEntry {
        binding: PrimaryBindings::Sampler as u32,
        resource: wgpu::BindingResource::Sampler(sampler),
      },
      wgpu::BindGroupEntry {
        binding: PrimaryBindings::GlyphTexture as u32,
        resource: wgpu::BindingResource::TextureView(glyph_texture),
      },
    ],
    label: Some("uniform_bind_group"),
  })
}

#[repr(C)]
#[derive(Copy, Clone, AsBytes)]
struct GlobalUniform {
  canvas_coordinate_map: [[f32; 2]; 3],
  texture_atlas_size: [u32; 2],
}

impl Vertex {
  fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
    use std::mem::size_of;
    wgpu::VertexBufferDescriptor {
      stride: size_of::<Vertex>() as wgpu::BufferAddress,
      step_mode: wgpu::InputStepMode::Vertex,
      attributes: &[
        wgpu::VertexAttributeDescriptor {
          offset: 0,
          shader_location: 0,
          format: wgpu::VertexFormat::Float2,
        },
        wgpu::VertexAttributeDescriptor {
          offset: size_of::<[f32; 2]>() as wgpu::BufferAddress,
          shader_location: 1,
          format: wgpu::VertexFormat::Float2,
        },
        wgpu::VertexAttributeDescriptor {
          offset: (size_of::<[f32; 2]>() * 2) as wgpu::BufferAddress,
          shader_location: 2,
          format: wgpu::VertexFormat::Uint,
        },
      ],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::*;
  use futures::executor::block_on;

  fn circle_50() -> Path {
    let mut path = Path::builder();
    path.add_circle(euclid::Point2D::new(0., 0.), 50., Winding::Positive);
    path.build()
  }

  #[test]
  fn coordinate_2d_start() {
    let matrix = coordinate_2d_to_device_matrix(400, 400);

    let lt = matrix.transform_point(Point::new(0., 0.));
    assert_eq!((lt.x, lt.y), (-1., 1.));

    let rt = matrix.transform_point(Point::new(400., 0.));
    assert_eq!((rt.x, rt.y), (1., 1.));

    let lb = matrix.transform_point(Point::new(0., 400.));
    assert_eq!((lb.x, lb.y), (-1., -1.));

    let rb = matrix.transform_point(Point::new(400., 400.));
    assert_eq!((rb.x, rb.y), (1., -1.0));
  }

  #[test]
  #[ignore = "gpu need"]
  fn smoke_draw_circle() {
    let (mut canvas, mut render) = block_on(create_canvas_with_render_headless(DeviceSize::new(
      400, 400,
    )));
    let path = circle_50();

    let mut layer = canvas.new_2d_layer();
    layer.set_style(FillStyle::Color(Color::BLACK));
    layer.translate(50., 50.);
    layer.fill_path(path);
    {
      let mut frame = canvas.next_frame(&mut render);
      frame.compose_2d_layer(layer);
    }

    unit_test::assert_canvas_eq!(render, "../test_imgs/smoke_draw_circle.png");
  }

  #[test]
  #[ignore = "gpu need"]
  fn color_palette_texture() {
    let (mut canvas, mut render) = block_on(create_canvas_with_render_headless(DeviceSize::new(
      400, 400,
    )));
    let path = circle_50();
    {
      let mut layer = canvas.new_2d_layer();

      let mut fill_color_circle = |color: Color, offset_x: f32, offset_y: f32| {
        layer
          .set_style(FillStyle::Color(color))
          .translate(offset_x, offset_y)
          .fill_path(path.clone());
      };

      fill_color_circle(Color::YELLOW, 50., 50.);
      fill_color_circle(Color::RED, 100., 0.);
      fill_color_circle(Color::PINK, 100., 0.);
      fill_color_circle(Color::GREEN, 100., 0.);
      fill_color_circle(Color::BLUE, -0., 100.);

      {
        let mut frame = canvas.next_frame(&mut render);
        frame.compose_2d_layer(layer);
      }

      unit_test::assert_canvas_eq!(render, "../test_imgs/color_palette_texture.png");
    }
  }
}
