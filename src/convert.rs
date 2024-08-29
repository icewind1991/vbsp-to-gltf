use gltf_json as json;

use crate::bsp::{bsp_models, push_bsp_model};
use crate::prop::push_or_get_model;
use crate::{ConvertOptions, Error};
use cgmath::{Deg, Quaternion, Rotation3};
use gltf::Glb;
use gltf_json::scene::UnitQuaternion;
use gltf_json::validation::USize64;
use gltf_json::{Buffer, Index, Node, Root, Scene};
use std::borrow::Cow;
use tf_asset_loader::Loader;
use vbsp::{Bsp, Entity};

pub fn export(bsp: Bsp, loader: &Loader, options: ConvertOptions) -> Result<Glb<'static>, Error> {
    let mut buffer = Vec::new();

    let mut root = Root::default();

    for (model, offset) in bsp_models(&bsp)? {
        let node = push_bsp_model(&mut buffer, &mut root, loader, &model, offset, &options);
        root.nodes.push(node);
    }

    let entity_props =
        bsp.entities
            .iter()
            .flat_map(|ent| ent.parse())
            .filter_map(|ent| match ent {
                Entity::PropDynamic(prop) => Some(prop.as_prop_placement()),
                Entity::PropPhysics(prop) => Some(prop.as_prop_placement()),
                Entity::PropDynamicOverride(prop) => Some(prop.as_prop_placement()),
                _ => None,
            });
    let static_props = bsp.static_props().map(|prop| prop.as_prop_placement());
    for prop in static_props.chain(entity_props) {
        if let Some(mesh) = push_or_get_model(
            &mut buffer,
            &mut root,
            loader,
            prop.model,
            prop.skin,
            &options,
        ) {
            let rotation = prop.rotation;

            let node = Node {
                camera: None,
                children: None,
                extensions: Default::default(),
                extras: Default::default(),
                matrix: None,
                mesh: Some(mesh),
                name: Some(prop.model.into()),
                rotation: Some(UnitQuaternion([
                    rotation.v.x,
                    rotation.v.y,
                    rotation.v.z,
                    rotation.s,
                ])),
                scale: None,
                translation: Some(map_coords(prop.origin)),
                skin: None,
                weights: None,
            };
            root.nodes.push(node);
        }
    }

    let node_indices = 0..root.nodes.len();
    let root_rotation = Quaternion::<f32>::from_angle_y(Deg(90.0));
    let root_node = Node {
        camera: None,
        children: Some(node_indices.map(|index| Index::new(index as u32)).collect()),
        extensions: Default::default(),
        extras: Default::default(),
        matrix: None,
        mesh: None,
        name: None,
        rotation: Some(UnitQuaternion([
            root_rotation.v.x,
            root_rotation.v.y,
            root_rotation.v.z,
            root_rotation.s,
        ])),
        scale: None,
        translation: None,
        skin: None,
        weights: None,
    };
    let root_index = root.nodes.len();
    root.nodes.push(root_node);

    root.scenes = vec![Scene {
        name: None,
        extensions: None,
        extras: Default::default(),
        nodes: vec![Index::new(root_index as u32)],
    }];

    root.buffers.push(Buffer {
        byte_length: USize64(buffer.len() as u64),
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: None,
    });

    let json_string = json::serialize::to_string(&root).expect("Serialization error");
    let mut json_offset = json_string.len() as u32;
    align_to_multiple_of_four(&mut json_offset);

    pad_byte_vector(&mut buffer);
    Ok(Glb {
        header: gltf::binary::Header {
            magic: *b"glTF",
            version: 2,
            length: json_offset + buffer.len() as u32,
        },
        bin: Some(Cow::Owned(buffer)),
        json: Cow::Owned(json_string.into_bytes()),
    })
}

fn align_to_multiple_of_four(n: &mut u32) {
    *n = (*n + 3) & !3;
}

pub fn pad_byte_vector(vec: &mut Vec<u8>) {
    while vec.len() % 4 != 0 {
        vec.push(0); // pad to multiple of four bytes
    }
}

// 1 hammer unit is ~1.905cm
#[allow(dead_code)]
pub const UNIT_SCALE: f32 = 1.0 / (1.905 * 100.0);

pub fn map_coords<C: Into<[f32; 3]>>(vec: C) -> [f32; 3] {
    let vec = vec.into();
    [vec[1], vec[2], vec[0]]
}
