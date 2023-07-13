use crate::RineApplication;

pub struct EguiIntegrator {
    state: egui_winit::State,
    context: egui::Context,
    renderer: egui_wgpu::Renderer,
}

impl EguiIntegrator {
    pub(crate) fn new(window_client: &crate::RineWindowClient) -> Self {
        Self {
            state: egui_winit::State::new(window_client.window()),
            context: egui::Context::default(),
            renderer: egui_wgpu::Renderer::new(window_client.device(), window_client.config().format, None, 1),
        }
    }

    pub fn on_event(&mut self, event: &winit::event::WindowEvent<'_>) -> egui_winit::EventResponse {
        self.state.on_event(&self.context, event)
    }

    pub fn redraw<A: RineApplication>(&mut self, window_client: &crate::RineWindowClient, commands: &mut Vec<wgpu::CommandBuffer>, encoder: &mut wgpu::CommandEncoder, framebuffer_view: &wgpu::TextureView, application: &mut A) {
        let window = &window_client.window();
        let surface_config = &window_client.config();
        let device = &window_client.device();
        let queue = &window_client.queue();
        let ctx = &self.context;
        ctx.begin_frame(self.state.take_egui_input(window_client.window()));
        application.egui_draw(&self.context);
        let output = ctx.end_frame();
        let paint_jobs = ctx.tessellate(output.shapes);

        self.state.handle_platform_output(&window, ctx, output.platform_output);

        let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: [surface_config.width, surface_config.height],
            pixels_per_point: window.scale_factor() as f32,
        };
        let texture_delta = &output.textures_delta;
        for (tid, image_delta) in &texture_delta.set {
            self.renderer.update_texture(&device, &queue, *tid, &image_delta);
        }
        commands.extend(self.renderer.update_buffers(&device, &queue, encoder, &paint_jobs, &screen_descriptor));

        // let egui_view = self.target.create_view(&wgpu::TextureViewDescriptor::default());

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &framebuffer_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    // load: wgpu::LoadOp::Clear(wgpu::Color {
                    //     r: 0.0,
                    //     g: 0.0,
                    //     b: 0.0,
                    //     a: 0.0,
                    // }),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        self.renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
        std::mem::drop(render_pass);
        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        // let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        //     label: Some("Rine - Egui Blip"),
        //     color_attachments: &[
        //         Some(wgpu::RenderPassColorAttachment {
        //             view: &framebuffer_view,
        //             resolve_target: None,
        //             ops: wgpu::Operations {
        //                 ..Default::default()
        //             }
        //         })
        //     ],
        //     ..Default::default()
        // });
        //     // BLIP HERE
        // std::mem::drop(render_pass);
    }
}
