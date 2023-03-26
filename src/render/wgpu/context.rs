use raw_window_handle::HasRawWindowHandle;
use wgpu;

pub struct WGPUContext {
    pub device: wgpu::Device,
    pub depthbuffer: wgpu::TextureView,
    pub framebuffer: wgpu::TextureView,
    pub msaa_depthbuffer: wgpu::TextureView,
    pub msaa_framebuffer: wgpu::TextureView,
    pub sample_count: u32,
    pub surface: wgpu::Surface,
    pub swap_chain: wgpu::SwapChain,
    pub swap_chain_desc: wgpu::SwapChainDescriptor,
    pub queue: wgpu::Queue,
}

impl WGPUContext {
    pub fn resize(&mut self, width: u32, height: u32) {
        self.swap_chain_desc.width = width;
        self.swap_chain_desc.height = height;
        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.swap_chain_desc);
        self.depthbuffer = depthbuffer(&self.device, width, height, 1);
        self.framebuffer = framebuffer(
            &self.device,
            width,
            height,
            self.swap_chain_desc.format,
            1
        );
        self.msaa_depthbuffer = depthbuffer(&self.device, width, height, self.sample_count);
        self.msaa_framebuffer= framebuffer(
            &self.device,
            width,
            height,
            self.swap_chain_desc.format,
            self.sample_count,
        );
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
                depth: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
            label: Some("Frame buffer"),
        }).create_view(&wgpu::TextureViewDescriptor::default())
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
                depth: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            label: Some("Depth buffer"),
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

pub async fn get_wgpu_context<W: HasRawWindowHandle>(
    window: &W,
    width: u32,
    height: u32,
) -> WGPUContext {
    let backend = if cfg!(windows) {
        // Vulkan now works better for me than DX12 ¯\_(ツ)_/¯
        wgpu::BackendBit::VULKAN
        // wgpu::BackendBit::DX12
    } else {
        wgpu::BackendBit::PRIMARY
    };
    let instance = wgpu::Instance::new(backend);
    let surface = unsafe { instance.create_surface(window) };
    // Maybe TODO: Figure out how to set this dynamically?
    let sample_count = 4; // Max supported on OSX
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
            compatible_surface: Some(&surface),
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: adapter.features(),
                limits: wgpu::Limits::default(),
                shader_validation: true,
            },
            None,
        )
        .await
        .unwrap();

    let swap_chain_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT, // We are drawing to the window
        format: wgpu::TextureFormat::Bgra8UnormSrgb, // https://github.com/gfx-rs/wgpu-rs/issues/123
        width,
        height,
        present_mode: wgpu::PresentMode::Immediate, // Should this be Mailbox? This appears to lower the average render time by 2ms and I see no tearing, so I'll leave it for now
    };

    let swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);
    let depthbuff = depthbuffer(&device, width, height, 1);
    let framebuff = framebuffer(&device, width, height, swap_chain_desc.format, 1);
    let msaa_depthbuffer = depthbuffer(&device, width, height, sample_count);
    let msaa_framebuffer = framebuffer(&device, width, height, swap_chain_desc.format, sample_count);

    WGPUContext {
        surface,
        depthbuffer: depthbuff,
        framebuffer: framebuff,
        msaa_framebuffer,
        msaa_depthbuffer,
        device,
        queue,
        swap_chain,
        swap_chain_desc,
        sample_count,
    }
}
