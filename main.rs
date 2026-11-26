//! SCGE - The Super Cool Game Engine
//! Phase 1: lightweight Vulkan window + clear-screen renderer.

mod renderer;

use renderer::VulkanRenderer;
use std::error::Error;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

struct App {
    window: Option<Window>,
    renderer: Option<VulkanRenderer>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("SCGE")
            .with_inner_size(winit::dpi::LogicalSize::new(960.0, 540.0));

        let window = event_loop
            .create_window(attributes)
            .expect("failed to create window");

        let renderer = unsafe {
            VulkanRenderer::new(&window).expect("failed to initialize Vulkan")
        };

        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    if let Some(renderer) = self.renderer.as_mut() {
                        unsafe {
                            renderer.resize(size.width, size.height)
                                .expect("failed to resize Vulkan surface");
                        }
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    unsafe {
                        renderer.draw_frame().expect("failed to render frame");
                    }
                }
                window.request_redraw();
            }

            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
