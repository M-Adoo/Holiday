use crate::{tessellator::Tessellator, GlRender, GpuBackend, TriangleLists, Vertex};
use futures::executor::block_on;
use painter::DeviceSize;
use std::{error::Error, iter};
use text::shaper::TextShaper;
mod color_pass;
pub mod surface;

use surface::{Surface, TextureSurface, WindowSurface};
use wgpu::util::DeviceExt;

use zerocopy::AsBytes;
mod img_pass;
use self::{color_pass::ColorPass, img_pass::ImagePass};

const TEXTURE_INIT_SIZE: (u16, u16) = (1024, 1024);
const TEXTURE_MAX_SIZE: (u16, u16) = (4096, 4096);

/// create wgpu backend with window
pub async fn wgpu_backend_with_wnd<W: raw_window_handle::HasRawWindowHandle>(
  window: &W,
  size: DeviceSize,
  tex_init_size: Option<(u16, u16)>,
  tex_max_size: Option<(u16, u16)>,
  shaper: TextShaper,
) -> GpuBackend<WgpuGl> {
  let init_size = tex_init_size.unwrap_or(TEXTURE_INIT_SIZE);
  let max_size = tex_max_size.unwrap_or(TEXTURE_MAX_SIZE);
  let tessellator = Tessellator::new(init_size, max_size, shaper);
  let gl = WgpuGl::from_wnd(window, size, AntiAliasing::Msaa4X).await;

  GpuBackend { tessellator, gl }
}

/// create wgpu backend windowless
pub async fn wgpu_backend_headless(
  size: DeviceSize,
  tex_init_size: Option<(u16, u16)>,
  tex_max_size: Option<(u16, u16)>,
  shaper: TextShaper,
) -> GpuBackend<WgpuGl<surface::TextureSurface>> {
  let init_size = tex_init_size.unwrap_or(TEXTURE_INIT_SIZE);
  let max_size = tex_max_size.unwrap_or(TEXTURE_MAX_SIZE);
  let tessellator = Tessellator::new(init_size, max_size, shaper);
  let gl = WgpuGl::headless(size).await;
  GpuBackend { tessellator, gl }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AntiAliasing {
  None = 1,
  Msaa2X = 2,
  Msaa4X = 4,
  Msaa8X = 8,
  Msaa16X = 16,
}

pub struct WgpuGl<S: Surface = WindowSurface> {
  device: wgpu::Device,
  queue: wgpu::Queue,
  surface: S,
  color_pass: ColorPass,
  img_pass: ImagePass,
  coordinate_matrix: wgpu::Buffer,
  primitives_layout: wgpu::BindGroupLayout,
  vertex_buffers: Option<VertexBuffers>,
  anti_aliasing: AntiAliasing,
  multisample_framebuffer: Option<wgpu::TextureView>,
  /// if the frame already draw something.
  empty_frame: bool,
}
struct VertexBuffers {
  vertices: wgpu::Buffer,
  vertex_size: usize,
  indices: wgpu::Buffer,
  index_size: usize,
}

impl WgpuGl<WindowSurface> {
  /// Create a canvas and bind to a native window, its size is `width` and
  /// `height`. If you want to create a headless window, use
  /// [`headless_render`](WgpuRender::headless_render).
  pub async fn from_wnd<W: raw_window_handle::HasRawWindowHandle>(
    window: &W,
    size: DeviceSize,
    anti_aliasing: AntiAliasing,
  ) -> Self {
    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

    let w_surface = unsafe { instance.create_surface(window) };

    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&w_surface),
        force_fallback_adapter: false,
      })
      .await
      .unwrap();

    Self::new(
      size,
      &adapter,
      |device| WindowSurface::new(w_surface, device, size),
      anti_aliasing,
    )
    .await
  }
}

impl WgpuGl<TextureSurface> {
  /// Create a headless wgpu render, if you want to bind to a window, use
  /// [`wnd_render`](WgpuRender::wnd_render).
  pub async fn headless(size: DeviceSize) -> Self {
    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
      })
      .await
      .unwrap();

    WgpuGl::new(
      size,
      &adapter,
      |device| {
        TextureSurface::new(
          device,
          size,
          wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        )
      },
      AntiAliasing::None,
    )
    .await
  }
}

impl<S: Surface> GlRender for WgpuGl<S> {
  fn begin_frame(&mut self) { self.empty_frame = true; }

  fn add_texture(&mut self, texture: crate::Texture) {
    self
      .img_pass
      .add_texture(texture, &self.device, &self.queue)
  }

  fn draw_triangles(&mut self, data: TriangleLists) {
    self.write_vertex_buffer(data.vertices, data.indices);
    let vertex_buffers = self.vertex_buffers.as_ref().unwrap();

    let mut encoder = self.create_command_encoder();
    let prim_bind_group = self.create_primitives_bind_group(data.primitives);

    let Self {
      device,
      coordinate_matrix,
      color_pass,
      img_pass,
      ..
    } = self;

    let uniforms = data
      .commands
      .iter()
      .filter_map(|cmd| match cmd {
        crate::DrawTriangles::Texture { texture_id, .. } => {
          let uniform = img_pass.create_texture_uniform(device, *texture_id, coordinate_matrix);
          Some((texture_id, uniform))
        }
        _ => None,
      })
      .collect::<std::collections::HashMap<_, _>>();

    let view = self
      .surface
      .current_texture()
      .create_view(&wgpu::TextureViewDescriptor::default());

    {
      let (view, resolve_target, store) = self.multisample_framebuffer.as_ref().map_or_else(
        || (&view, None, true),
        |multi_sample| (multi_sample, Some(&view), false),
      );
      let load = if self.empty_frame {
        wgpu::LoadOp::Clear(wgpu::Color::WHITE)
      } else {
        wgpu::LoadOp::Load
      };
      let ops = wgpu::Operations { load, store };
      let rpass_color_attachment = wgpu::RenderPassColorAttachment { view, resolve_target, ops };

      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Triangles render pass"),
        color_attachments: &[rpass_color_attachment],
        depth_stencil_attachment: None,
      });

      render_pass.set_vertex_buffer(0, vertex_buffers.vertices.slice(..));
      render_pass.set_index_buffer(vertex_buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
      render_pass.set_bind_group(1, &prim_bind_group, &[]);
      data.commands.iter().for_each(|cmd| match cmd {
        crate::DrawTriangles::Color(rg) => {
          render_pass.set_pipeline(&color_pass.pipeline);
          render_pass.set_bind_group(0, &color_pass.uniform, &[]);
          render_pass.draw_indexed(rg.clone(), 0, 0..1);
        }
        crate::DrawTriangles::Texture { rg, texture_id } => {
          render_pass.set_pipeline(&img_pass.pipeline);
          render_pass.set_bind_group(0, uniforms.get(texture_id).unwrap(), &[]);
          render_pass.draw_indexed(rg.clone(), 0, 0..1);
        }
      });
    }
    self.empty_frame = false;

    self.queue.submit(iter::once(encoder.finish()));
  }

  fn end_frame<'a>(&mut self, cancel: bool) {
    if !cancel {
      self.surface.present();
    }
    self.img_pass.end_frame();
  }

  fn resize(&mut self, size: DeviceSize) {
    self.surface.resize(&self.device, &self.queue, size);
    self.coordinate_matrix = coordinate_matrix_buffer_2d(&self.device, size.width, size.height);
    self
      .color_pass
      .resize(&self.coordinate_matrix, &self.device)
  }

  fn capture(&self, capture: painter::CaptureCallback) -> Result<(), Box<dyn Error>> {
    let mut encoder = self.create_command_encoder();
    let buffer = self.surface.copy_as_rgba_buffer(&self.device, &mut encoder);
    self.queue.submit(iter::once(encoder.finish()));

    let buffer_slice = buffer.slice(..);
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

    // Poll the device in a blocking manner so that our future resolves.
    self.device.poll(wgpu::Maintain::Wait);
    block_on(buffer_future)?;

    let size = self.surface.view_size();
    let slice = buffer_slice.get_mapped_range();
    let buffer_bytes_per_row = slice.len() as u32 / size.height;
    let img_bytes_pre_row = (size.width * 4) as usize;
    let rows = (0..size.height).map(|i| {
      let offset = (i * buffer_bytes_per_row) as usize;
      &slice.as_ref()[offset..offset + img_bytes_pre_row]
    });

    capture(size, Box::new(rows));
    Ok(())
  }
}

impl<S: Surface> WgpuGl<S> {
  async fn new<C>(
    size: DeviceSize,
    adapter: &wgpu::Adapter,
    surface_ctor: C,
    anti_aliasing: AntiAliasing,
  ) -> WgpuGl<S>
  where
    C: FnOnce(&wgpu::Device) -> S,
  {
    let (device, queue) = adapter
      .request_device(
        &wgpu::DeviceDescriptor {
          label: Some("Request device"),
          features: wgpu::Features::empty(),
          limits: Default::default(),
        },
        None,
      )
      .await
      .unwrap();

    let surface = surface_ctor(&device);

    let primitive_layout = primitives_layout(&device);
    let coordinate_matrix = coordinate_matrix_buffer_2d(&device, size.width, size.height);

    let msaa_count = anti_aliasing as u32;
    let color_pass = ColorPass::new(
      &device,
      surface.format(),
      &coordinate_matrix,
      &primitive_layout,
      msaa_count,
    );
    let texture_pass = ImagePass::new(&device, surface.format(), &primitive_layout, msaa_count);

    let multisample_framebuffer =
      Self::multisample_framebuffer(&device, size, surface.format(), msaa_count);
    WgpuGl {
      device,
      surface,
      queue,
      color_pass,
      img_pass: texture_pass,
      coordinate_matrix,
      primitives_layout: primitive_layout,
      empty_frame: true,
      vertex_buffers: None,
      anti_aliasing,
      multisample_framebuffer,
    }
  }

  #[inline]
  pub fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing) {
    if self.anti_aliasing != anti_aliasing {
      let Self {
        color_pass,
        img_pass,
        primitives_layout,
        surface,
        device,
        ..
      } = self;
      self.anti_aliasing = anti_aliasing;
      let msaa_count = anti_aliasing as u32;
      let format = surface.format();
      color_pass.set_anti_aliasing(msaa_count, primitives_layout, device, format);
      img_pass.set_anti_aliasing(msaa_count, primitives_layout, device, format);
    }
  }

  fn create_command_encoder(&self) -> wgpu::CommandEncoder {
    self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Create Encoder") })
  }

  fn multisample_framebuffer(
    device: &wgpu::Device,
    size: DeviceSize,
    format: wgpu::TextureFormat,
    sample_count: u32,
  ) -> Option<wgpu::TextureView> {
    (sample_count > 1).then(|| {
      let multisampled_texture_extent = wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth_or_array_layers: 1,
      };

      let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        size: multisampled_texture_extent,
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
      };

      device
        .create_texture(multisampled_frame_descriptor)
        .create_view(&wgpu::TextureViewDescriptor::default())
    })
  }

  fn create_primitives_bind_group<T: AsBytes>(&self, primitives: &[T]) -> wgpu::BindGroup {
    let primitives_buffer = self
      .device
      .create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Primitive Buffer"),
        contents: primitives.as_bytes(),
        usage: wgpu::BufferUsages::STORAGE,
      });
    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &self.primitives_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Buffer(primitives_buffer.as_entire_buffer_binding()),
      }],
      label: Some("Primitive buffer bind group"),
    })
  }

  fn write_vertex_buffer(&mut self, vertices: &[Vertex], indices: &[u32]) {
    let Self { device, vertex_buffers, .. } = self;
    let new_vertex_buffer = || {
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertices buffer"),
        contents: vertices.as_bytes(),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
      })
    };
    let new_index_buffer = || {
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: indices.as_bytes(),
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        label: Some("Indices buffer"),
      })
    };

    if let Some(buffers) = vertex_buffers {
      if buffers.vertex_size >= vertices.len() {
        self
          .queue
          .write_buffer(&buffers.vertices, 0, vertices.as_bytes());
      } else {
        buffers.vertices = new_vertex_buffer();
      }
      buffers.vertex_size = vertices.len();
      if buffers.index_size >= indices.len() {
        self
          .queue
          .write_buffer(&buffers.indices, 0, indices.as_bytes())
      } else {
        buffers.indices = new_index_buffer();
      }
      buffers.index_size = indices.len();
    } else {
      *vertex_buffers = Some(VertexBuffers {
        vertices: new_vertex_buffer(),
        vertex_size: vertices.len(),
        indices: new_index_buffer(),
        index_size: indices.len(),
      });
    }
  }
}

fn primitives_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
  device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &[wgpu::BindGroupLayoutEntry {
      binding: 0,
      visibility: wgpu::ShaderStages::VERTEX,
      ty: wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Storage { read_only: true },
        has_dynamic_offset: false,
        min_binding_size: None,
      },
      count: None,
    }],
    label: Some("Primitive layout (maybe changed every draw)"),
  })
}

fn coordinate_matrix_buffer_2d(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Buffer {
  device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    contents: [
      [2. / width as f32, 0., 0., 0.],
      [0., -2. / height as f32, 0., 0.],
      [0., 0., 1., 0.],
      [-1., 1., 0., 1.],
    ]
    .as_bytes(),
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    label: Some("2d coordinate transform buffer."),
  })
}

impl Vertex {
  fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
    use std::mem::size_of;
    wgpu::VertexBufferLayout {
      array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: &[
        wgpu::VertexAttribute {
          offset: 0,
          shader_location: 0,
          format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
          offset: (size_of::<[f32; 2]>()) as wgpu::BufferAddress,
          shader_location: 1,
          format: wgpu::VertexFormat::Uint32,
        },
      ],
    }
  }
}
