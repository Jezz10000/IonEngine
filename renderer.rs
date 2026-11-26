use ash::{
    ext::debug_utils,
    khr::{surface, swapchain},
    vk, Entry, Instance,
};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::{error::Error, ffi::CString, mem, os::raw::c_void};
use winit::window::Window;

pub struct VulkanRenderer {
    entry: Entry,
    instance: Instance,
    surface_loader: surface::Instance,
    surface: vk::SurfaceKHR,
    device: ash::Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    swapchain_loader: swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,
    in_flight: vk::Fence,
    format: vk::Format,
    extent: vk::Extent2D,
}

impl VulkanRenderer {
    pub unsafe fn new(window: &Window) -> Result<Self, Box<dyn Error>> {
        let entry = Entry::load()?;

        let app_name = CString::new("SCGE")?;
        let engine_name = CString::new("SCGE")?;

        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::make_api_version(0, 1, 3, 0));

        let display = window.display_handle()?.as_raw();
        let extension_names = ash_window::enumerate_required_extensions(display)?;

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(extension_names);

        let instance = entry.create_instance(&create_info, None)?;

        let window_handle = window.window_handle()?.as_raw();
        let surface = ash_window::create_surface(
            &entry,
            &instance,
            display,
            window_handle,
            None,
        )?;

        let surface_loader = surface::Instance::new(&entry, &instance);

        let physical_devices = instance.enumerate_physical_devices()?;
        let (physical_device, graphics_family, present_family) =
            pick_physical_device(&instance, &surface_loader, surface, &physical_devices)?;

        let priorities = [1.0_f32];
        let mut families = vec![graphics_family];
        if present_family != graphics_family {
            families.push(present_family);
        }

        let queue_infos: Vec<_> = families.iter().map(|&family| {
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(family)
                .queue_priorities(&priorities)
        }).collect();

        let device_extensions = [swapchain::NAME.as_ptr()];
        let device_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&device_extensions);

        let device = instance.create_device(physical_device, &device_info, None)?;
        let graphics_queue = device.get_device_queue(graphics_family, 0);
        let present_queue = device.get_device_queue(present_family, 0);

        let swapchain_loader = swapchain::Device::new(&instance, &device);
        let mut renderer = Self {
            entry,
            instance,
            surface_loader,
            surface,
            device,
            graphics_queue,
            present_queue,
            swapchain_loader,
            swapchain: vk::SwapchainKHR::null(),
            swapchain_images: Vec::new(),
            swapchain_image_views: Vec::new(),
            render_pass: vk::RenderPass::null(),
            framebuffers: Vec::new(),
            command_pool: vk::CommandPool::null(),
            command_buffers: Vec::new(),
            image_available: vk::Semaphore::null(),
            render_finished: vk::Semaphore::null(),
            in_flight: vk::Fence::null(),
            format: vk::Format::UNDEFINED,
            extent: vk::Extent2D { width: 0, height: 0 },
        };

        let size = window.inner_size();
        renderer.create_swapchain(physical_device, graphics_family, present_family, size.width, size.height)?;
        renderer.create_sync_objects()?;

        Ok(renderer)
    }

    unsafe fn create_swapchain(
        &mut self,
        physical_device: vk::PhysicalDevice,
        graphics_family: u32,
        present_family: u32,
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn Error>> {
        let support = query_swapchain_support(
            physical_device,
            &self.surface_loader,
            self.surface,
        )?;

        let format = choose_format(&support.formats);
        let present_mode = choose_present_mode(&support.present_modes);
        let extent = choose_extent(&support.capabilities, width, height);

        let mut image_count = support.capabilities.min_image_count + 1;
        if support.capabilities.max_image_count > 0 {
            image_count = image_count.min(support.capabilities.max_image_count);
        }

        let families = [graphics_family, present_family];
        let (sharing_mode, queue_family_indices) = if graphics_family != present_family {
            (vk::SharingMode::CONCURRENT, families.to_vec())
        } else {
            (vk::SharingMode::EXCLUSIVE, Vec::new())
        };

        let info = vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(sharing_mode)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(support.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        let swapchain = self.swapchain_loader.create_swapchain(&info, None)?;
        let images = self.swapchain_loader.get_swapchain_images(swapchain)?;

        self.swapchain = swapchain;
        self.swapchain_images = images;
        self.format = format.format;
        self.extent = extent;

        self.swapchain_image_views = self.swapchain_images.iter().map(|&image| {
            let view_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(self.format)
                .components(vk::ComponentMapping::default())
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                );
            self.device.create_image_view(&view_info, None)
        }).collect::<Result<_, _>>()?;

        let color_attachment = vk::AttachmentDescription::default()
            .format(self.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let color_refs = [color_ref];
        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_refs);

        let attachments = [color_attachment];
        let subpasses = [subpass];
        let render_pass_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses);

        self.render_pass = self.device.create_render_pass(&render_pass_info, None)?;

        self.framebuffers = self.swapchain_image_views.iter().map(|&view| {
            let views = [view];
            let info = vk::FramebufferCreateInfo::default()
                .render_pass(self.render_pass)
                .attachments(&views)
                .width(self.extent.width)
                .height(self.extent.height)
                .layers(1);
            self.device.create_framebuffer(&info, None)
        }).collect::<Result<_, _>>()?;

        let pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(graphics_family);

        self.command_pool = self.device.create_command_pool(&pool_info, None)?;

        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(self.framebuffers.len() as u32);

        self.command_buffers = self.device.allocate_command_buffers(&alloc_info)?;

        Ok(())
    }

    unsafe fn create_sync_objects(&mut self) -> Result<(), Box<dyn Error>> {
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        self.image_available = self.device.create_semaphore(&semaphore_info, None)?;
        self.render_finished = self.device.create_semaphore(&semaphore_info, None)?;

        let fence_info = vk::FenceCreateInfo::default()
            .flags(vk::FenceCreateFlags::SIGNALED);
        self.in_flight = self.device.create_fence(&fence_info, None)?;

        Ok(())
    }

    pub unsafe fn draw_frame(&mut self) -> Result<(), Box<dyn Error>> {
        self.device.wait_for_fences(&[self.in_flight], true, u64::MAX)?;

        let (image_index, _) = self.swapchain_loader.acquire_next_image(
            self.swapchain,
            u64::MAX,
            self.image_available,
            vk::Fence::null(),
        )?;

        self.device.reset_fences(&[self.in_flight])?;

        let command_buffer = self.command_buffers[image_index as usize];
        self.device.reset_command_buffer(
            command_buffer,
            vk::CommandBufferResetFlags::empty(),
        )?;

        let begin = vk::CommandBufferBeginInfo::default();
        self.device.begin_command_buffer(command_buffer, &begin)?;

        let clear = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.025, 0.035, 0.05, 1.0],
            },
        }];

        let render_info = vk::RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .framebuffer(self.framebuffers[image_index as usize])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.extent,
            })
            .clear_values(&clear);

        self.device.cmd_begin_render_pass(
            command_buffer,
            &render_info,
            vk::SubpassContents::INLINE,
        );
        self.device.cmd_end_render_pass(command_buffer);
        self.device.end_command_buffer(command_buffer)?;

        let wait_semaphores = [self.image_available];
        let signal_semaphores = [self.render_finished];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

        let submit = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&[command_buffer])
            .signal_semaphores(&signal_semaphores);

        self.device.queue_submit(
            self.graphics_queue,
            &[submit],
            self.in_flight,
        )?;

        let swapchains = [self.swapchain];
        let image_indices = [image_index];
        let present = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        match self.swapchain_loader.queue_present(self.present_queue, &present) {
            Ok(_) => {}
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) |
            Err(vk::Result::SUBOPTIMAL_KHR) => {}
            Err(e) => return Err(e.into()),
        }

        Ok(())
    }

    pub unsafe fn resize(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn Error>> {
        if width == 0 || height == 0 {
            return Ok(());
        }

        self.device.device_wait_idle()?;

        for &fb in &self.framebuffers {
            self.device.destroy_framebuffer(fb, None);
        }
        self.framebuffers.clear();

        for &view in &self.swapchain_image_views {
            self.device.destroy_image_view(view, None);
        }
        self.swapchain_image_views.clear();

        if self.render_pass != vk::RenderPass::null() {
            self.device.destroy_render_pass(self.render_pass, None);
            self.render_pass = vk::RenderPass::null();
        }

        if self.command_pool != vk::CommandPool::null() {
            self.device.destroy_command_pool(self.command_pool, None);
            self.command_pool = vk::CommandPool::null();
        }

        self.swapchain_loader.destroy_swapchain(self.swapchain, None);

        let physical_devices = self.instance.enumerate_physical_devices()?;
        let (physical_device, graphics_family, present_family) =
            pick_physical_device(
                &self.instance,
                &self.surface_loader,
                self.surface,
                &physical_devices,
            )?;

        self.create_swapchain(
            physical_device,
            graphics_family,
            present_family,
            width,
            height,
        )?;

        Ok(())
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();

            self.device.destroy_fence(self.in_flight, None);
            self.device.destroy_semaphore(self.render_finished, None);
            self.device.destroy_semaphore(self.image_available, None);

            if self.command_pool != vk::CommandPool::null() {
                self.device.destroy_command_pool(self.command_pool, None);
            }

            for &fb in &self.framebuffers {
                self.device.destroy_framebuffer(fb, None);
            }

            if self.render_pass != vk::RenderPass::null() {
                self.device.destroy_render_pass(self.render_pass, None);
            }

            for &view in &self.swapchain_image_views {
                self.device.destroy_image_view(view, None);
            }

            if self.swapchain != vk::SwapchainKHR::null() {
                self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            }

            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}

unsafe fn pick_physical_device(
    instance: &Instance,
    surface_loader: &surface::Instance,
    surface: vk::SurfaceKHR,
    devices: &[vk::PhysicalDevice],
) -> Result<(vk::PhysicalDevice, u32, u32), Box<dyn Error>> {
    for &device in devices {
        let props = instance.get_physical_device_properties(device);
        let queue_families = instance.get_physical_device_queue_family_properties(device);

        let mut graphics = None;
        let mut present = None;

        for (index, family) in queue_families.iter().enumerate() {
            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                graphics = Some(index as u32);
            }

            if surface_loader.get_physical_device_surface_support(
                device,
                index as u32,
                surface,
            )? {
                present = Some(index as u32);
            }
        }

        let extension_props =
            instance.enumerate_device_extension_properties(device)?;

        let has_swapchain = extension_props.iter().any(|ext| {
            let name = std::ffi::CStr::from_ptr(ext.extension_name.as_ptr());
            name == swapchain::NAME
        });

        if graphics.is_some() && present.is_some() && has_swapchain {
            let _ = props;
            return Ok((device, graphics.unwrap(), present.unwrap()));
        }
    }

    Err("no suitable Vulkan physical device found".into())
}

struct SwapchainSupport {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

unsafe fn query_swapchain_support(
    device: vk::PhysicalDevice,
    loader: &surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<SwapchainSupport, Box<dyn Error>> {
    Ok(SwapchainSupport {
        capabilities: loader.get_physical_device_surface_capabilities(device, surface)?,
        formats: loader.get_physical_device_surface_formats(device, surface)?,
        present_modes: loader.get_physical_device_surface_present_modes(device, surface)?,
    })
}

fn choose_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    formats.iter()
        .copied()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB &&
            f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or(formats[0])
}

fn choose_present_mode(modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    modes.iter()
        .copied()
        .find(|&m| m == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO)
}

fn choose_extent(
    capabilities: &vk::SurfaceCapabilitiesKHR,
    width: u32,
    height: u32,
) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        vk::Extent2D {
            width: width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    }
}
