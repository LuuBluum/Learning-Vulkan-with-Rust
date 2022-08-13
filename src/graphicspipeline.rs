use ash::extensions::{
    ext::DebugUtils,
    khr::Surface,
    khr::Swapchain,
    khr::{AndroidSurface, WaylandSurface, Win32Surface, XcbSurface, XlibSurface},
};
use ash::prelude::*;
use ash::{vk, Entry};
use raw_window_handle::HasRawWindowHandle;
use std::collections::HashSet;
use std::ffi::{c_void, CStr};
use std::fs;
use std::ptr;
use std::vec::Vec;
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

pub struct VulkanDetails {
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
    pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,
}

pub struct HelloTriangleApplication {
    event_loop: winit::event_loop::EventLoop<()>,
    window: winit::window::Window,
    vulkan_details: VulkanDetails,
}

impl VulkanDetails {
    pub fn new(window: &winit::window::Window) -> Self {
        let (
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
            pipeline_layout,
            graphics_pipeline,
        ) = VulkanDetails::init_vulkan(&window);
        Self {
            entry: entry,
            instance: instance,
            debug_messenger: debug_messenger,
            surface: surface,
            physical_device: physical_device,
            device: device,
            graphics_queue: graphics_queue,
            present_queue: present_queue,
            swap_chain: swap_chain,
            swap_chain_images: swap_chain_images,
            swap_chain_image_format: swap_chain_image_format,
            swap_chain_extent: swap_chain_extent,
            swap_chain_image_views: swap_chain_image_views,
            render_pass: render_pass,
            pipeline_layout: pipeline_layout,
            graphics_pipeline: graphics_pipeline,
        }
    }
    fn init_vulkan(
        window: &winit::window::Window,
    ) -> (
        ash::Entry,
        ash::Instance,
        vk::DebugUtilsMessengerEXT,
        vk::SurfaceKHR,
        vk::PhysicalDevice,
        ash::Device,
        vk::Queue,
        vk::Queue,
        vk::SwapchainKHR,
        Vec<vk::Image>,
        vk::Format,
        vk::Extent2D,
        Vec<vk::ImageView>,
        vk::RenderPass,
        vk::PipelineLayout,
        vk::Pipeline,
    ) {
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
        let image_views = VulkanDetails::create_image_views(
            &device,
            &swap_chain_images,
            &swap_chain_image_format,
        );
        let render_pass = VulkanDetails::create_render_pass(&device, &swap_chain_image_format);
        let (pipeline_layout, graphics_pipeline) =
            VulkanDetails::create_graphics_pipeline(&device, &render_pass);
        (
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
            image_views,
            render_pass,
            pipeline_layout,
            graphics_pipeline,
        )
    }
    fn create_instance(entry: &ash::Entry) -> VkResult<ash::Instance> {
        if !VulkanDetails::check_validation_layer_support(&entry) {
            return Err(vk::Result::ERROR_INITIALIZATION_FAILED);
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
            present_mode: present_mode,
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

        let render_pass_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            attachment_count: 1,
            p_attachments: &color_attachment,
            subpass_count: 1,
            p_subpasses: &subpass,
            ..Default::default()
        };
        unsafe { device.create_render_pass(&render_pass_info, None).unwrap() }
    }
    fn create_graphics_pipeline(
        device: &ash::Device,
        render_pass: &vk::RenderPass,
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

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            vertex_binding_description_count: 0,
            p_vertex_binding_descriptions: ptr::null(),
            vertex_attribute_description_count: 0,
            p_vertex_attribute_descriptions: ptr::null(),
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
            set_layout_count: 0,
            p_set_layouts: ptr::null(),
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
    fn cleanup(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_render_pass(self.render_pass, None);
            for image_view in &self.swap_chain_image_views {
                self.device.destroy_image_view(*image_view, None);
            }
            Swapchain::new(&self.instance, &self.device).destroy_swapchain(self.swap_chain, None);
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
}
