use crate::gltf_builder::push_or_get_material;
use crate::{map_coords, Error};
use bytemuck::{offset_of, Pod, Zeroable};
use gltf_json::accessor::{ComponentType, GenericComponentType, Type};
use gltf_json::buffer::{Target, View};
use gltf_json::mesh::{Mode, Primitive, Semantic};
use gltf_json::validation::Checked::Valid;
use gltf_json::{Accessor, Index, Mesh, Root, Value};
use std::mem::size_of;
use tf_asset_loader::Loader;
use vmdl::{Mdl, Model, SkinTable, Vtx, Vvd};

#[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
#[repr(C)]
pub struct ModelVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

impl From<&vmdl::vvd::Vertex> for ModelVertex {
    fn from(vertex: &vmdl::vvd::Vertex) -> Self {
        ModelVertex {
            position: map_coords(vertex.position),
            uv: vertex.texture_coordinates,
            normal: vertex.normal.into(),
        }
    }
}

fn push_vertices(buffer: &mut Vec<u8>, gltf: &mut Root, model: &Model) {
    let start = buffer.len() as u32;
    let view_start = gltf.buffer_views.len() as u32;
    let vertex_count = model.vertices().len() as u32;

    let (min, max) = model.bounding_box();
    let min = map_coords(min);
    let max = map_coords(max);

    let vertex_data = model
        .vertices()
        .iter()
        .map(ModelVertex::from)
        .flat_map(bytemuck::cast::<_, [u8; size_of::<ModelVertex>()]>);
    buffer.extend(vertex_data);

    let vertex_buffer_view = View {
        buffer: Index::new(0),
        byte_length: buffer.len() as u32 - start,
        byte_offset: Some(start),
        byte_stride: Some(size_of::<ModelVertex>() as u32),
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        target: Some(Valid(Target::ArrayBuffer)),
    };

    gltf.buffer_views.push(vertex_buffer_view);

    let positions = Accessor {
        buffer_view: Some(Index::new(view_start)),
        byte_offset: Some(offset_of!(ModelVertex, position) as u32),
        count: vertex_count,
        component_type: Valid(GenericComponentType(ComponentType::F32)),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Valid(Type::Vec3),
        min: Some(Value::from(Vec::from(min))),
        max: Some(Value::from(Vec::from(max))),
        name: None,
        normalized: false,
        sparse: None,
    };
    let uvs = Accessor {
        buffer_view: Some(Index::new(view_start)),
        byte_offset: Some(offset_of!(ModelVertex, uv) as u32),
        count: vertex_count,
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
    let normals = Accessor {
        buffer_view: Some(Index::new(view_start)),
        byte_offset: Some(offset_of!(ModelVertex, normal) as u32),
        count: vertex_count,
        component_type: Valid(GenericComponentType(ComponentType::F32)),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Valid(Type::Vec3),
        min: None,
        max: None,
        name: None,
        normalized: false,
        sparse: None,
    };

    gltf.accessors.extend([positions, uvs, normals]);
}

#[tracing::instrument(skip(loader))]
pub fn load_prop(loader: &Loader, name: &str) -> Result<Model, Error> {
    let load = |name: &str| -> Result<Vec<u8>, Error> {
        loader
            .load(name)?
            .ok_or(Error::ResourceNotFound(name.into()))
    };
    let mdl = Mdl::read(&load(name)?)?;
    let vtx = Vtx::read(&load(&name.replace(".mdl", ".dx90.vtx"))?)?;
    let vvd = Vvd::read(&load(&name.replace(".mdl", ".vvd"))?)?;

    Ok(Model::from_parts(mdl, vtx, vvd))
}

pub fn push_or_get_model(
    buffer: &mut Vec<u8>,
    gltf: &mut Root,
    loader: &Loader,
    model: &str,
    skin: i32,
) -> Index<Mesh> {
    let skinned_name = format!("{model}_{skin}");
    match get_mesh_index(&gltf.meshes, &skinned_name) {
        Some(index) => index,
        None => {
            let prop = load_prop(loader, model).expect("failed to load prop");
            let index = gltf.meshes.len() as u32;
            let material = push_model(buffer, gltf, loader, &prop, skin);
            gltf.meshes.push(material);
            Index::new(index)
        }
    }
}

fn get_mesh_index(meshes: &[Mesh], name: &str) -> Option<Index<Mesh>> {
    meshes
        .iter()
        .enumerate()
        .find_map(|(i, mat)| (mat.name.as_deref() == Some(name)).then_some(i))
        .map(|i| Index::new(i as u32))
}

pub fn push_model(
    buffer: &mut Vec<u8>,
    gltf: &mut Root,
    loader: &Loader,
    model: &Model,
    skin: i32,
) -> Mesh {
    let accessor_start = gltf.accessors.len() as u32;
    push_vertices(buffer, gltf, model);
    let skin_table = model
        .skin_tables()
        .nth(skin as usize)
        .unwrap_or_else(|| model.skin_tables().next().unwrap());

    let primitives = model
        .meshes()
        .map(|mesh| push_primitive(buffer, gltf, loader, &mesh, accessor_start, &skin_table))
        .collect();

    Mesh {
        extensions: Default::default(),
        extras: Default::default(),
        name: Some(format!("{}_{skin}", model.name())),
        primitives,
        weights: None,
    }
}

pub fn push_primitive(
    buffer: &mut Vec<u8>,
    gltf: &mut Root,
    loader: &Loader,
    mesh: &vmdl::Mesh,
    vertex_accessor_start: u32,
    skin: &SkinTable,
) -> Primitive {
    let buffer_start = buffer.len() as u32;
    let view_start = gltf.buffer_views.len() as u32;
    let accessor_start = gltf.accessors.len() as u32;

    buffer.extend(
        mesh.vertex_strip_indices()
            .flatten()
            .flat_map(|index| (index as u32).to_le_bytes()),
    );

    let byte_length = buffer.len() as u32 - buffer_start;

    let view = View {
        buffer: Index::new(0),
        byte_length,
        byte_offset: Some(buffer_start),
        byte_stride: None,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        target: Some(Valid(Target::ElementArrayBuffer)),
    };
    gltf.buffer_views.push(view);

    let accessor = Accessor {
        buffer_view: Some(Index::new(view_start)),
        byte_offset: Some(0),
        count: byte_length / size_of::<u32>() as u32,
        component_type: Valid(GenericComponentType(ComponentType::U32)),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Valid(Type::Scalar),
        min: None,
        max: None,
        name: None,
        normalized: false,
        sparse: None,
    };
    gltf.accessors.push(accessor);

    let texture = skin
        .texture_info(mesh.material_index())
        .expect("mat out of bounds");
    let texture_path = find_material(&texture.name, &texture.search_paths, loader)
        .expect("failed to find texture");
    let material_index = push_or_get_material(buffer, gltf, loader, &texture_path);

    Primitive {
        attributes: {
            let mut map = std::collections::BTreeMap::new();
            map.insert(
                Valid(Semantic::Positions),
                Index::new(vertex_accessor_start),
            );
            map.insert(
                Valid(Semantic::TexCoords(0)),
                Index::new(vertex_accessor_start + 1),
            );
            map.insert(
                Valid(Semantic::Normals),
                Index::new(vertex_accessor_start + 2),
            );
            map
        },
        extensions: Default::default(),
        extras: Default::default(),
        indices: Some(Index::new(accessor_start)),
        material: Some(material_index),
        mode: Valid(Mode::Triangles),
        targets: None,
    }
}

fn find_material(name: &str, paths: &[String], loader: &Loader) -> Option<String> {
    for dir in paths {
        let full_name = format!(
            "{}{}.vmt",
            dir.to_ascii_lowercase().trim_start_matches('/'),
            name.to_ascii_lowercase().trim_end_matches(".vmt")
        );
        let path = format!("materials/{full_name}");
        if loader.exists(&path).unwrap_or_default() {
            return Some(full_name);
        }
    }
    None
}
