use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::PixelSize;

pub struct WGPUContext {
    pub device: wgpu::Device,
    pub depthbuffer: wgpu::TextureView,
    pub framebuffer: wgpu::TextureView,
    pub msaa_depthbuffer: wgpu::TextureView,
    pub msaa_framebuffer: wgpu::TextureView,
    pub sample_count: u32,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub queue: wgpu::Queue,
}

impl WGPUContext {
    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        self.depthbuffer = depthbuffer(&self.device, width, height, 1);
        self.framebuffer = framebuffer(&self.device, width, height, self.surface_config.format, 1);
        self.msaa_depthbuffer = depthbuffer(&self.device, width, height, self.sample_count);
        self.msaa_framebuffer = framebuffer(
            &self.device,
            width,
            height,
            self.surface_config.format,
            self.sample_count,
        );
    }

    pub fn size(&self) -> PixelSize {
        PixelSize {
            width: self.surface_config.width,
            height: self.surface_config.height,
        }
    }
}

fn framebuffer(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    sample_count: u32,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
            label: Some("Frame buffer"),
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn depthbuffer(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    sample_count: u32,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
            label: Some("Depth buffer"),
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

pub async fn get_wgpu_context<W: HasWindowHandle + HasDisplayHandle>(
    window: &W,
    width: u32,
    height: u32,
) -> WGPUContext {
    let backends = if cfg!(windows) {
        wgpu::Backends::DX12
    } else {
        wgpu::Backends::PRIMARY
    };
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let surface = unsafe {
        instance
            .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_window_handle: window.window_handle().unwrap().as_raw(),
                raw_display_handle: Some(window.display_handle().unwrap().as_raw()),
            })
            .expect("Failed to get a surface")
    };
    // Maybe TODO: Figure out how to set this dynamically?
    let sample_count = 4; // Max supported on OSX
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to get an adapter");

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            label: None,
            memory_hints: Default::default(),
            trace: Default::default(),
            experimental_features: Default::default(),
        })
        .await
        .expect("Failed to get a device");

    let surface_caps = surface.get_capabilities(&adapter);
    let format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| !f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);

    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width,
        height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &surface_config);

    let depthbuff = depthbuffer(&device, width, height, 1);
    let framebuff = framebuffer(&device, width, height, surface_config.format, 1);
    let msaa_depthbuffer = depthbuffer(&device, width, height, sample_count);
    let msaa_framebuffer = framebuffer(&device, width, height, surface_config.format, sample_count);

    WGPUContext {
        surface,
        surface_config,
        depthbuffer: depthbuff,
        framebuffer: framebuff,
        msaa_framebuffer,
        msaa_depthbuffer,
        device,
        queue,
        sample_count,
    }
}
