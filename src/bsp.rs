use crate::convert::map_coords;
use crate::error::Error;
use crate::gltf_builder::push_or_get_material;
use bytemuck::{offset_of, Pod, Zeroable};
use gltf_json::accessor::{ComponentType, GenericComponentType, Type};
use gltf_json::buffer::{Stride, Target, View};
use gltf_json::mesh::{Mode, Primitive, Semantic};
use gltf_json::validation::Checked::Valid;
use gltf_json::validation::USize64;
use gltf_json::{Accessor, Index, Mesh, Node, Root, Value};
use std::mem::size_of;
use tf_asset_loader::Loader;
use vbsp::{Bsp, Entity, Face, Handle, Model, Vector};

pub fn bsp_models(bsp: &Bsp) -> Result<Vec<(Handle<Model>, Vector)>, Error> {
    let world_model = bsp
        .models()
        .next()
        .ok_or(Error::Other("No world model".into()))?;

    let mut models: Vec<_> = bsp
        .entities
        .iter()
        .flat_map(|ent| ent.parse())
        .filter_map(|ent| match ent {
            Entity::Brush(ent)
            | Entity::BrushIllusionary(ent)
            | Entity::BrushWall(ent)
            | Entity::BrushWallToggle(ent) => Some(ent),
            _ => None,
        })
        .flat_map(|brush| Some((brush.model[1..].parse::<usize>().ok()?, brush.origin)))
        .flat_map(|(index, origin)| Some((bsp.models().nth(index)?, origin)))
        .collect();
    models.push((
        world_model,
        Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    ));

    Ok(models)
}

fn bounding_box(vertices: impl IntoIterator<Item = Vector>) -> ([f32; 3], [f32; 3]) {
    let mut min = Vector::from([f32::MAX, f32::MAX, f32::MAX]);
    let mut max = Vector::from([f32::MIN, f32::MIN, f32::MIN]);

    for point in vertices {
        min.x = f32::min(min.x, point.x);
        min.y = f32::min(min.y, point.y);
        min.z = f32::min(min.z, point.z);

        max.x = f32::max(max.x, point.x);
        max.y = f32::max(max.y, point.y);
        max.z = f32::max(max.z, point.z);
    }
    (min.into(), max.into())
}

#[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
#[repr(C)]
pub struct BspVertexData {
    position: [f32; 3],
    uv: [f32; 2],
}

pub fn push_bsp_model(
    buffer: &mut Vec<u8>,
    gltf: &mut Root,
    loader: &Loader,
    model: &Handle<Model>,
    offset: Vector,
) -> Node {
    let primitives = model
        .faces()
        .filter(|face| face.is_visible())
        .map(|face| push_bsp_face(buffer, gltf, loader, &face))
        .collect();

    let mesh = Mesh {
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        primitives,
        weights: None,
    };

    let mesh_index = gltf.meshes.len() as u32;
    gltf.meshes.push(mesh);

    Node {
        camera: None,
        children: None,
        extensions: Default::default(),
        extras: Default::default(),
        matrix: None,
        mesh: Some(Index::new(mesh_index)),
        name: Some("bsp".into()),
        rotation: None,
        scale: None,
        translation: Some(map_coords(offset)),
        skin: None,
        weights: None,
    }
}

pub fn push_bsp_face(
    buffer: &mut Vec<u8>,
    gltf: &mut Root,
    loader: &Loader,
    face: &Handle<Face>,
) -> Primitive {
    let vertex_count = face.vertex_positions().count() as u64;

    let buffer_start = buffer.len() as u64;

    let (min, max) = bounding_box(face.vertex_positions());

    let texture = face.texture();
    let vertices = face.vertex_positions().map(move |pos| BspVertexData {
        position: map_coords(pos),
        uv: texture.uv(pos),
    });

    let vertex_data = vertices.flat_map(bytemuck::cast::<_, [u8; size_of::<BspVertexData>()]>);
    buffer.extend(vertex_data);

    let vertex_buffer_view = View {
        buffer: Index::new(0),
        byte_length: USize64(buffer.len() as u64 - buffer_start),
        byte_offset: Some(USize64(buffer_start)),
        byte_stride: Some(Stride(size_of::<BspVertexData>())),
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        target: Some(Valid(Target::ArrayBuffer)),
    };

    let vertex_view = Index::new(gltf.buffer_views.len() as u32);
    gltf.buffer_views.push(vertex_buffer_view);

    let positions = Accessor {
        buffer_view: Some(vertex_view),
        byte_offset: Some(USize64(offset_of!(BspVertexData, position) as u64)),
        count: USize64(vertex_count),
        component_type: Valid(GenericComponentType(ComponentType::F32)),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Valid(Type::Vec3),
        min: Some(Value::from(map_coords(min).to_vec())),
        max: Some(Value::from(map_coords(max).to_vec())),
        name: None,
        normalized: false,
        sparse: None,
    };
    let uvs = Accessor {
        buffer_view: Some(vertex_view),
        byte_offset: Some(USize64(offset_of!(BspVertexData, uv) as u64)),
        count: USize64(vertex_count),
        component_type: Valid(GenericComponentType(ComponentType::F32)),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Valid(Type::Vec2),
        min: None,
        max: None,
        name: None,
        normalized: false,
        sparse: None,
    };

    let accessor_start = gltf.accessors.len() as u32;
    gltf.accessors.push(positions);
    gltf.accessors.push(uvs);

    let material_index = push_or_get_material(buffer, gltf, loader, face.texture().name());

    Primitive {
        attributes: {
            let mut map = std::collections::BTreeMap::new();
            map.insert(Valid(Semantic::Positions), Index::new(accessor_start));
            map.insert(
                Valid(Semantic::TexCoords(0)),
                Index::new(accessor_start + 1),
            );
            map
        },
        extensions: Default::default(),
        extras: Default::default(),
        indices: None,
        material: Some(material_index),
        mode: Valid(Mode::Triangles),
        targets: None,
    }
}
