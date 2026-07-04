// pipeline.rs
//

use ash::vk;
use std::io::Cursor;

use crate::error::{RendererResult, VkError, check};

/// Everything needed to build one graphics pipeline. GROWTH POINTS (add as
/// fields, never as branches in create): depth_attachment_format, vertex
/// bindings/attributes, push-constant ranges, blend state.
pub(crate) struct PipelineDesc<'a> {
    pub vertex_spv: &'a [u8],
    pub fragment_spv: &'a [u8],
    pub color_format: vk::Format,
    pub topology: vk::PrimitiveTopology,
    pub polygon_mode: vk::PolygonMode,
    pub cull_mode: vk::CullModeFlags,
    pub front_face: vk::FrontFace,
    pub vertex_bindings: &'a [vk::VertexInputBindingDescription],
    pub vertex_attributes: &'a [vk::VertexInputAttributeDescription],
    pub push_constant_ranges: &'a [vk::PushConstantRange],
    pub depth_format: Option<vk::Format>, // Some = depth test/write on, reverse-Z
}

pub(crate) struct GraphicsPipeline {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
}

impl GraphicsPipeline {
    pub(crate) fn create(device: &ash::Device, desc: &PipelineDesc) -> RendererResult<Self> {
        let vert = create_shader_module(device, desc.vertex_spv)?;
        let frag = create_shader_module(device, desc.fragment_spv)?;

        let entry = c"main";
        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert)
                .name(entry),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag)
                .name(entry),
        ];

        // No vertex buffers yet — positions are baked in the shader.
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(desc.vertex_bindings)
            .vertex_attribute_descriptions(desc.vertex_attributes);

        let input_assembly =
            vk::PipelineInputAssemblyStateCreateInfo::default().topology(desc.topology);

        // Counts fixed here; actual viewport/scissor supplied per-frame via cmd.
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let raster = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(desc.polygon_mode)
            .cull_mode(desc.cull_mode)
            .front_face(desc.front_face)
            .line_width(1.0);

        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false);
        let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(std::slice::from_ref(&blend_attachment));

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let layout_info =
            vk::PipelineLayoutCreateInfo::default().push_constant_ranges(desc.push_constant_ranges);
        let layout = check!(
            unsafe { device.create_pipeline_layout(&layout_info, None) },
            "create pipeline layout"
        )?;

        let depth_stencil = desc.depth_format.map(|_| {
            vk::PipelineDepthStencilStateCreateInfo::default()
                .depth_test_enable(true)
                .depth_write_enable(true)
                .depth_compare_op(vk::CompareOp::GREATER) // reverse-Z: nearer = larger depth
                .min_depth_bounds(0.0)
                .max_depth_bounds(1.0)
        });

        let color_formats = [desc.color_format];
        let mut rendering =
            vk::PipelineRenderingCreateInfo::default().color_attachment_formats(&color_formats);
        if let Some(df) = desc.depth_format {
            rendering = rendering.depth_attachment_format(df);
        }

        let mut info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&raster)
            .multisample_state(&multisample)
            .color_blend_state(&color_blend)
            .dynamic_state(&dynamic_state)
            .layout(layout)
            .push_next(&mut rendering);

        if let Some(ref ds) = depth_stencil {
            info = info.depth_stencil_state(ds);
        }

        let result = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                std::slice::from_ref(&info),
                None,
            )
        };

        // Modules are consumed by pipeline creation — free them regardless of
        // outcome, before the `?` early-returns.
        unsafe {
            device.destroy_shader_module(vert, None);
            device.destroy_shader_module(frag, None);
        }

        let pipeline = result.map_err(|(_, r)| VkError::new("create graphics pipeline", r))?[0];

        Ok(Self { pipeline, layout })
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        device.destroy_pipeline(self.pipeline, None);
        device.destroy_pipeline_layout(self.layout, None);
    }
}

fn create_shader_module(device: &ash::Device, spv: &[u8]) -> RendererResult<vk::ShaderModule> {
    // include_bytes! gives an unaligned &[u8]; read_spv returns a u32-aligned
    // Vec<u32> and validates the length. Skipping this is the classic
    // crash/black-screen trap.
    let code = ash::util::read_spv(&mut Cursor::new(spv))
        .map_err(|_| crate::error::RendererError::Config("invalid SPIR-V"))?;
    let info = vk::ShaderModuleCreateInfo::default().code(&code);
    Ok(check!(
        unsafe { device.create_shader_module(&info, None) },
        "create shader module"
    )?)
}
