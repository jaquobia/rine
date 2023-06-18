/// Create a window (winit) and graphics api context (wgpu), then start polling events and passing it to a  
/// [RineApplication]
/// The application requires a custom create method, but all other methods are optional
/// for customizing the window and graphics api
pub fn start_rine_application<A: RineApplication + 'static>() {
    let event_loop = winit::event_loop::EventLoop::new();
    let window_builder = A::configure_window(winit::window::WindowBuilder::new().with_title("Rine Application!"));
    let window = match window_builder.build(&event_loop) {
        Ok(window) => window,
        Err(e) => { log::error!("Could not create a window! {}", e); return; },
    };
    let mut window_client = {
        let (width, height): (u32, u32) = window.inner_size().into();

        let backends = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
        let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            dx12_shader_compiler,
            ..Default::default()
        });

        let surface = unsafe {
            instance.create_surface(&window).expect("Window does not have a surface")
        };

        let adapter =
            pollster::block_on(wgpu::util::initialize_adapter_from_env_or_default(&instance, backends, Some(&surface)))
            .expect("No suitable GPU adapters found on the system!");
        log::info!("Created gpu adapter: {:?}", adapter.get_info());

        let adapter_features = adapter.features();
        let adapter_capabilities = adapter.get_downlevel_capabilities();
        let adapter_limits = adapter.limits();
        // let adapter_limits = wgpu::Limits::default();

        log::info!("Adapter capabilities: {:?}", adapter_capabilities);

        let (required_features, optional_features) = A::gpu_features();
        let requested_downlevel_capabilities = A::gpu_downlevel_capabilities();
        let requested_limits = A::gpu_limits();

        if !adapter_features.contains(required_features) {
            log::error!("Adapter does not support reqested features! {:?}", required_features - adapter_features);
        }

        if !adapter_capabilities.flags.contains(requested_downlevel_capabilities.flags) {
            log::error!("Adapter does not support the requested downlevel capabilities! {:?}", adapter_capabilities.flags - requested_downlevel_capabilities.flags);
        }

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some(&window.title()),
            features: (optional_features & adapter_features) | required_features,
            limits: requested_limits.using_resolution(adapter_limits),
        }, 
        None,
        )).expect("Could not find a suitable device for the adapter!");

        let mut config = surface.get_default_config(&adapter, width, height).expect("Surface configuration incompatible with the adapter!");
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities.formats[0].clone();
        // let config = wgpu::SurfaceConfiguration {
        //     usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        //     view_formats: vec![],
        //     format,
        //     width,
        //     height,
        //     present_mode: wgpu::PresentMode::AutoVsync,
        //     alpha_mode: wgpu::CompositeAlphaMode::Auto,
        // };
        surface.configure(&device, &config);

        RineWindowClient { window, instance, surface, config, adapter, device, queue }
    };

    let mut application: A = A::create();

    event_loop.run(move |event, window_target, control_flow| {
        let _ = &window_client;
        match event {
            winit::event::Event::WindowEvent {
                event:
                    winit::event::WindowEvent::Resized(size)
                    | winit::event::WindowEvent::ScaleFactorChanged {
                        new_inner_size: &mut size,
                        ..
                    },
                    ..
            } => {
                {
                    let config = &mut window_client.config;
                    config.width = size.width.max(1);
                    config.height = size.height.max(1);
                    window_client.surface.configure(&window_client.device, &config);
                }
                application.resize(&mut window_client);
                // rine_window_client.surface.configure(&rine_window_client.device, &rine_window_client.config);
            },
            winit::event::Event::WindowEvent { window_id, event } if event == winit::event::WindowEvent::Destroyed || event == winit::event::WindowEvent::CloseRequested => {
                control_flow.set_exit();
            } // Window Events
            _ => { application.handle_event(&event, &window_client); }
        }
    });
}

#[derive(Debug)]
/// Holds all the necessary structs for manipulating the gpu
/// and window
pub struct RineWindowClient {
    pub window: winit::window::Window,
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

/// Defines the interface necessary for an application to run
/// Must implement the [RineApplication::create] function and
/// define the [RineApplication::Implementor] as the structs type
/// (a bit recursive - yes, but im too dumb to get around that issue)
pub trait RineApplication {
    fn create() -> Self;

    /// Apply configurations to the winit window
    fn configure_window(window_builder: winit::window::WindowBuilder) -> winit::window::WindowBuilder { window_builder }

    /// Return (Required features, Optional Features)
    fn gpu_features() -> (wgpu::Features, wgpu::Features) {
        (wgpu::Features::empty(), wgpu::Features::empty())
    }

    /// Return needed downlevel capabilities
    fn gpu_downlevel_capabilities() -> wgpu::DownlevelCapabilities {
        wgpu::DownlevelCapabilities::default()
    }

    /// Return needed limits
    fn gpu_limits() -> wgpu::Limits { wgpu::Limits::default() }

    /// Handle the polled events
    fn handle_event<T>(&mut self, event: &winit::event::Event<T>, window_client: &RineWindowClient) {}

    fn resize(&mut self, window_client: &RineWindowClient) {}
}
