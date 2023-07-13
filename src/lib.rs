#[cfg(feature = "egui-int")]
mod egui_integration;

#[cfg(feature = "egui-int")]
pub use egui;

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

        let config = surface.get_default_config(&adapter, width, height).expect("Surface configuration incompatible with the adapter!");
        surface.configure(&device, &config);

        RineWindowClient { window, instance, surface, config, adapter, device, queue }
    };

    let mut application: A = A::create(&window_client);

    #[cfg(feature = "egui-int")]
    let mut egui_int = egui_integration::EguiIntegrator::new(&window_client);

    let mut system_request_redraw = false;
    let mut gpu_redraw = false;

    event_loop.run(move |event, _window_target, control_flow| {
        let _ = &window_client;
        
        #[cfg(feature = "egui-int")]
        if let winit::event::Event::WindowEvent {event, window_id} = &event {
            let response = egui_int.on_event(&event);
            if response.repaint {
                window_client.window().request_redraw(); // either use this or set the boolean
                // system_request_redraw = true;
            }
            if response.consumed {
                return;
            }
        }
        
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
                window_client.resize_surface(size.into());
                application.resize(size.into(), &mut window_client);
            },
            winit::event::Event::WindowEvent { window_id: _, event } if event == winit::event::WindowEvent::Destroyed || event == winit::event::WindowEvent::CloseRequested => {
                control_flow.set_exit();
            }, // Window Events
            winit::event::Event::RedrawRequested(_window) => {
                system_request_redraw = true;
            },
            winit::event::Event::MainEventsCleared => {
                gpu_redraw = true;
                application.handle_event(&event, control_flow, &mut window_client);
                
            }
            _ => { application.handle_event(&event, control_flow, &mut window_client); }
        }

        if gpu_redraw { 
            gpu_redraw = false;
            // begin render work
            let mut commands = vec![];

            let output_frame = match window_client.surface().get_current_texture() {
                Ok(frame) => frame,
                // This error occurs when the app is minimized on Windows.
                // Silently return here to prevent spamming the console with:
                // "The underlying surface has changed, and therefore the swap chain must be updated"
                Err(wgpu::SurfaceError::Outdated) => { return; }
                Err(wgpu::SurfaceError::Lost) => { window_client.resize_in_place(); return; }
                Err(wgpu::SurfaceError::OutOfMemory) => { log::error!("Ran out of memory! Shutting down!"); control_flow.set_exit(); return; }
                Err(e) => { log::error!("Dropped frame with error: {}", e); return; }
            };
            let output_view = output_frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            
            let mut encoder = window_client.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Rine main(render) thread encoder"),
            });

            application.draw(&window_client, &mut encoder, &output_view);

            // redraw elements that update every frame
            if system_request_redraw {
                system_request_redraw = false;
                // redraw all elements that only need to be appplied on dirty
            } // if system redraw

            #[cfg(feature = "egui-int")]
            egui_int.redraw(&window_client, &mut commands, &mut encoder, &output_view, &mut application);
            commands.extend(std::iter::once(encoder.finish()));

            // submit render work
            window_client.queue().submit(commands.into_iter());
            output_frame.present();
        }

    });
}

#[derive(Debug)]
/// Holds all the necessary structs for manipulating the gpu
/// and window
pub struct RineWindowClient {
    window: winit::window::Window,
    instance: wgpu::Instance,
    surface: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl RineWindowClient {

    /// Get reference to the winit window handle
    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }

    /// Get reference to the wgpu instance
    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    /// Get reference to the wgpu surface
    /// Might add mutable accessor incase hal backend is needed
    pub fn surface(&self) -> &wgpu::Surface {
        &self.surface
    }

    /// Get reference to the wgpu surface configuration
    /// Might add mutator functions for the config fields
    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    /// Get reference to the wgpu adapter
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    /// Get reference to the wgpu device
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Get reference to the wgpu queue
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// The window has changed, resize the framebuffer
    pub fn resize_surface(&mut self, new_size: (u32, u32)) {
        if new_size.0 > 0 && new_size.1 > 0 {
            self.config.width = new_size.0.max(1);
            self.config.height = new_size.1.max(1);
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Should be called whenever the surface is lost [wgpu::SurfacrError::Lost]
    pub fn resize_in_place(&mut self) {
        self.resize_surface((self.config.width, self.config.height));
    }

    /// Returns whether vsync is enabled
    pub fn get_vsync(&self) -> bool {
        self.config.present_mode == wgpu::PresentMode::AutoVsync
    }

    /// Change the state of vsync
    pub fn set_vsync(&mut self, vsync: bool) {
        self.config.present_mode = if vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
    }

    /// Toggle the state of vsync
    pub fn toggle_vsync(&mut self) {
        self.set_vsync(self.config.present_mode == wgpu::PresentMode::AutoNoVsync);
    }
}

/// Defines the interface necessary for an application to run
/// Must implement the [RineApplication::create] function and
/// define the [RineApplication::Implementor] as the structs type
/// (a bit recursive - yes, but im too dumb to get around that issue)
pub trait RineApplication {
    fn create(window_client: &RineWindowClient) -> Self;

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
    fn handle_event<T>(&mut self, event: &winit::event::Event<T>, control_flow: &mut winit::event_loop::ControlFlow, window_client: &mut RineWindowClient) {}

    fn resize(&mut self, size: (u32, u32), window_client: &RineWindowClient) {}

    fn draw(&mut self, window_client: &RineWindowClient, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) { }

    // fn draw_requested(&mut self, window_client: &RineWindowClient) -> Vec<wgpu::CommandBuffer> { vec![] }

    #[cfg(feature = "egui-int")]
    fn egui_draw(&mut self, ctx: &egui::Context) {}
}
