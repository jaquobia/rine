mod rine_main_example {

    use rine::RineApplication;
    use winit::keyboard::{KeyCode, ModifiersState};

    pub struct TestApplication { input_manager: rine_input_manager::InputManager, winit_helper: winit_input_helper::WinitInputHelper }

    impl TestApplication {
    }

    impl RineApplication for TestApplication {

        fn create(_window_client: &rine::RineWindowClient) -> Self {
            let mut input_manager = rine_input_manager::InputManager::new();
            input_manager.register_input("forward", KeyCode::KeyW, ModifiersState::SHIFT, rine_input_manager::InputState::Pressed | rine_input_manager::InputState::Held);
            Self { input_manager, winit_helper: winit_input_helper::WinitInputHelper::new() }
        }

        fn draw(&mut self, _window_client: &rine::RineWindowClient, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
            
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Test(Main) Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 0., b: 0., a: 0. }), store: wgpu::StoreOp::Store }
                    })
                ],
                ..Default::default()
            });
            std::mem::drop(render_pass); // stop borrowing encoder mutably
        }

        
        #[cfg(feature = "egui-int")]
        fn egui_draw(&mut self, ctx: &egui::Context) {
            egui::Window::new("Rine Demo Window").show(ctx, |ui| {
                ui.label("Some Text");
            });
        }

        fn handle_event<T>(&mut self, event: &winit::event::Event<T>, _window_client: &mut rine::RineWindowClient) -> bool {
            if self.winit_helper.update(event) {
                let inputs = &self.input_manager;
                if inputs.get_input("forward", &self.winit_helper) {
                    log::warn!("Forward was pressed!");
                }
            } // update
            return false;
            
        }
    }
 
}

fn main() {
    simple_logger::SimpleLogger::new().with_level(log::LevelFilter::Warn).env().init().unwrap();
    rine::start_rine_application::<rine_main_example::TestApplication>();
}
