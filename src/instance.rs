use std::ffi::CStr;
use std::ffi::c_void;
use ash::{prelude, vk, Entry};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    dpi::PhysicalSize,
};
use ash::extensions::khr::Win32Surface;
use ash::extensions::khr::Surface;
use ash::extensions::ext::DebugUtils;

const WIDTH : u32 = 800;
const HEIGHT : u32 = 600;

const VALIDATION_LAYERS : &[*const i8] = &[
    unsafe { CStr::from_bytes_with_nul_unchecked("VK_LAYER_KHRONOS_validation\0".as_bytes()).as_ptr() }
];

const REQUIRED_EXTENSIONS : &[*const i8] = &[
    Surface::name().as_ptr(),
    Win32Surface::name().as_ptr(),
    DebugUtils::name().as_ptr(),
];

pub struct HelloTriangleApplication {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

extern "system" fn debug_callback(
    _message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
    ) -> vk::Bool32 {
        print!("validation layer: {}", unsafe { CStr::from_ptr((*callback_data).p_message).to_str().unwrap() });
        vk::FALSE
    }

impl HelloTriangleApplication {
    fn check_validation_layer_support(entry: &ash::Entry) -> bool
    {
        let layer_properties = entry.enumerate_instance_layer_properties().unwrap();
        for layer in VALIDATION_LAYERS {
            if let None = layer_properties.iter().find(|l| {
                // This horrible construction is because Vulkan operates with C strings and Rust does not
               unsafe { &CStr::from_ptr(l.layer_name.as_ptr()).to_str().unwrap() == &CStr::from_ptr(*layer).to_str().unwrap() }
            })
            {
                return false
            }
        }
        true
    }
    fn init_vulkan(entry: &ash::Entry) -> prelude::VkResult<ash::Instance> {
        if !HelloTriangleApplication::check_validation_layer_support(&entry)
        {
            panic!("Could not find support for all layers!");
        }
        let app_info = vk::ApplicationInfo {
            api_version: vk::make_api_version(0, 1, 0, 0),
            ..Default::default()
        };
        let mut create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            ..Default::default()
        };
        create_info.enabled_layer_count = VALIDATION_LAYERS.len() as u32;
        create_info.pp_enabled_layer_names = VALIDATION_LAYERS.as_ptr();
        create_info.enabled_extension_count = REQUIRED_EXTENSIONS.len() as u32;
        create_info.pp_enabled_extension_names = REQUIRED_EXTENSIONS.as_ptr();
        create_info.p_next = &HelloTriangleApplication::setup_debug_messenger() as *const _ as *const c_void;
        unsafe { entry.create_instance(&create_info, None) }
    }
    fn setup_debug_messenger() -> vk::DebugUtilsMessengerCreateInfoEXT {
        vk::DebugUtilsMessengerCreateInfoEXT {
            s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            pfn_user_callback: Some(debug_callback),
            ..Default::default()
        }
    }
    pub fn new() -> Self {
        let entry = Entry::linked();
        let instance = HelloTriangleApplication::init_vulkan(&entry).unwrap();
        let debug_messenger = unsafe { DebugUtils::new(&entry, &instance).create_debug_utils_messenger(&HelloTriangleApplication::setup_debug_messenger(), None).unwrap() };
        Self {
            entry: entry,
            instance: instance,
            debug_messenger: debug_messenger,
        }
    }
    fn init_window() -> Result<(winit::event_loop::EventLoop<()>, winit::window::Window), winit::error::OsError> {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_resizable(false)
            .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT))
            .build(&event_loop)?;
        Ok((event_loop, window))
    }
    fn cleanup(&mut self)
    {
        unsafe {
            DebugUtils::new(&self.entry, &self.instance).destroy_debug_utils_messenger(self.debug_messenger, None);
            self.instance.destroy_instance(None);
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
                },
                _ => (),
            }
        });
    }
}