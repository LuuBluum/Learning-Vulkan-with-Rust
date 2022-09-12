use ash::extensions::{
    ext::DebugUtils,
    khr::Surface,
    khr::Swapchain,
    khr::{AndroidSurface, WaylandSurface, Win32Surface, XcbSurface, XlibSurface},
};
use ash::prelude::*;
use ash::{vk, Entry};
use glam::{Vec2, Vec3};
use memoffset::offset_of;
use raw_window_handle::HasRawWindowHandle;
use std::collections::HashSet;
use std::ffi::{c_void, CStr};
use std::fs;
use std::mem::size_of;
use std::ptr;
use std::time::SystemTime;
use std::vec::Vec;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

const MAX_FRAMES_IN_FLIGHT: usize = 2;

const VALIDATION_LAYERS: &[*const i8] = &[unsafe {
    CStr::from_bytes_with_nul_unchecked("VK_LAYER_KHRONOS_validation\0".as_bytes()).as_ptr()
}];

const DEVICE_EXTENSIONS: &[*const i8] =
    &[unsafe { CStr::from_bytes_with_nul_unchecked("VK_KHR_swapchain\0".as_bytes()).as_ptr() }];

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

#[repr(C)]
struct Vertex {
    pos: glam::Vec2,
    color: glam::Vec3,
}

impl Vertex {
    fn get_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }
    fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, color) as u32,
            },
        ]
    }
}

const VERTICES: [Vertex; 4] = [
    Vertex {
        pos: Vec2 { x: -0.5, y: -0.5 },
        color: Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
    },
    Vertex {
        pos: Vec2 { x: 0.5, y: -0.5 },
        color: Vec3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
    },
    Vertex {
        pos: Vec2 { x: 0.5, y: 0.5 },
        color: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
    },
    Vertex {
        pos: Vec2 { x: -0.5, y: 0.5 },
        color: Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    },
];

const INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

#[repr(C)]
struct UniformBufferObject {
    model: glam::Mat4,
    view: glam::Mat4,
    proj: glam::Mat4,
}

struct SwapchainSupportDetails {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainSupportDetails {
    pub fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &vk::PhysicalDevice,
        surface: &vk::SurfaceKHR,
    ) -> Self {
        let surface_interface = Surface::new(entry, instance);
        Self {
            capabilities: unsafe {
                surface_interface
                    .get_physical_device_surface_capabilities(*device, *surface)
                    .unwrap()
            },
            formats: unsafe {
                surface_interface
                    .get_physical_device_surface_formats(*device, *surface)
                    .unwrap()
            },
            present_modes: unsafe {
                surface_interface
                    .get_physical_device_surface_present_modes(*device, *surface)
                    .unwrap()
            },
        }
    }
}

struct VulkanDetails {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    swap_chain: vk::SwapchainKHR,
    swap_chain_images: Vec<vk::Image>,
    swap_chain_image_format: vk::Format,
    swap_chain_extent: vk::Extent2D,
    swap_chain_image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,
    swap_chain_framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    uniform_buffers: Vec<vk::Buffer>,
    uniform_buffers_memory: Vec<vk::DeviceMemory>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    framebuffer_resized: bool,
    current_frame: usize,
    start_time: SystemTime,
}

pub struct HelloTriangleApplication {
    event_loop: winit::event_loop::EventLoop<()>,
    window: winit::window::Window,
    vulkan_details: VulkanDetails,
}

impl VulkanDetails {
    pub fn new(window: &winit::window::Window) -> Self {
        let entry = Entry::linked();
        let instance = VulkanDetails::create_instance(&entry).unwrap();
        let debug_messenger = VulkanDetails::create_debug_messenger(&entry, &instance);
        let surface = VulkanDetails::create_surface(&window, &entry, &instance).unwrap();
        let physical_device =
            VulkanDetails::pick_physical_device(&entry, &instance, &surface).unwrap();
        let device =
            VulkanDetails::create_logical_device(&entry, &instance, &physical_device, &surface);
        let (graphics_queue_index, present_queue_index) =
            VulkanDetails::find_queue_familes(&entry, &instance, &physical_device, &surface);
        let graphics_queue =
            unsafe { device.get_device_queue(graphics_queue_index.unwrap() as u32, 0) };
        let present_queue =
            unsafe { device.get_device_queue(present_queue_index.unwrap() as u32, 0) };
        let (swap_chain, swap_chain_images, swap_chain_image_format, swap_chain_extent) =
            VulkanDetails::create_swap_chain(
                window,
                &entry,
                &instance,
                &physical_device,
                &device,
                &surface,
            );
        let swap_chain_image_views = VulkanDetails::create_image_views(
            &device,
            &swap_chain_images,
            &swap_chain_image_format,
        );
        let render_pass = VulkanDetails::create_render_pass(&device, &swap_chain_image_format);
        let descriptor_set_layout = VulkanDetails::create_descriptor_set_layout(&device);
        let (pipeline_layout, graphics_pipeline) =
            VulkanDetails::create_graphics_pipeline(&device, &render_pass, &descriptor_set_layout);
        let swap_chain_framebuffers = VulkanDetails::create_framebuffers(
            &device,
            &swap_chain_image_views,
            &swap_chain_extent,
            &render_pass,
        );
        let command_pool = VulkanDetails::create_command_pool(
            &entry,
            &instance,
            &physical_device,
            &device,
            &surface,
        );
        let (vertex_buffer, vertex_buffer_memory) = VulkanDetails::create_vertex_buffer(
            &instance,
            &physical_device,
            &device,
            &command_pool,
            &graphics_queue,
        );
        let (index_buffer, index_buffer_memory) = VulkanDetails::create_index_buffer(
            &instance,
            &physical_device,
            &device,
            &command_pool,
            &graphics_queue,
        );
        let (uniform_buffers, uniform_buffers_memory) =
            VulkanDetails::create_uniform_buffers(&instance, &physical_device, &device);
        let descriptor_pool = VulkanDetails::create_descriptor_pool(&device);
        let descriptor_sets = VulkanDetails::create_descriptor_sets(
            &device,
            &uniform_buffers,
            &descriptor_set_layout,
            &descriptor_pool,
        );
        let command_buffers = VulkanDetails::create_command_buffers(&device, &command_pool);
        let (image_available_semaphores, render_finished_semaphores, in_flight_fences) =
            VulkanDetails::create_sync_objects(&device);
        Self {
            entry,
            instance,
            debug_messenger,
            surface,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swap_chain,
            swap_chain_images,
            swap_chain_image_format,
            swap_chain_extent,
            swap_chain_image_views,
            render_pass,
            descriptor_set_layout,
            pipeline_layout,
            graphics_pipeline,
            swap_chain_framebuffers,
            command_pool,
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            uniform_buffers,
            uniform_buffers_memory,
            descriptor_pool,
            descriptor_sets,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            framebuffer_resized: false,
            current_frame: 0,
            start_time: SystemTime::UNIX_EPOCH,
        }
    }
    fn create_instance(entry: &ash::Entry) -> VkResult<ash::Instance> {
        if !VulkanDetails::check_validation_layer_support(&entry) {
            return Err(vk::Result::ERROR_INITIALIZATION_FAILED);
        }
        let app_info = vk::ApplicationInfo {
            s_type: vk::StructureType::APPLICATION_INFO,
            p_application_name: CStr::from_bytes_with_nul("Hello Triangle\0".as_bytes())
                .unwrap()
                .as_ptr(),
            application_version: vk::make_api_version(0, 1, 0, 0),
            p_engine_name: CStr::from_bytes_with_nul("No Engine\0".as_bytes())
                .unwrap()
                .as_ptr(),
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
            p_next: &VulkanDetails::populate_debug_messenger_create_info() as *const _
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
                    &VulkanDetails::populate_debug_messenger_create_info(),
                    None,
                )
                .unwrap()
        }
    }
    fn create_surface(
        window: &winit::window::Window,
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> VkResult<vk::SurfaceKHR> {
        match window.raw_window_handle() {
            raw_window_handle::RawWindowHandle::AndroidNdk(handle) => {
                let surface_create_info = vk::AndroidSurfaceCreateInfoKHR {
                    s_type: vk::StructureType::ANDROID_SURFACE_CREATE_INFO_KHR,
                    window: handle.a_native_window,
                    ..Default::default()
                };
                let android_surface = AndroidSurface::new(&entry, &instance);
                unsafe { android_surface.create_android_surface(&surface_create_info, None) }
            }
            raw_window_handle::RawWindowHandle::Win32(handle) => {
                let surface_create_info = vk::Win32SurfaceCreateInfoKHR {
                    s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
                    hwnd: handle.hwnd,
                    hinstance: handle.hinstance,
                    ..Default::default()
                };
                let win32_surface = Win32Surface::new(&entry, &instance);
                unsafe { win32_surface.create_win32_surface(&surface_create_info, None) }
            }
            raw_window_handle::RawWindowHandle::Wayland(handle) => {
                let surface_create_info = vk::WaylandSurfaceCreateInfoKHR {
                    s_type: vk::StructureType::WAYLAND_SURFACE_CREATE_INFO_KHR,
                    display: handle.surface,
                    ..Default::default()
                };
                let wayland_surface = WaylandSurface::new(&entry, &instance);
                unsafe { wayland_surface.create_wayland_surface(&surface_create_info, None) }
            }
            raw_window_handle::RawWindowHandle::Xcb(handle) => {
                let surface_create_info = vk::XcbSurfaceCreateInfoKHR {
                    s_type: vk::StructureType::XCB_SURFACE_CREATE_INFO_KHR,
                    window: handle.window,
                    ..Default::default()
                };
                let xcb_surface = XcbSurface::new(&entry, &instance);
                unsafe { xcb_surface.create_xcb_surface(&surface_create_info, None) }
            }
            raw_window_handle::RawWindowHandle::Xlib(handle) => {
                let surface_create_info = vk::XlibSurfaceCreateInfoKHR {
                    s_type: vk::StructureType::XLIB_SURFACE_CREATE_INFO_KHR,
                    window: handle.window,
                    ..Default::default()
                };
                let xlib_surface = XlibSurface::new(&entry, &instance);
                unsafe { xlib_surface.create_xlib_surface(&surface_create_info, None) }
            }
            _ => Err(vk::Result::ERROR_INITIALIZATION_FAILED),
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
    fn pick_physical_device(
        entry: &ash::Entry,
        instance: &ash::Instance,
        surface: &vk::SurfaceKHR,
    ) -> VkResult<vk::PhysicalDevice> {
        let mut physical_device: Option<vk::PhysicalDevice> = None;
        let devices = unsafe { instance.enumerate_physical_devices().unwrap() };
        if devices.len() == 0 {
            return Err(vk::Result::ERROR_INITIALIZATION_FAILED);
        }
        for device in devices {
            if VulkanDetails::is_device_suitable(entry, instance, &device, surface) {
                physical_device = Some(device);
            }
        }
        physical_device.ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)
    }
    fn is_device_suitable(
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &vk::PhysicalDevice,
        surface: &vk::SurfaceKHR,
    ) -> bool {
        let (graphics_queue_index, present_queue_index) =
            VulkanDetails::find_queue_familes(entry, instance, device, surface);
        let swap_chain_support = SwapchainSupportDetails::new(entry, instance, device, surface);
        graphics_queue_index.is_some()
            && present_queue_index.is_some()
            && VulkanDetails::check_device_extension_support(instance, device)
            && !swap_chain_support.formats.is_empty()
            && !swap_chain_support.present_modes.is_empty()
    }
    fn check_device_extension_support(
        instance: &ash::Instance,
        device: &vk::PhysicalDevice,
    ) -> bool {
        let extension_properties = unsafe {
            instance
                .enumerate_device_extension_properties(*device)
                .unwrap()
        };
        for device_extension in DEVICE_EXTENSIONS {
            if extension_properties
                .iter()
                .find(|extension_property| unsafe {
                    &CStr::from_ptr(extension_property.extension_name.as_ptr())
                        .to_str()
                        .unwrap()
                        == &CStr::from_ptr(*device_extension).to_str().unwrap()
                })
                .is_none()
            {
                return false;
            }
        }
        true
    }
    fn find_queue_familes(
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &vk::PhysicalDevice,
        surface: &vk::SurfaceKHR,
    ) -> (Option<usize>, Option<usize>) {
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(*device) };
        let surface_details = Surface::new(entry, instance);
        (
            queue_family_properties.iter().position(|&queue_family| {
                queue_family.queue_flags & vk::QueueFlags::GRAPHICS == vk::QueueFlags::GRAPHICS
            }),
            queue_family_properties
                .iter()
                .enumerate()
                .position(|(index, _)| unsafe {
                    surface_details
                        .get_physical_device_surface_support(*device, index as u32, *surface)
                        .unwrap()
                }),
        )
    }
    fn create_logical_device(
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        surface: &vk::SurfaceKHR,
    ) -> ash::Device {
        let (gq, pq) = VulkanDetails::find_queue_familes(entry, instance, physical_device, surface);
        let mut queues = HashSet::new();
        queues.insert(gq.unwrap() as u32);
        queues.insert(pq.unwrap() as u32);
        let mut device_queue_create_infos = Vec::new();
        for queue in queues {
            device_queue_create_infos.push(vk::DeviceQueueCreateInfo {
                s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
                queue_family_index: queue,
                queue_count: 1,
                p_queue_priorities: &1.0,
                ..Default::default()
            })
        }
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
            enabled_extension_count: DEVICE_EXTENSIONS.len() as u32,
            pp_enabled_extension_names: DEVICE_EXTENSIONS.as_ptr(),
            ..Default::default()
        };
        unsafe {
            instance
                .create_device(*physical_device, &device_create_info, None)
                .unwrap()
        }
    }
    fn create_swap_chain(
        window: &winit::window::Window,
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        device: &ash::Device,
        surface: &vk::SurfaceKHR,
    ) -> (vk::SwapchainKHR, Vec<vk::Image>, vk::Format, vk::Extent2D) {
        let swap_chain_support =
            SwapchainSupportDetails::new(entry, instance, physical_device, surface);
        let format = VulkanDetails::choose_swap_surface_format(swap_chain_support.formats);
        let present_mode =
            VulkanDetails::choose_swap_present_mode(swap_chain_support.present_modes);
        let image_count = {
            if swap_chain_support.capabilities.max_image_count > 0
                && swap_chain_support.capabilities.min_image_count
                    > swap_chain_support.capabilities.max_image_count
            {
                swap_chain_support.capabilities.max_image_count
            } else {
                swap_chain_support.capabilities.min_image_count + 1
            }
        };
        let extent = VulkanDetails::choose_swap_extent(window, &swap_chain_support.capabilities);
        let (graphics_queue_index, present_mode_index) =
            VulkanDetails::find_queue_familes(entry, instance, physical_device, surface);
        let queue_index_equivalent = graphics_queue_index.unwrap() == present_mode_index.unwrap();
        let queue_family_indices = vec![graphics_queue_index.unwrap(), present_mode_index.unwrap()];
        let create_info = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            surface: *surface,
            min_image_count: image_count,
            image_format: format.format,
            image_color_space: format.color_space,
            image_extent: extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: if queue_index_equivalent {
                vk::SharingMode::EXCLUSIVE
            } else {
                vk::SharingMode::CONCURRENT
            },
            queue_family_index_count: if queue_index_equivalent { 0 } else { 2 },
            p_queue_family_indices: if queue_index_equivalent {
                ptr::null()
            } else {
                queue_family_indices.as_ptr() as *const u32
            },
            pre_transform: swap_chain_support.capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode,
            clipped: vk::TRUE,
            old_swapchain: vk::SwapchainKHR::null(),
            ..Default::default()
        };
        let swap_chain_handle = Swapchain::new(instance, device);
        let swap_chain = unsafe {
            swap_chain_handle
                .create_swapchain(&create_info, None)
                .unwrap()
        };
        let swap_chain_images =
            unsafe { swap_chain_handle.get_swapchain_images(swap_chain).unwrap() };
        (swap_chain, swap_chain_images, format.format, extent)
    }
    fn choose_swap_surface_format(formats: Vec<vk::SurfaceFormatKHR>) -> vk::SurfaceFormatKHR {
        for available_format in &formats {
            if available_format.format == vk::Format::B8G8R8A8_SRGB
                && available_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            {
                return *available_format;
            }
        }
        formats[0]
    }
    fn choose_swap_present_mode(present_modes: Vec<vk::PresentModeKHR>) -> vk::PresentModeKHR {
        for available_present_mode in present_modes {
            if available_present_mode == vk::PresentModeKHR::MAILBOX {
                return available_present_mode;
            }
        }
        vk::PresentModeKHR::FIFO
    }
    fn choose_swap_extent(
        window: &winit::window::Window,
        capabilities: &vk::SurfaceCapabilitiesKHR,
    ) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            let window_size = window.inner_size();
            vk::Extent2D {
                width: window_size.width.clamp(
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width,
                ),
                height: window_size.height.clamp(
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height,
                ),
            }
        }
    }
    fn create_image_views(
        device: &ash::Device,
        swap_chain_images: &Vec<vk::Image>,
        swap_chain_image_format: &vk::Format,
    ) -> Vec<vk::ImageView> {
        let mut output_vec = Vec::new();
        for image in swap_chain_images {
            let create_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                image: *image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: *swap_chain_image_format,
                components: vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                },
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            unsafe { output_vec.push(device.create_image_view(&create_info, None).unwrap()) };
        }
        output_vec
    }
    fn create_render_pass(
        device: &ash::Device,
        swap_chain_image_format: &vk::Format,
    ) -> vk::RenderPass {
        let color_attachment = vk::AttachmentDescription {
            format: *swap_chain_image_format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        };

        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };

        let subpass = vk::SubpassDescription {
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            ..Default::default()
        };

        let dependency = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(),
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ..Default::default()
        };

        let render_pass_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            attachment_count: 1,
            p_attachments: &color_attachment,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 1,
            p_dependencies: &dependency,
            ..Default::default()
        };
        unsafe { device.create_render_pass(&render_pass_info, None).unwrap() }
    }
    fn create_descriptor_set_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
        let ubo_layout_binding = vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            p_immutable_samplers: ptr::null(),
        };

        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            binding_count: 1,
            p_bindings: &ubo_layout_binding,
            ..Default::default()
        };

        unsafe {
            device
                .create_descriptor_set_layout(&layout_info, None)
                .unwrap()
        }
    }
    fn create_graphics_pipeline(
        device: &ash::Device,
        render_pass: &vk::RenderPass,
        layout: &vk::DescriptorSetLayout,
    ) -> (vk::PipelineLayout, vk::Pipeline) {
        let vert_shader_code = fs::read("shaders/vert.spv").unwrap();
        let frag_shader_code = fs::read("shaders/frag.spv").unwrap();

        let vert_shader_module = VulkanDetails::create_shader_module(device, vert_shader_code);
        let frag_shader_module = VulkanDetails::create_shader_module(device, frag_shader_code);

        let vert_shader_stage_info = vk::PipelineShaderStageCreateInfo {
            s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
            stage: vk::ShaderStageFlags::VERTEX,
            module: vert_shader_module,
            p_name: CStr::from_bytes_with_nul("main\0".as_bytes())
                .unwrap()
                .as_ptr(),
            ..Default::default()
        };

        let frag_shader_stage_info = vk::PipelineShaderStageCreateInfo {
            s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
            stage: vk::ShaderStageFlags::FRAGMENT,
            module: frag_shader_module,
            p_name: CStr::from_bytes_with_nul("main\0".as_bytes())
                .unwrap()
                .as_ptr(),
            ..Default::default()
        };

        let shader_stages = vec![vert_shader_stage_info, frag_shader_stage_info];

        let binding_description = Vertex::get_binding_description();
        let attribute_descriptions = Vertex::get_attribute_descriptions();

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            vertex_binding_description_count: 1,
            p_vertex_binding_descriptions: &binding_description,
            vertex_attribute_description_count: attribute_descriptions.len() as u32,
            p_vertex_attribute_descriptions: attribute_descriptions.as_ptr(),
            ..Default::default()
        };

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };

        let viewport_state = vk::PipelineViewportStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            viewport_count: 1,
            scissor_count: 1,
            ..Default::default()
        };

        let rasterizer = vk::PipelineRasterizationStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            depth_clamp_enable: vk::FALSE,
            rasterizer_discard_enable: vk::FALSE,
            polygon_mode: vk::PolygonMode::FILL,
            line_width: 1.0,
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::CLOCKWISE,
            depth_bias_enable: vk::FALSE,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            ..Default::default()
        };

        let multisampling = vk::PipelineMultisampleStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            sample_shading_enable: vk::FALSE,
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            min_sample_shading: 1.0,
            p_sample_mask: ptr::null(),
            alpha_to_coverage_enable: vk::FALSE,
            alpha_to_one_enable: vk::FALSE,
            ..Default::default()
        };

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            color_write_mask: vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
            blend_enable: vk::FALSE,
            src_color_blend_factor: vk::BlendFactor::ONE,
            dst_color_blend_factor: vk::BlendFactor::ZERO,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
        };

        let color_blending = vk::PipelineColorBlendStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            logic_op_enable: vk::FALSE,
            logic_op: vk::LogicOp::COPY,
            attachment_count: 1,
            p_attachments: &color_blend_attachment,
            blend_constants: [0.0, 0.0, 0.0, 0.0],
            ..Default::default()
        };

        let dynamic_states = vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

        let dynamic_state = vk::PipelineDynamicStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            dynamic_state_count: dynamic_states.len() as u32,
            p_dynamic_states: dynamic_states.as_ptr(),
            ..Default::default()
        };

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo {
            s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
            set_layout_count: 1,
            p_set_layouts: layout,
            push_constant_range_count: 0,
            p_push_constant_ranges: ptr::null(),
            ..Default::default()
        };

        let pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .unwrap()
        };

        let pipeline_info = vk::GraphicsPipelineCreateInfo {
            s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
            stage_count: 2,
            p_stages: shader_stages.as_ptr(),
            p_vertex_input_state: &vertex_input_info,
            p_input_assembly_state: &input_assembly,
            p_viewport_state: &viewport_state,
            p_rasterization_state: &rasterizer,
            p_multisample_state: &multisampling,
            p_depth_stencil_state: ptr::null(),
            p_color_blend_state: &color_blending,
            p_dynamic_state: &dynamic_state,
            layout: pipeline_layout,
            render_pass: *render_pass,
            subpass: 0,
            base_pipeline_handle: vk::Pipeline::null(),
            base_pipeline_index: -1,
            ..Default::default()
        };

        let graphics_pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .unwrap()[0]
        };

        unsafe {
            device.destroy_shader_module(frag_shader_module, None);
            device.destroy_shader_module(vert_shader_module, None);
        }
        (pipeline_layout, graphics_pipeline)
    }
    fn create_shader_module(device: &ash::Device, code: Vec<u8>) -> vk::ShaderModule {
        let create_info = vk::ShaderModuleCreateInfo {
            s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
            code_size: code.len(),
            p_code: code.as_ptr() as *const u32,
            ..Default::default()
        };
        unsafe { device.create_shader_module(&create_info, None).unwrap() }
    }
    fn create_framebuffers(
        device: &ash::Device,
        swap_chain_image_views: &Vec<vk::ImageView>,
        swap_chain_extent: &vk::Extent2D,
        render_pass: &vk::RenderPass,
    ) -> Vec<vk::Framebuffer> {
        let mut framebuffers = Vec::new();

        for image_view in swap_chain_image_views {
            let framebuffer_info = vk::FramebufferCreateInfo {
                s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
                render_pass: *render_pass,
                attachment_count: 1,
                p_attachments: image_view,
                width: swap_chain_extent.width,
                height: swap_chain_extent.height,
                layers: 1,
                ..Default::default()
            };
            framebuffers
                .push(unsafe { device.create_framebuffer(&framebuffer_info, None).unwrap() });
        }
        framebuffers
    }
    fn create_command_pool(
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        device: &ash::Device,
        surface: &vk::SurfaceKHR,
    ) -> vk::CommandPool {
        let (graphics_queue_family_index, _) =
            VulkanDetails::find_queue_familes(entry, instance, physical_device, surface);
        let pool_info = vk::CommandPoolCreateInfo {
            s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index: graphics_queue_family_index.unwrap() as u32,
            ..Default::default()
        };
        unsafe { device.create_command_pool(&pool_info, None).unwrap() }
    }
    fn create_vertex_buffer(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        device: &ash::Device,
        command_pool: &vk::CommandPool,
        graphics_queue: &vk::Queue,
    ) -> (vk::Buffer, vk::DeviceMemory) {
        let buffer_size = (VERTICES.len() * size_of::<Vertex>()) as u64;
        let (staging_buffer, staging_buffer_memory) = VulkanDetails::create_buffer(
            instance,
            physical_device,
            device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        unsafe {
            let data = device
                .map_memory(
                    staging_buffer_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();

            (data as *mut [Vertex; VERTICES.len()]).write(VERTICES);
            device.unmap_memory(staging_buffer_memory);
        }
        let (mut buffer, buffer_memory) = VulkanDetails::create_buffer(
            instance,
            physical_device,
            device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        VulkanDetails::copy_buffer(
            device,
            command_pool,
            graphics_queue,
            &staging_buffer,
            &mut buffer,
            buffer_size,
        );

        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_buffer_memory, None);
        }

        (buffer, buffer_memory)
    }
    fn find_memory_type(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> u32 {
        let mem_properties =
            unsafe { instance.get_physical_device_memory_properties(*physical_device) };
        for i in 0..mem_properties.memory_type_count {
            if (type_filter & (1 << i)) != 0
                && mem_properties.memory_types[i as usize].property_flags & properties == properties
            {
                return i;
            }
        }
        panic!("Unable to find suitable memory type!")
    }
    fn create_buffer(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        device: &ash::Device,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> (vk::Buffer, vk::DeviceMemory) {
        let buffer_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let buffer = unsafe { device.create_buffer(&buffer_info, None).unwrap() };

        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let alloc_info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
            allocation_size: mem_requirements.size,
            memory_type_index: VulkanDetails::find_memory_type(
                instance,
                physical_device,
                mem_requirements.memory_type_bits,
                properties,
            ),
            ..Default::default()
        };

        let buffer_memory = unsafe { device.allocate_memory(&alloc_info, None).unwrap() };

        unsafe {
            device.bind_buffer_memory(buffer, buffer_memory, 0).unwrap();
        }
        (buffer, buffer_memory)
    }
    fn copy_buffer(
        device: &ash::Device,
        command_pool: &vk::CommandPool,
        graphics_queue: &vk::Queue,
        src_buffer: &vk::Buffer,
        dst_buffer: &mut vk::Buffer,
        size: vk::DeviceSize,
    ) {
        let alloc_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            level: vk::CommandBufferLevel::PRIMARY,
            command_pool: *command_pool,
            command_buffer_count: 1,
            ..Default::default()
        };
        let command_buffer = unsafe { device.allocate_command_buffers(&alloc_info).unwrap()[0] };

        let begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };

        let copy_region = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size,
        };

        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            command_buffer_count: 1,
            p_command_buffers: &command_buffer,
            ..Default::default()
        };

        unsafe {
            device
                .begin_command_buffer(command_buffer, &begin_info)
                .unwrap();
            device.cmd_copy_buffer(command_buffer, *src_buffer, *dst_buffer, &[copy_region]);
            device.end_command_buffer(command_buffer).unwrap();
            device
                .queue_submit(*graphics_queue, &[submit_info], vk::Fence::null())
                .unwrap();
            device.queue_wait_idle(*graphics_queue).unwrap();
            device.free_command_buffers(*command_pool, &[command_buffer]);
        }
    }
    fn create_index_buffer(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        device: &ash::Device,
        command_pool: &vk::CommandPool,
        graphics_queue: &vk::Queue,
    ) -> (vk::Buffer, vk::DeviceMemory) {
        let buffer_size = (INDICES.len() * size_of::<u16>()) as u64;
        let (staging_buffer, staging_buffer_memory) = VulkanDetails::create_buffer(
            instance,
            physical_device,
            device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        unsafe {
            let data = device
                .map_memory(
                    staging_buffer_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();

            (data as *mut [u16; INDICES.len()]).write(INDICES);
            device.unmap_memory(staging_buffer_memory);
        }
        let (mut buffer, buffer_memory) = VulkanDetails::create_buffer(
            instance,
            physical_device,
            device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        VulkanDetails::copy_buffer(
            device,
            command_pool,
            graphics_queue,
            &staging_buffer,
            &mut buffer,
            buffer_size,
        );

        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_buffer_memory, None);
        }

        (buffer, buffer_memory)
    }
    fn create_uniform_buffers(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        device: &ash::Device,
    ) -> (Vec<vk::Buffer>, Vec<vk::DeviceMemory>) {
        let buffer_size = std::mem::size_of::<UniformBufferObject>() as vk::DeviceSize;

        let mut uniform_buffers = Vec::new();
        let mut uniform_buffers_memory = Vec::new();

        uniform_buffers.reserve(MAX_FRAMES_IN_FLIGHT);
        uniform_buffers_memory.reserve(MAX_FRAMES_IN_FLIGHT);

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let (uniform_buffer, uniform_buffer_memory) = VulkanDetails::create_buffer(
                instance,
                physical_device,
                device,
                buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            uniform_buffers.push(uniform_buffer);
            uniform_buffers_memory.push(uniform_buffer_memory);
        }
        (uniform_buffers, uniform_buffers_memory)
    }
    fn create_descriptor_pool(device: &ash::Device) -> vk::DescriptorPool {
        let pool_size = vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
        };

        let pool_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            pool_size_count: 1,
            p_pool_sizes: &pool_size,
            max_sets: MAX_FRAMES_IN_FLIGHT as u32,
            ..Default::default()
        };

        unsafe { device.create_descriptor_pool(&pool_info, None).unwrap() }
    }
    fn create_descriptor_sets(
        device: &ash::Device,
        uniform_buffers: &Vec<vk::Buffer>,
        descriptor_set_layout: &vk::DescriptorSetLayout,
        descriptor_pool: &vk::DescriptorPool,
    ) -> Vec<vk::DescriptorSet> {
        let layouts = vec![*descriptor_set_layout; MAX_FRAMES_IN_FLIGHT];
        let alloc_info = vk::DescriptorSetAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
            descriptor_pool: *descriptor_pool,
            descriptor_set_count: MAX_FRAMES_IN_FLIGHT as u32,
            p_set_layouts: layouts.as_ptr(),
            ..Default::default()
        };

        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&alloc_info).unwrap() };

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let buffer_info = vk::DescriptorBufferInfo {
                buffer: uniform_buffers[i],
                offset: 0,
                range: size_of::<UniformBufferObject>() as u64,
            };

            let descriptor_write = vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                dst_set: descriptor_sets[i],
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                p_buffer_info: &buffer_info,
                p_image_info: ptr::null(),
                p_texel_buffer_view: ptr::null(),
                ..Default::default()
            };

            unsafe {
                device.update_descriptor_sets(
                    [descriptor_write].as_ref(),
                    &[] as &[vk::CopyDescriptorSet],
                );
            }
        }
        descriptor_sets
    }
    fn create_command_buffers(
        device: &ash::Device,
        command_pool: &vk::CommandPool,
    ) -> Vec<vk::CommandBuffer> {
        let alloc_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            command_pool: *command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: MAX_FRAMES_IN_FLIGHT as u32,
            ..Default::default()
        };
        unsafe { device.allocate_command_buffers(&alloc_info).unwrap() }
    }
    fn create_sync_objects(
        device: &ash::Device,
    ) -> (Vec<vk::Semaphore>, Vec<vk::Semaphore>, Vec<vk::Fence>) {
        let semaphore_info = vk::SemaphoreCreateInfo {
            s_type: vk::StructureType::SEMAPHORE_CREATE_INFO,
            ..Default::default()
        };
        let fence_info = vk::FenceCreateInfo {
            s_type: vk::StructureType::FENCE_CREATE_INFO,
            flags: vk::FenceCreateFlags::SIGNALED,
            ..Default::default()
        };
        let mut image_available_semaphores = Vec::new();
        let mut render_finished_semaphores = Vec::new();
        let mut in_flight_fences = Vec::new();
        unsafe {
            for _ in 0..MAX_FRAMES_IN_FLIGHT {
                image_available_semaphores
                    .push(device.create_semaphore(&semaphore_info, None).unwrap());
                render_finished_semaphores
                    .push(device.create_semaphore(&semaphore_info, None).unwrap());
                in_flight_fences.push(device.create_fence(&fence_info, None).unwrap());
            }
        }
        (
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
        )
    }
    fn record_command_buffer(&self, image_index: usize) {
        let begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            ..Default::default()
        };
        unsafe {
            self.device
                .begin_command_buffer(self.command_buffers[self.current_frame], &begin_info)
                .unwrap();
        }
        let clear_color = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };
        let render_pass_info = vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
            render_pass: self.render_pass,
            framebuffer: self.swap_chain_framebuffers[image_index],
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swap_chain_extent,
            },
            clear_value_count: 1,
            p_clear_values: &clear_color,
            ..Default::default()
        };
        unsafe {
            self.device.cmd_begin_render_pass(
                self.command_buffers[self.current_frame],
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );
            self.device.cmd_bind_pipeline(
                self.command_buffers[self.current_frame],
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            );
        }
        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: self.swap_chain_extent.width as f32,
            height: self.swap_chain_extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        unsafe {
            self.device
                .cmd_set_viewport(self.command_buffers[self.current_frame], 0, &[viewport]);
        }
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: self.swap_chain_extent,
        };
        let vertex_buffers = [self.vertex_buffer];
        let offsets: [vk::DeviceSize; 1] = [0];
        unsafe {
            self.device
                .cmd_set_scissor(self.command_buffers[self.current_frame], 0, &[scissor]);
            self.device.cmd_bind_vertex_buffers(
                self.command_buffers[self.current_frame],
                0,
                &vertex_buffers,
                &offsets,
            );
            self.device.cmd_bind_index_buffer(
                self.command_buffers[self.current_frame],
                self.index_buffer,
                0,
                vk::IndexType::UINT16,
            );
            self.device.cmd_bind_descriptor_sets(
                self.command_buffers[self.current_frame],
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                [self.descriptor_sets[self.current_frame]].as_ref(),
                &[],
            );
            self.device.cmd_draw_indexed(
                self.command_buffers[self.current_frame],
                INDICES.len() as u32,
                1,
                0,
                0,
                0,
            );
            self.device
                .cmd_end_render_pass(self.command_buffers[self.current_frame]);
            self.device
                .end_command_buffer(self.command_buffers[self.current_frame])
                .unwrap();
        }
    }
    fn update_uniform_buffer(&mut self, current_image: usize) {
        if self.start_time == SystemTime::UNIX_EPOCH {
            self.start_time = SystemTime::now();
        }

        let current_time = SystemTime::now();

        let time = current_time.duration_since(self.start_time).unwrap();

        let mut ubo = UniformBufferObject {
            model: glam::Mat4::from_rotation_z(time.as_secs_f32() * 90f32.to_radians()),
            view: glam::Mat4::look_at_lh(
                glam::vec3(2.0f32, 2.0f32, 2.0f32),
                glam::vec3(0.0f32, 0.0f32, 0.0f32),
                glam::vec3(0.0f32, 0.0f32, 1.0f32),
            ),
            proj: glam::Mat4::perspective_lh(
                45.0f32.to_radians(),
                (self.swap_chain_extent.width as f32) / (self.swap_chain_extent.height as f32),
                0.1f32,
                10.0f32,
            ),
        };

        ubo.proj.y_axis.y *= -1.0f32;

        unsafe {
            let data = self
                .device
                .map_memory(
                    self.uniform_buffers_memory[current_image],
                    0,
                    std::mem::size_of::<UniformBufferObject>() as u64,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();

            (data as *mut UniformBufferObject).write(ubo);
            self.device
                .unmap_memory(self.uniform_buffers_memory[current_image]);
        }
    }
    fn draw_frame(&mut self, window: &winit::window::Window) {
        unsafe {
            self.device
                .wait_for_fences(&[self.in_flight_fences[self.current_frame]], true, u64::MAX)
                .unwrap();
            let swap_chain_handle = Swapchain::new(&self.instance, &self.device);
            let (image_index, _) = match swap_chain_handle.acquire_next_image(
                self.swap_chain,
                u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null(),
            ) {
                Ok(value) => value,
                Err(error) => match error {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => {
                        self.recreate_swap_chain(window);
                        return;
                    }
                    _ => panic!("Problem with the surface!"),
                },
            };
            self.device
                .reset_fences(&[self.in_flight_fences[self.current_frame]])
                .unwrap();
            self.device
                .reset_command_buffer(
                    self.command_buffers[self.current_frame],
                    vk::CommandBufferResetFlags::empty(),
                )
                .unwrap();
            self.record_command_buffer(image_index as usize);
            self.update_uniform_buffer(self.current_frame);
            let submit_info = vk::SubmitInfo {
                s_type: vk::StructureType::SUBMIT_INFO,
                wait_semaphore_count: 1,
                p_wait_semaphores: [self.image_available_semaphores[self.current_frame]].as_ptr(),
                p_wait_dst_stage_mask: [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT].as_ptr(),
                command_buffer_count: 1,
                p_command_buffers: [self.command_buffers[self.current_frame]].as_ptr(),
                signal_semaphore_count: 1,
                p_signal_semaphores: [self.render_finished_semaphores[self.current_frame]].as_ptr(),
                ..Default::default()
            };
            self.device
                .queue_submit(
                    self.graphics_queue,
                    &[submit_info],
                    self.in_flight_fences[self.current_frame],
                )
                .unwrap();
            let present_info = vk::PresentInfoKHR {
                s_type: vk::StructureType::PRESENT_INFO_KHR,
                wait_semaphore_count: 1,
                p_wait_semaphores: [self.render_finished_semaphores[self.current_frame]].as_ptr(),
                swapchain_count: 1,
                p_swapchains: [self.swap_chain].as_ptr(),
                p_image_indices: &image_index,
                ..Default::default()
            };
            match swap_chain_handle.queue_present(self.present_queue, &present_info) {
                Ok(should_recreate) => {
                    if should_recreate || self.framebuffer_resized {
                        self.framebuffer_resized = false;
                        self.recreate_swap_chain(window);
                    }
                }
                Err(error) => match error {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => self.recreate_swap_chain(window),
                    _ => panic!("Unable to present!"),
                },
            };
            self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        }
    }
    fn cleanup_swap_chain(&mut self) {
        unsafe {
            for framebuffer in &self.swap_chain_framebuffers {
                self.device.destroy_framebuffer(*framebuffer, None);
            }
            for image_view in &self.swap_chain_image_views {
                self.device.destroy_image_view(*image_view, None);
            }
            Swapchain::new(&self.instance, &self.device).destroy_swapchain(self.swap_chain, None);
        }
    }
    fn recreate_swap_chain(&mut self, window: &winit::window::Window) {
        unsafe { self.device.device_wait_idle().unwrap() };

        self.cleanup_swap_chain();

        (
            self.swap_chain,
            self.swap_chain_images,
            self.swap_chain_image_format,
            self.swap_chain_extent,
        ) = VulkanDetails::create_swap_chain(
            &window,
            &self.entry,
            &self.instance,
            &self.physical_device,
            &self.device,
            &self.surface,
        );

        self.swap_chain_image_views = VulkanDetails::create_image_views(
            &self.device,
            &self.swap_chain_images,
            &self.swap_chain_image_format,
        );

        self.swap_chain_framebuffers = VulkanDetails::create_framebuffers(
            &self.device,
            &self.swap_chain_image_views,
            &self.swap_chain_extent,
            &self.render_pass,
        );
    }
    fn cleanup(&mut self) {
        unsafe {
            self.cleanup_swap_chain();
            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.device.destroy_buffer(self.uniform_buffers[i], None);
                self.device
                    .free_memory(self.uniform_buffers_memory[i], None);
            }
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.device.destroy_buffer(self.index_buffer, None);
            self.device.free_memory(self.index_buffer_memory, None);
            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);
            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_render_pass(self.render_pass, None);
            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.device
                    .destroy_semaphore(self.image_available_semaphores[i], None);
                self.device
                    .destroy_semaphore(self.render_finished_semaphores[i], None);
                self.device.destroy_fence(self.in_flight_fences[i], None);
            }
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            DebugUtils::new(&self.entry, &self.instance)
                .destroy_debug_utils_messenger(self.debug_messenger, None);
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
            event_loop,
            window,
            vulkan_details,
        }
    }
    pub fn run(mut self) -> ! {
        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == self.window.id() => *control_flow = ControlFlow::Exit,
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    window_id,
                } if window_id == self.window.id() => {
                    self.vulkan_details.framebuffer_resized = true;
                    if size.width > 0 && size.height > 0 {
                        self.vulkan_details.draw_frame(&self.window);
                    }
                }
                Event::LoopDestroyed => {
                    unsafe { self.vulkan_details.device.device_wait_idle().unwrap() };
                    self.vulkan_details.cleanup();
                }
                _ => {
                    if self.window.inner_size().width > 0 && self.window.inner_size().height > 0 {
                        self.vulkan_details.draw_frame(&self.window);
                    }
                }
            }
        });
    }
    fn init_window(
    ) -> Result<(winit::event_loop::EventLoop<()>, winit::window::Window), winit::error::OsError>
    {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_resizable(true)
            .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT))
            .build(&event_loop)?;
        Ok((event_loop, window))
    }
}
