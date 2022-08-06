use std::collections::HashSet;
use std::ffi::CStr;
use std::ffi::c_void;
use ash::prelude::*;
use ash::{vk, Entry};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    dpi::PhysicalSize,
};
use ash::extensions::khr::{AndroidSurface, WaylandSurface, Win32Surface, XcbSurface, XlibSurface};
use ash::extensions::khr::Surface;
use ash::extensions::ext::DebugUtils;
use raw_window_handle::HasRawWindowHandle;

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

extern "system" fn debug_callback(
    _message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
    ) -> vk::Bool32 {
        print!("validation layer: {}", unsafe { CStr::from_ptr((*callback_data).p_message).to_str().unwrap() });
        vk::FALSE
    }

pub struct VulkanDetails {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
}    

pub struct HelloTriangleApplication {
    event_loop: winit::event_loop::EventLoop<()>,
    window: winit::window::Window,
    vulkan_details: VulkanDetails,
}

impl VulkanDetails {
    pub fn new(window: &winit::window::Window) -> Self {
        let (entry, instance, debug_messenger, surface, physical_device, device, graphics_queue, present_queue) = VulkanDetails::init_vulkan(&window);
        Self {
            entry: entry,
            instance: instance,
            debug_messenger: debug_messenger,
            surface: surface,
            physical_device: physical_device,
            device: device,
            graphics_queue: graphics_queue,
            present_queue: present_queue,
        }
    }
    fn init_vulkan(window: &winit::window::Window) -> (
        ash::Entry,
        ash::Instance,
        vk::DebugUtilsMessengerEXT,
        vk::SurfaceKHR,
        vk::PhysicalDevice,
        ash::Device,
        vk::Queue,
        vk::Queue,
    ) {
        let entry = Entry::linked();
        let instance = VulkanDetails::create_instance(&entry).unwrap();
        let debug_messenger = VulkanDetails::create_debug_messenger(&entry, &instance);
        let surface = VulkanDetails::create_surface(&window, &entry, &instance).unwrap();
        let physical_device = VulkanDetails::pick_physical_device(&entry, &instance, &surface).unwrap();
        let device = VulkanDetails::create_logical_device(&entry, &instance, &physical_device, &surface);
        let (graphics_queue_index, present_queue_index) = VulkanDetails::find_queue_familes(&entry, &instance, &physical_device, &surface);
        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_index.unwrap() as u32, 0) };
        let present_queue = unsafe { device.get_device_queue(present_queue_index.unwrap() as u32, 0) };
        (entry, instance, debug_messenger, surface, physical_device, device, graphics_queue, present_queue)
    }
    fn create_instance(entry: &ash::Entry) -> VkResult<ash::Instance> {
        if !VulkanDetails::check_validation_layer_support(&entry)
        {
            return Err(vk::Result::ERROR_INITIALIZATION_FAILED)
        }
        let app_info = vk::ApplicationInfo {
            s_type: vk::StructureType::APPLICATION_INFO,
            p_application_name: unsafe { CStr::from_bytes_with_nul_unchecked("Hello Triangle\0".as_bytes()).as_ptr() },
            application_version: vk::make_api_version(0, 1, 0, 0),
            p_engine_name: unsafe { CStr::from_bytes_with_nul_unchecked("No Engine\0".as_bytes()).as_ptr() },
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
            p_next: &VulkanDetails::populate_debug_messenger_create_info() as *const _ as *const c_void,
            ..Default::default()
        };
        unsafe { entry.create_instance(&create_info, None) }
    }
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
    fn create_debug_messenger(entry: &ash::Entry, instance: &ash::Instance) -> vk::DebugUtilsMessengerEXT {
        unsafe { DebugUtils::new(&entry, &instance).create_debug_utils_messenger(&VulkanDetails::populate_debug_messenger_create_info(), None).unwrap() }
    }
    fn create_surface(window: &winit::window::Window, entry: &ash::Entry, instance: &ash::Instance) -> VkResult<vk::SurfaceKHR> {
        match window.raw_window_handle() {
            raw_window_handle::RawWindowHandle::AndroidNdk(handle) => {
                let surface_create_info = vk::AndroidSurfaceCreateInfoKHR {
                    s_type: vk::StructureType::ANDROID_SURFACE_CREATE_INFO_KHR,
                    window: handle.a_native_window,
                    ..Default::default()
                };
                let android_surface = AndroidSurface::new(&entry, &instance);
                unsafe { android_surface.create_android_surface(&surface_create_info, None) }
            },
            raw_window_handle::RawWindowHandle::Win32(handle) => {
                let surface_create_info = vk::Win32SurfaceCreateInfoKHR {
                    s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
                    hwnd: handle.hwnd,
                    hinstance: handle.hinstance,
                    ..Default::default()
                };
                let win32_surface = Win32Surface::new(&entry, &instance);
                unsafe { win32_surface.create_win32_surface(&surface_create_info, None) }
            },
            raw_window_handle::RawWindowHandle::Wayland(handle) => {
                let surface_create_info = vk::WaylandSurfaceCreateInfoKHR {
                    s_type: vk::StructureType::WAYLAND_SURFACE_CREATE_INFO_KHR,
                    display: handle.surface,
                    ..Default::default()
                };
                let wayland_surface = WaylandSurface::new(&entry, &instance);
                unsafe { wayland_surface.create_wayland_surface(&surface_create_info, None) }
            },
            raw_window_handle::RawWindowHandle::Xcb(handle) => {
                let surface_create_info = vk::XcbSurfaceCreateInfoKHR {
                    s_type: vk::StructureType::XCB_SURFACE_CREATE_INFO_KHR,
                    window: handle.window,
                    ..Default::default()
                };
                let xcb_surface = XcbSurface::new(&entry, &instance);
                unsafe { xcb_surface.create_xcb_surface(&surface_create_info, None) }
            },
            raw_window_handle::RawWindowHandle::Xlib(handle) => {
                let surface_create_info = vk::XlibSurfaceCreateInfoKHR {
                    s_type: vk::StructureType::XLIB_SURFACE_CREATE_INFO_KHR,
                    window: handle.window,
                    ..Default::default()
                };
                let xlib_surface = XlibSurface::new(&entry, &instance);
                unsafe { xlib_surface.create_xlib_surface(&surface_create_info, None) }
            },
            _ => Err(vk::Result::ERROR_INITIALIZATION_FAILED)
        }
    }
    fn populate_debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
        vk::DebugUtilsMessengerCreateInfoEXT {
            s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE |
                              vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
                              vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL |
                          vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION |
                          vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            pfn_user_callback: Some(debug_callback),
            ..Default::default()
        }
    }
    fn pick_physical_device(entry: &ash::Entry, instance: &ash::Instance, surface: &vk::SurfaceKHR) -> VkResult<vk::PhysicalDevice> {
        let mut physical_device: Option<vk::PhysicalDevice> = None;
        let devices = unsafe { instance.enumerate_physical_devices().unwrap() };
        if devices.len() == 0 {
            return Err(vk::Result::ERROR_INITIALIZATION_FAILED)
        }
        for device in devices {
            if VulkanDetails::is_device_suitable(entry, instance, &device, surface) {
                physical_device = Some(device);
            }
        }
        physical_device.ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)
    }
    fn is_device_suitable(entry: &ash::Entry, instance: &ash::Instance, device: &vk::PhysicalDevice, surface: &vk::SurfaceKHR) -> bool {
        let (graphics_queue_index, present_queue_index) = VulkanDetails::find_queue_familes(entry, instance, device, surface);
        graphics_queue_index.is_some() && present_queue_index.is_some()
    }
    fn find_queue_familes(entry: &ash::Entry, instance: &ash::Instance, device: &vk::PhysicalDevice, surface: &vk::SurfaceKHR) -> (Option<usize>, Option<usize>) {
        let queue_family_properties = unsafe { instance.get_physical_device_queue_family_properties(*device) };
        let surface_details = Surface::new(entry, instance);
        ( queue_family_properties.iter().position(|&queue_family| queue_family.queue_flags & vk::QueueFlags::GRAPHICS == vk::QueueFlags::GRAPHICS),
          queue_family_properties.iter().enumerate().position(|(index, _)| unsafe { surface_details.get_physical_device_surface_support(*device, index as u32, *surface).unwrap() })
        )
    }
    fn create_logical_device(entry: &ash::Entry, instance: &ash::Instance, physical_device: &vk::PhysicalDevice, surface: &vk::SurfaceKHR) -> ash::Device {
        let (gq, pq) = VulkanDetails::find_queue_familes(entry, instance, physical_device, surface);
        let mut queues = HashSet::new();
        queues.insert(gq.unwrap() as u32);
        queues.insert(pq.unwrap() as u32);
        let mut device_queue_create_infos = Vec::new();
        for queue in queues {
            device_queue_create_infos.push(
                vk::DeviceQueueCreateInfo {
                    s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
                    queue_family_index: queue,
                    queue_count: 1,
                    p_queue_priorities: &1.0,
                    ..Default::default()
                }
            )
        };
        let device_features = vk::PhysicalDeviceFeatures {
            ..Default::default()
        };
        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DEVICE_CREATE_INFO,
            queue_create_info_count: device_queue_create_infos.len() as u32,
            p_queue_create_infos: device_queue_create_infos.as_ptr(),
            p_enabled_features: &device_features,
            enabled_layer_count: VALIDATION_LAYERS.len() as u32,
            pp_enabled_layer_names: VALIDATION_LAYERS.as_ptr(),
            ..Default::default()
        };
        unsafe { instance.create_device(*physical_device, &device_create_info, None).unwrap() }
    }
    fn cleanup(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            DebugUtils::new(&self.entry, &self.instance).destroy_debug_utils_messenger(self.debug_messenger, None);
            Surface::new(&self.entry, &self.instance).destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}

impl HelloTriangleApplication {
    pub fn new() -> Self {
        let (event_loop, window) = HelloTriangleApplication::init_window().unwrap();
        let vulkan_details = VulkanDetails::new(&window);
        Self {
            event_loop: event_loop,
            window: window,
            vulkan_details: vulkan_details,
        }
    }
    pub fn run(mut self) -> ! {
        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == self.window.id() => {
                    self.vulkan_details.cleanup();
                    *control_flow = ControlFlow::Exit
                },
                _ => (),
            }
        });
    }
    fn init_window() -> Result<(winit::event_loop::EventLoop<()>, winit::window::Window), winit::error::OsError> {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_resizable(false)
            .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT))
            .build(&event_loop)?;
        Ok((event_loop, window))
    }
}