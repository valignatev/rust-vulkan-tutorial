use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::{Surface, XlibSurface};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::{vk, vk_make_version};

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::unix::{WindowBuilderExtUnix, WindowExtUnix, XWindowType};
use winit::window::{Window, WindowBuilder};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

unsafe fn create_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: &Window,
) -> Result<vk::SurfaceKHR, vk::Result> {
    let x11_display = window.xlib_display().unwrap();
    let x11_window = window.xlib_window().unwrap();
    let x11_create_info = vk::XlibSurfaceCreateInfoKHR {
        window: x11_window as vk::Window,
        dpy: x11_display as *mut vk::Display,
        ..Default::default()
    };
    let xlib_surface_loader = XlibSurface::new(entry, instance);
    xlib_surface_loader.create_xlib_surface(&x11_create_info, None)
}

fn vk_to_string(raw_array: &[c_char]) -> String {
    let raw_string = unsafe { CStr::from_ptr(raw_array.as_ptr()) };
    raw_string
        .to_str()
        .expect("Failed to convert raw string.")
        .to_owned()
}

fn required_extension_names() -> Vec<*const i8> {
    // Why is Surface not enough?
    vec![
        Surface::name().as_ptr(),
        XlibSurface::name().as_ptr(),
        DebugUtils::name().as_ptr(),
    ]
}

// NOTE: in the production code you won't probably hardcode these names,
//  as vulkan header file provides a macro for them, and ash structs have
//  `name` associated function to get them: Swapchain::name()
const REQUIRED_VALIDATION_LAYERS: [&'static str; 1] = ["VK_LAYER_KHRONOS_validation"];
const DEVICE_EXTENSIONS: [&'static str; 1] = ["VK_KHR_swapchain"];

fn enabled_validation_layer_names() -> Vec<CString> {
    // Can't inline raw_names
    // because the CString contents gets moved and dropped
    // It'll result in "Layer not found" Vk error.
    REQUIRED_VALIDATION_LAYERS
        .iter()
        .map(|&layer_name| CString::new(layer_name).unwrap())
        .collect()
}

// The callback function used in Debug Utils
unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let severity = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[Verbose]",
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[Warning]",
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[Error]",
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[Info]",
        _ => "[Unknown]",
    };
    let types = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
        _ => "[Unknown]",
    };
    let message = CStr::from_ptr((*p_callback_data).p_message);
    println!("[Debug]{}{}{:?}", severity, types, message);

    vk::FALSE
}

struct QueueFamilyIndices {
    graphics_family: Option<u32>,
    present_family: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn new() -> Self {
        Self {
            graphics_family: None,
            present_family: None,
        }
    }
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}

struct SurfaceStuff {
    surface_loader: ash::extensions::khr::Surface,
    surface: vk::SurfaceKHR,
}

struct SwapChainSupportDetails {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

struct SwapchainStuff {
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    swapchain_format: vk::Format,
    swapchain_extent: vk::Extent2D,
    swapchain_images: Vec<vk::Image>,
}

struct VulkanApp {
    _entry: ash::Entry,
    _physical_device: vk::PhysicalDevice,
    _graphics_queue: vk::Queue,
    _present_queue: vk::Queue,
    instance: ash::Instance,
    device: ash::Device,
    surface_loader: ash::extensions::khr::Surface,
    surface: vk::SurfaceKHR,
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    _swapchain_images: Vec<vk::Image>,
    _swapchain_format: vk::Format,
    _swapchain_extent: vk::Extent2D,
}

impl VulkanApp {
    fn new(window: &Window) -> VulkanApp {
        let entry = ash::Entry::new().unwrap();
        let instance = Self::create_instance(&entry);
        let surface_stuff = Self::create_surface(&entry, &instance, &window);
        let physical_device = Self::pick_physical_device(&instance, &surface_stuff);
        let (logical_device, graphics_queue, present_queue) =
            Self::create_logical_device(&instance, physical_device, &surface_stuff);
        let (debug_utils_loader, debug_messenger) = Self::setup_debug_utils(&entry, &instance);
        let swapchain_stuff =
            Self::create_swapchain(&instance, physical_device, &logical_device, &surface_stuff);
        VulkanApp {
            _entry: entry,
            instance,
            surface: surface_stuff.surface,
            surface_loader: surface_stuff.surface_loader,
            debug_utils_loader,
            debug_messenger,

            _physical_device: physical_device,
            device: logical_device,

            _graphics_queue: graphics_queue,
            _present_queue: present_queue,

            swapchain_loader: swapchain_stuff.swapchain_loader,
            swapchain: swapchain_stuff.swapchain,
            _swapchain_images: swapchain_stuff.swapchain_images,
            _swapchain_format: swapchain_stuff.swapchain_format,
            _swapchain_extent: swapchain_stuff.swapchain_extent,
        }
    }

    fn create_instance(entry: &ash::Entry) -> ash::Instance {
        if Self::check_validation_layers_support(entry) == false {
            panic!("Validation layers requested, but not available");
        }
        let app_name = CString::new("Hello Triangle").unwrap();
        let engine_name = CString::new("No Engine").unwrap();
        // You can create vk structs with builders
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(vk_make_version!(1, 0, 0))
            .engine_name(&engine_name)
            .engine_version(vk_make_version!(1, 0, 0))
            .api_version(vk_make_version!(1, 1, 0))
            .build();

        // This creates info used to debug issues in vk::createInstance and
        // vk::destroyInstance
        let debug_utils_create_info = populate_debug_messenger_create_info();

        // Provides VK_EXT debug utils
        let extension_names = required_extension_names();

        let enabled_layer_raw_names = enabled_validation_layer_names();

        let enabled_layer_names: Vec<*const c_char> = enabled_layer_raw_names
            .iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect();
        // You can create structs plainly by providing all fields
        let create_info = vk::InstanceCreateInfo {
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_next: &debug_utils_create_info as *const vk::DebugUtilsMessengerCreateInfoEXT
                as *const c_void,
            p_application_info: &app_info,
            enabled_extension_count: extension_names.len() as u32,
            pp_enabled_extension_names: extension_names.as_ptr(),
            enabled_layer_count: enabled_layer_names.len() as u32,
            pp_enabled_layer_names: enabled_layer_names.as_ptr(),
            flags: vk::InstanceCreateFlags::empty(),
        };

        let instance: ash::Instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("Failed to create instance")
        };
        instance
    }

    fn create_surface(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: &Window,
    ) -> SurfaceStuff {
        let surface =
            unsafe { create_surface(entry, instance, window).expect("Failed to create surface") };
        let surface_loader = Surface::new(entry, instance);

        SurfaceStuff {
            surface_loader,
            surface,
        }
    }

    fn pick_physical_device(
        instance: &ash::Instance,
        surface_stuff: &SurfaceStuff,
    ) -> vk::PhysicalDevice {
        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .expect("Failed to enumerate physical devices")
        };
        println!(
            "Found {} devices with Vulkan support",
            physical_devices.len()
        );
        for &physical_device in physical_devices.iter() {
            if Self::is_device_suitable(instance, physical_device, surface_stuff) {
                return physical_device;
            }
        }
        panic!("No suitable physical devices");
    }

    fn is_device_suitable(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        surface_stuff: &SurfaceStuff,
    ) -> bool {
        // More features can be queried with `get_physical_device_features`
        let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };

        let device_type = match device_properties.device_type {
            vk::PhysicalDeviceType::CPU => "Cpu",
            vk::PhysicalDeviceType::INTEGRATED_GPU => "Integrated GPU",
            vk::PhysicalDeviceType::DISCRETE_GPU => "Discrete GPU",
            vk::PhysicalDeviceType::VIRTUAL_GPU => "Virtual GPU",
            vk::PhysicalDeviceType::OTHER => "Unknown",
            _ => panic!("Matching on device type failed"),
        };
        let device_name = vk_to_string(&device_properties.device_name);
        println!(
            "\tDevice Name: {}, id: {}, type: {}",
            device_name, device_properties.device_id, device_type,
        );

        let indices = Self::find_queue_family(instance, physical_device, surface_stuff);
        let is_queue_family_supported = indices.is_complete();
        let is_device_extension_supported =
            Self::check_device_extension_support(instance, physical_device);
        let is_swapchain_adequate = if is_device_extension_supported {
            let swapchain_support = Self::query_swapchain_support(physical_device, surface_stuff);
            !swapchain_support.formats.is_empty() && !swapchain_support.present_modes.is_empty()
        } else {
            false
        };
        is_queue_family_supported && is_device_extension_supported && is_swapchain_adequate
    }

    fn find_queue_family(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        surface_stuff: &SurfaceStuff,
    ) -> QueueFamilyIndices {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        let mut queue_family_indices = QueueFamilyIndices::new();

        for (index, queue_family) in queue_families.iter().enumerate() {
            if queue_family.queue_count > 0
                && queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
            {
                queue_family_indices.graphics_family = Some(index as u32);
            }

            let is_present_support = unsafe {
                surface_stuff
                    .surface_loader
                    .get_physical_device_surface_support(
                        physical_device,
                        index as u32,
                        surface_stuff.surface,
                    )
            };
            if queue_family.queue_count > 0 && is_present_support {
                queue_family_indices.present_family = Some(index as u32);
            }

            if queue_family_indices.is_complete() {
                break;
            }
        }

        queue_family_indices
    }

    fn check_device_extension_support(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> bool {
        let available_extensions = unsafe {
            instance
                .enumerate_device_extension_properties(physical_device)
                .expect("Failed to get device extension properties.")
        };
        let mut available_extension_names = vec![];
        println!("\tAvailable Device Extensions: ");
        for extension in available_extensions.iter() {
            let extension_name = vk_to_string(&extension.extension_name);
            println!(
                "\t\tName: {}, Version: {}",
                extension_name, extension.spec_version
            );
            available_extension_names.push(extension_name);
        }
        let mut required_extensions = HashSet::new();
        for extension in DEVICE_EXTENSIONS.iter() {
            required_extensions.insert(extension.to_string());
        }
        for extension_name in available_extension_names.iter() {
            required_extensions.remove(extension_name);
        }
        required_extensions.is_empty()
    }

    fn query_swapchain_support(
        physical_device: vk::PhysicalDevice,
        surface_stuff: &SurfaceStuff,
    ) -> SwapChainSupportDetails {
        unsafe {
            let capabilities = surface_stuff
                .surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface_stuff.surface)
                .expect("Failed to query for surface capabilities");
            let formats = surface_stuff
                .surface_loader
                .get_physical_device_surface_formats(physical_device, surface_stuff.surface)
                .expect("Failed to query for surface formats");
            let present_modes = surface_stuff
                .surface_loader
                .get_physical_device_surface_present_modes(physical_device, surface_stuff.surface)
                .expect("Failed to query for surface present modes");
            SwapChainSupportDetails {
                capabilities,
                formats,
                present_modes,
            }
        }
    }

    fn choose_swapchain_format(
        available_formats: &Vec<vk::SurfaceFormatKHR>,
    ) -> vk::SurfaceFormatKHR {
        // Check if list contains most widely used R8G8B8A8 format with nonlinear color space
        for available_format in available_formats {
            if available_format.format == vk::Format::R8G8B8A8_UNORM
                && available_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            {
                return available_format.clone();
            }
        }

        // Return the first format from the list
        return available_formats.first().unwrap().clone();
    }

    fn choose_swapchain_present_mode(
        available_present_modes: &Vec<vk::PresentModeKHR>,
    ) -> vk::PresentModeKHR {
        for &available_present_mode in available_present_modes.iter() {
            if available_present_mode == vk::PresentModeKHR::MAILBOX {
                return available_present_mode;
            }
        }
        vk::PresentModeKHR::FIFO
    }

    fn choose_swap_extent(capabilities: &vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::max_value() {
            capabilities.current_extent
        } else {
            vk::Extent2D {
                width: capabilities
                    .min_image_extent
                    .width
                    .max(WIDTH)
                    .min(capabilities.max_image_extent.width),
                height: capabilities
                    .min_image_extent
                    .height
                    .max(HEIGHT)
                    .min(capabilities.max_image_extent.height),
            }
        }
    }

    fn create_swapchain(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: &ash::Device,
        surface_stuff: &SurfaceStuff,
    ) -> SwapchainStuff {
        let swapchain_support = Self::query_swapchain_support(physical_device, surface_stuff);
        let surface_format = Self::choose_swapchain_format(&swapchain_support.formats);
        let present_mode = Self::choose_swapchain_present_mode(&swapchain_support.present_modes);
        let extent = Self::choose_swap_extent(&swapchain_support.capabilities);
        // Sometimes we may have to wait on the driver to complete its stuff before
        // we can acquire another image to render to. Therefore it's recommended to
        // request at least one more image than the minimum
        let mut image_count = swapchain_support.capabilities.min_image_count + 1;
        if swapchain_support.capabilities.max_image_count > 0
            && image_count > swapchain_support.capabilities.max_image_count
        {
            image_count = swapchain_support.capabilities.max_image_count;
        }

        let indices = Self::find_queue_family(instance, physical_device, surface_stuff);

        let mut create_info = vk::SwapchainCreateInfoKHR {
            surface: surface_stuff.surface,
            min_image_count: image_count,
            image_format: surface_format.format,
            image_color_space: surface_format.color_space,
            image_extent: extent,
            // This is always 1 unless you are developing a stereoscopic 3D app.
            image_array_layers: 1,
            // We render into images in the swapchain, so they're used as color
            // attachment
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            pre_transform: swapchain_support.capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode,
            clipped: vk::TRUE,
            old_swapchain: vk::SwapchainKHR::null(),
            ..Default::default()
        };

        if indices.graphics_family != indices.present_family {
            create_info.image_sharing_mode = vk::SharingMode::CONCURRENT;
            create_info.queue_family_index_count = 2;
            create_info.p_queue_family_indices = vec![
                indices.graphics_family.unwrap(),
                indices.present_family.unwrap(),
            ]
            .as_ptr();
        } else {
            create_info.image_sharing_mode = vk::SharingMode::EXCLUSIVE;
        }

        let swapchain_loader = ash::extensions::khr::Swapchain::new(instance, device);
        let swapchain = unsafe {
            swapchain_loader
                .create_swapchain(&create_info, None)
                .expect("Failed to create Swapchain")
        };

        let swapchain_images = unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain)
                .expect("Failed to get Swapchain Images.")
        };

        SwapchainStuff {
            swapchain_loader,
            swapchain,
            swapchain_format: surface_format.format,
            swapchain_extent: extent,
            swapchain_images,
        }
    }

    fn create_logical_device(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        surface_stuff: &SurfaceStuff,
    ) -> (ash::Device, vk::Queue, vk::Queue) {
        let indices = Self::find_queue_family(instance, physical_device, surface_stuff);
        let graphics_family = indices.graphics_family.unwrap();
        let present_family = indices.present_family.unwrap();

        let mut unique_queue_families = HashSet::new();
        unique_queue_families.insert(graphics_family);
        unique_queue_families.insert(present_family);

        let queue_priorities = [1.0_f32];
        let mut queue_create_infos = vec![];
        for &queue_family in unique_queue_families.iter() {
            let queue_create_info = vk::DeviceQueueCreateInfo {
                queue_family_index: queue_family,
                p_queue_priorities: queue_priorities.as_ptr(),
                queue_count: queue_priorities.len() as u32,
                ..Default::default()
            };
            queue_create_infos.push(queue_create_info);
        }

        let physical_device_features = vk::PhysicalDeviceFeatures {
            ..Default::default() // default is just enable no features.
        };

        let enabled_layer_raw_names = enabled_validation_layer_names();
        let enabled_layer_names: Vec<*const c_char> = enabled_layer_raw_names
            .iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect();

        let enabled_extension_names = [ash::extensions::khr::Swapchain::name().as_ptr()];

        let device_create_info = vk::DeviceCreateInfo {
            queue_create_info_count: queue_create_infos.len() as u32,
            p_queue_create_infos: queue_create_infos.as_ptr(),
            enabled_layer_count: enabled_layer_names.len() as u32,
            pp_enabled_layer_names: enabled_layer_names.as_ptr(),
            p_enabled_features: &physical_device_features,
            enabled_extension_count: enabled_extension_names.len() as u32,
            pp_enabled_extension_names: enabled_extension_names.as_ptr(),
            ..Default::default()
        };

        // TODO: figure out reasons why logical device should be created
        //  through the instance call. In the C/C++, logical device ton't interact
        //  directly with instances and instance isn't used in logical device
        //  creation.
        let device: ash::Device = unsafe {
            instance
                .create_device(physical_device, &device_create_info, None)
                .expect("Failed to create logical Device!")
        };
        let graphics_queue = unsafe { device.get_device_queue(graphics_family, 0) };
        let present_queue = unsafe { device.get_device_queue(present_family, 0) };
        (device, graphics_queue, present_queue)
    }

    fn setup_debug_utils(
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> (ash::extensions::ext::DebugUtils, vk::DebugUtilsMessengerEXT) {
        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);

        let messenger_create_info = populate_debug_messenger_create_info();
        let utils_messenger = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&messenger_create_info, None)
                .expect("Failed to create debug utils messenger")
        };
        (debug_utils_loader, utils_messenger)
    }

    fn draw_frame(&mut self) {}

    fn run(mut self, event_loop: EventLoop<()>, window: Window) {
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
                } => {
                    self.draw_frame();
                    *control_flow = ControlFlow::Exit;
                }
                _ => *control_flow = ControlFlow::Poll,
            }
        });
    }

    fn check_validation_layers_support(entry: &ash::Entry) -> bool {
        let layer_properties = entry
            .enumerate_instance_layer_properties()
            .expect("Failed to enumerate Instance Layer Properties!");
        if layer_properties.len() <= 0 {
            eprintln!("No available layers.");
            return false;
        } else {
            println!("Instance Available layers: ");
            for layer in layer_properties.iter() {
                let layer_name = vk_to_string(&layer.layer_name);
                println!("\t{}", layer_name);
            }
        }

        for required_layer_name in REQUIRED_VALIDATION_LAYERS.iter() {
            let mut is_found = false;
            for layer_property in layer_properties.iter() {
                let test_layer_name = vk_to_string(&layer_property.layer_name);
                if (*required_layer_name) == test_layer_name {
                    is_found = true;
                    break;
                }
            }
            if !is_found {
                return false;
            }
        }

        true
    }
}

fn populate_debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
    vk::DebugUtilsMessengerCreateInfoEXT {
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
            // | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
            // | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
            | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        pfn_user_callback: Some(vulkan_debug_utils_callback),
        ..Default::default()
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_messenger, None);
            self.instance.destroy_instance(None);
        }
    }
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

fn main() {
    let event_loop = EventLoop::new();
    let window = init_window(&event_loop);
    let app = VulkanApp::new(&window);
    // TODO: Not sure if moving window here is a good idea
    app.run(event_loop, window);
}
