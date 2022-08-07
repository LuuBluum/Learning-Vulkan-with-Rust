use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::Surface;
use ash::extensions::khr::Win32Surface;
use ash::{prelude, vk, Entry};
use std::ffi::c_void;
use std::ffi::CStr;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

const VALIDATION_LAYERS: &[*const i8] = &[unsafe {
    CStr::from_bytes_with_nul_unchecked("VK_LAYER_KHRONOS_validation\0".as_bytes()).as_ptr()
}];

const REQUIRED_EXTENSIONS: &[*const i8] = &[
    Surface::name().as_ptr(),
    Win32Surface::name().as_ptr(),
    DebugUtils::name().as_ptr(),
];

extern "system" fn debug_callback(
    _message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    print!("validation layer: {}", unsafe {
        CStr::from_ptr((*callback_data).p_message).to_str().unwrap()
    });
    vk::FALSE
}

pub struct HelloTriangleApplication {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    graphics_queue: vk::Queue,
}

impl HelloTriangleApplication {
    pub fn new() -> Self {
        let (entry, instance, debug_messenger, physical_device, device, graphics_queue) =
            HelloTriangleApplication::init_vulkan();
        Self {
            entry: entry,
            instance: instance,
            debug_messenger: debug_messenger,
            physical_device: physical_device,
            device: device,
            graphics_queue: graphics_queue,
        }
    }
    fn init_vulkan() -> (
        ash::Entry,
        ash::Instance,
        vk::DebugUtilsMessengerEXT,
        vk::PhysicalDevice,
        ash::Device,
        vk::Queue,
    ) {
        let entry = Entry::linked();
        let instance = HelloTriangleApplication::create_instance(&entry).unwrap();
        let debug_messenger = HelloTriangleApplication::create_debug_messenger(&entry, &instance);
        let physical_device = HelloTriangleApplication::pick_physical_device(&instance);
        let device = HelloTriangleApplication::create_logical_device(&instance, &physical_device);
        let graphics_queue = unsafe {
            device.get_device_queue(
                HelloTriangleApplication::find_queue_familes(&instance, &physical_device).unwrap()
                    as u32,
                0,
            )
        };
        (
            entry,
            instance,
            debug_messenger,
            physical_device,
            device,
            graphics_queue,
        )
    }
    fn create_instance(entry: &ash::Entry) -> prelude::VkResult<ash::Instance> {
        if !HelloTriangleApplication::check_validation_layer_support(&entry) {
            panic!("Could not find support for all layers!");
        }
        let app_info = vk::ApplicationInfo {
            s_type: vk::StructureType::APPLICATION_INFO,
            p_application_name: unsafe {
                CStr::from_bytes_with_nul_unchecked("Hello Triangle\0".as_bytes()).as_ptr()
            },
            application_version: vk::make_api_version(0, 1, 0, 0),
            p_engine_name: unsafe {
                CStr::from_bytes_with_nul_unchecked("No Engine\0".as_bytes()).as_ptr()
            },
            engine_version: vk::make_api_version(0, 1, 0, 0),
            api_version: vk::make_api_version(0, 1, 0, 0),
            ..Default::default()
        };
        let create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            enabled_layer_count: VALIDATION_LAYERS.len() as u32,
            pp_enabled_layer_names: VALIDATION_LAYERS.as_ptr(),
            enabled_extension_count: REQUIRED_EXTENSIONS.len() as u32,
            pp_enabled_extension_names: REQUIRED_EXTENSIONS.as_ptr(),
            p_next: &HelloTriangleApplication::populate_debug_messenger_create_info() as *const _
                as *const c_void,
            ..Default::default()
        };
        unsafe { entry.create_instance(&create_info, None) }
    }
    fn check_validation_layer_support(entry: &ash::Entry) -> bool {
        let layer_properties = entry.enumerate_instance_layer_properties().unwrap();
        for layer in VALIDATION_LAYERS {
            if let None = layer_properties.iter().find(|l| {
                // This horrible construction is because Vulkan operates with C strings and Rust does not
                unsafe {
                    &CStr::from_ptr(l.layer_name.as_ptr()).to_str().unwrap()
                        == &CStr::from_ptr(*layer).to_str().unwrap()
                }
            }) {
                return false;
            }
        }
        true
    }
    fn create_debug_messenger(
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> vk::DebugUtilsMessengerEXT {
        unsafe {
            DebugUtils::new(&entry, &instance)
                .create_debug_utils_messenger(
                    &HelloTriangleApplication::populate_debug_messenger_create_info(),
                    None,
                )
                .unwrap()
        }
    }
    fn populate_debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
        vk::DebugUtilsMessengerCreateInfoEXT {
            s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            pfn_user_callback: Some(debug_callback),
            ..Default::default()
        }
    }
    fn pick_physical_device(instance: &ash::Instance) -> vk::PhysicalDevice {
        let mut physical_device: Option<vk::PhysicalDevice> = None;
        let devices = unsafe { instance.enumerate_physical_devices().unwrap() };
        if devices.len() == 0 {
            panic!("Failed to find GPUs with Vulkan support!");
        }
        for device in devices {
            if HelloTriangleApplication::is_device_suitable(&instance, &device) {
                physical_device = Some(device);
            }
        }
        physical_device.unwrap()
    }
    fn is_device_suitable(instance: &ash::Instance, device: &vk::PhysicalDevice) -> bool {
        HelloTriangleApplication::find_queue_familes(instance, device).is_some()
    }
    fn find_queue_familes(instance: &ash::Instance, device: &vk::PhysicalDevice) -> Option<usize> {
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(*device) };
        queue_family_properties.iter().position(|&queue_family| {
            queue_family.queue_flags & vk::QueueFlags::GRAPHICS == vk::QueueFlags::GRAPHICS
        })
    }
    fn create_logical_device(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
    ) -> ash::Device {
        let device_queue_create_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            queue_family_index: HelloTriangleApplication::find_queue_familes(
                instance,
                physical_device,
            )
            .unwrap() as u32,
            queue_count: 1,
            p_queue_priorities: &1.0,
            ..Default::default()
        };
        let device_features = vk::PhysicalDeviceFeatures {
            ..Default::default()
        };
        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DEVICE_CREATE_INFO,
            queue_create_info_count: 1,
            p_queue_create_infos: &device_queue_create_info,
            p_enabled_features: &device_features,
            enabled_layer_count: VALIDATION_LAYERS.len() as u32,
            pp_enabled_layer_names: VALIDATION_LAYERS.as_ptr(),
            ..Default::default()
        };
        unsafe {
            instance
                .create_device(*physical_device, &device_create_info, None)
                .unwrap()
        }
    }
    pub fn run(mut self) -> ! {
        let (event_loop, window) = HelloTriangleApplication::init_window().unwrap();
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == window.id() => {
                    self.cleanup();
                    *control_flow = ControlFlow::Exit
                }
                _ => (),
            }
        });
    }
    fn init_window(
    ) -> Result<(winit::event_loop::EventLoop<()>, winit::window::Window), winit::error::OsError>
    {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_resizable(false)
            .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT))
            .build(&event_loop)?;
        Ok((event_loop, window))
    }
    fn cleanup(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            DebugUtils::new(&self.entry, &self.instance)
                .destroy_debug_utils_messenger(self.debug_messenger, None);
            self.instance.destroy_instance(None);
        }
    }
}
