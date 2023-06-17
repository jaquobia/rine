mod rine_main_example {

    use rine::RineApplication;

    pub struct TestApplication {}

    impl RineApplication for TestApplication {
        type Implementor = TestApplication;

        fn create() -> TestApplication {
            TestApplication {}
        }

        fn handle_event<T>(&mut self, event: &winit::event::Event<T>, window_client: &rine::RineWindowClient) {
            if let winit::event::Event::MainEventsCleared = event {
                let render_result = {
                    // let output = window_client.surface.get_current_texture().expect("Could not create the surface texture!");
                    let output = window_client.surface.get_current_texture().unwrap();

                    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder: wgpu::CommandEncoder = window_client.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Test(Main) Command Encoder"),
                    });
                    let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Test(Main) Render Pass"),
                        color_attachments: &[
                            Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 0., b: 0., a: 0. }), store: true }
                            })
                        ],
                        ..Default::default()
                    });
                    std::mem::drop(render_pass); // stop borrowing encoder mutably
                    window_client.queue.submit(std::iter::once(encoder.finish()));
                    output.present();
                }; // Render Result
            } // Main events cleared
        }
    }
 
}

fn main() {
    simple_logger::SimpleLogger::new().with_level(log::LevelFilter::Warn).env().init().unwrap();
    rine::start_rine_application::<rine_main_example::TestApplication>();
}
