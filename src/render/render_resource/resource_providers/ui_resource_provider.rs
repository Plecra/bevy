use crate as bevy;
use crate::{
    ecs,
    prelude::Node,
    render::{
        render_graph::RenderGraph,
        render_resource::{
            resource_name, BufferArrayInfo, BufferInfo, BufferUsage, RenderResource,
            ResourceProvider,
        },
        renderer::Renderer,
        shader::AsUniforms,
    },
};
use bevy_derive::Uniforms;
use bevy_transform::prelude::Parent;
use legion::prelude::*;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(Clone, Copy, Debug, AsBytes, FromBytes, Uniforms)]
pub struct Rect {
    #[uniform(instance)]
    pub position: [f32; 2],
    #[uniform(instance)]
    pub size: [f32; 2],
    #[uniform(instance)]
    pub color: [f32; 4],
    #[uniform(instance)]
    pub z_index: f32,
}

pub struct UiResourceProvider {
    pub instance_buffer: Option<RenderResource>,
}

impl UiResourceProvider {
    pub fn new() -> Self {
        UiResourceProvider {
            instance_buffer: None,
        }
    }

    pub fn update(&mut self, renderer: &mut dyn Renderer, world: &World) {
        let node_query = <Read<Node>>::query().filter(!component::<Parent>());

        let mut data = Vec::new();
        if node_query.iter(world).count() > 0 {
            // TODO: this probably isn't the best way to handle z-ordering
            let mut z = 0.9999;
            {
                let mut add_data: Box<dyn FnMut(&World, Entity, ()) -> Option<()>> =
                    Box::new(|world, entity, _| {
                        let node = world.get_component::<Node>(entity).unwrap();
                        data.push(Rect {
                            position: node.global_position.into(),
                            size: node.size.into(),
                            color: node.color.into(),
                            z_index: z,
                        });

                        z -= 0.0001;
                        Some(())
                    });

                for entity in node_query
                    .iter_entities(world)
                    .map(|(entity, _)| entity)
                    .collect::<Vec<Entity>>()
                {
                    ecs::run_on_hierarchy(world, entity, (), &mut add_data);
                }
            }
        }

        if data.len() == 0 {
            return;
        }

        let size = std::mem::size_of::<Rect>() as u64;
        let data_len = data.len() as u64;

        if let Some(old_instance_buffer) = self.instance_buffer {
            renderer.remove_buffer(old_instance_buffer);
        }

        let buffer = renderer.create_buffer_with_data(
            BufferInfo {
                size,
                buffer_usage: BufferUsage::COPY_SRC | BufferUsage::VERTEX,
                array_info: Some(BufferArrayInfo {
                    item_capacity: data_len,
                    item_count: data_len,
                    item_size: size,
                }),
                ..Default::default()
            },
            data.as_bytes(),
        );

        renderer
            .get_render_resources_mut()
            .set_named_resource(resource_name::buffer::UI_INSTANCES, buffer);
        self.instance_buffer = Some(buffer);
    }
}

impl ResourceProvider for UiResourceProvider {
    fn initialize(
        &mut self,
        _renderer: &mut dyn Renderer,
        _world: &mut World,
        resources: &Resources,
    ) {
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.set_vertex_buffer_descriptor(Rect::get_vertex_buffer_descriptor().cloned().unwrap());
    }

    fn update(&mut self, renderer: &mut dyn Renderer, world: &mut World, _resources: &Resources) {
        self.update(renderer, world);
    }
}