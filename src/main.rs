use std::ffi::CString;
use std::ptr;

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::{Surface, XlibSurface};
use ash::{vk, vk_make_version};
use ash::version::{EntryV1_0, InstanceV1_0};

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::unix::{WindowBuilderExtUnix, XWindowType};
use winit::window::{Window, WindowBuilder};

fn required_extension_names() -> Vec<*const i8> {
    // Why is Surface not enough?
    vec![
        Surface::name().as_ptr(),
        XlibSurface::name().as_ptr(),
        DebugUtils::name().as_ptr(),
    ]
}

struct VulkanApp {
    entry: ash::Entry,
    instance: ash::Instance,
}

impl VulkanApp {
    fn new() -> VulkanApp {
        let entry = ash::Entry::new().unwrap();
        let instance = Self::create_instance(&entry);
        VulkanApp {
            entry,
            instance,
        }
    }

    fn run(self) {
        let event_loop = EventLoop::new();
        Self::main_loop(event_loop);
    }

    fn create_instance(entry: &ash::Entry) -> ash::Instance {
        let app_name = CString::new("Hello Triangle").unwrap();
        // You can create structs with builders
        let engine_name = CString::new("No Engine").unwrap();
        let extension_names = required_extension_names();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(vk_make_version!(1, 0, 0))
            .engine_name(&engine_name)
            .engine_version(vk_make_version!(1, 0, 0))
            .api_version(vk_make_version!(1, 1, 0))
            .build();

        // Or plainly specify struct's fields
        let create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            enabled_extension_count: extension_names.len() as u32,
            pp_enabled_extension_names: extension_names.as_ptr(),
            // You can do ..Deafult::default() instead of passing defaults explicitly
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_next: ptr::null(),
            enabled_layer_count: 0,
            pp_enabled_layer_names: ptr::null(),
            flags: vk::InstanceCreateFlags::empty(),
        };

        let instance: ash::Instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("Failed to create instance")
        };
        instance
    }

    fn init_window(event_loop: &EventLoop<()>) -> Window {
        WindowBuilder::new()
            .with_title("Fcking Vulkan")
            .with_inner_size((800, 600).into())
            // Special for my i3wm, to foce window to be floating
            .with_x11_window_type(XWindowType::Dialog)
            .build(&event_loop)
            .expect("Failed to create a window")
    }

    fn main_loop(event_loop: EventLoop<()>) {
        let window = Self::init_window(&event_loop);
        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::EventsCleared => {
                    // Update application here
                    window.request_redraw();
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    // Render here
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                _ => *control_flow = ControlFlow::Poll,
            }
        });
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None)
        }
    }
}

fn main() {
    let app = VulkanApp::new();
    app.run();
}
