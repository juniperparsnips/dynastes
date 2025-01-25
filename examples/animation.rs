use std::collections::HashMap;

use bevy::prelude::*;
use dynastes::{AnimationStateMachine, Dynastes, DynastesPlugin};

use crate::helpers::*;

mod helpers;

fn main() {
    env_logger::init();

    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(DynastesPlugin::<SomeStateSystem>::default())
        .init_asset::<SomeStateSystem>()
        .add_systems(Startup, setup_animations)
        .run();
}

fn setup_animations(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let image = asset_server.load("sprite-sheet.png");

    let layout = TextureAtlasLayout::from_grid(UVec2 { x: 128, y: 128 }, 26, 2, None, None);
    let layout_handle = asset_server.add(layout);
    let texture_atlas = TextureAtlas {
        layout: layout_handle,
        // NOTE: if the texture atlas's starting index is less than the default state's
        // start index it will panic.
        index: 26,
    };

    let mut states = HashMap::new();
    states.insert(
        "Idle".to_string(),
        SomeState {
            name: "Idle".to_string(),
            first: 26,
            last: 51,
            durations: vec![66; 26],
            atlas: texture_atlas.clone(),
        },
    );
    states.insert(
        "Walk".to_string(),
        SomeState {
            name: "Walk".to_string(),
            first: 0,
            last: 9,
            durations: vec![66; 10],
            atlas: texture_atlas,
        },
    );

    let state_system = SomeStateSystem { image, states };
    let system_handle = asset_server.add(state_system);

    let mut edges = HashMap::new();
    edges.insert("Idle".to_string(), Some("Walk".to_string()));
    edges.insert("Walk".to_string(), Some("Idle".to_string()));

    let state_machine = AnimationStateMachine {
        default_state_name: "Idle".to_string(),
        edges,
        states: system_handle,
    };

    let state_machine_handle = asset_server.add(state_machine);

    commands.spawn((
        Dynastes(state_machine_handle),
        Transform::from_scale(Vec3::splat(2.0)),
    ));
}
