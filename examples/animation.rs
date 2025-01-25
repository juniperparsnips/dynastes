use std::collections::HashMap;

use bevy::prelude::*;
use dynastes::{
    state_machine::{AnimationStateMachine, State, StateInfo},
    Dynastes, DynastesPlugin,
};

fn main() {
    env_logger::init();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DynastesPlugin::<SomeState>::default())
        .add_systems(Startup, setup_animations)
        .run();
}

#[derive(Debug, TypePath)]
struct SomeState {
    first: usize,
    last: usize,
    durations: Vec<u64>,
    atlas: TextureAtlas,
}

impl State for SomeState {
    fn first(&self) -> usize {
        self.first
    }

    fn last(&self) -> usize {
        self.last
    }

    fn duration(&self, index: usize) -> Option<u64> {
        self.durations.get(index - self.first).copied()
    }

    fn atlas(&self) -> &TextureAtlas {
        &self.atlas
    }
}

fn setup_animations(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let image_handle = asset_server.load("sprite-sheet.png");
    let layout = TextureAtlasLayout::from_grid(UVec2 { x: 128, y: 128 }, 26, 2, None, None);
    let layout_handle = asset_server.add(layout);
    let texture_atlas = TextureAtlas {
        layout: layout_handle,
        // NOTE: if the texture atlas's starting index is less than the default state's
        // start index it will panic.
        index: 26,
    };

    let mut states = HashMap::with_capacity(2);
    states.insert(
        "Idle".to_string(),
        StateInfo {
            next_state: Some("Walk".to_string()),
            state: SomeState {
                first: 26,
                last: 51,
                durations: vec![66; 26],
                atlas: texture_atlas.clone(),
            },
        },
    );
    states.insert(
        "Walk".to_string(),
        StateInfo {
            next_state: Some("Idle".to_string()),
            state: SomeState {
                first: 0,
                last: 9,
                durations: vec![66; 10],
                atlas: texture_atlas,
            },
        },
    );

    let state_machine = AnimationStateMachine {
        image: image_handle,
        default_state_name: "Idle".to_string(),
        states,
    };

    let state_machine_handle = asset_server.add(state_machine);

    commands.spawn((
        Dynastes(state_machine_handle),
        Transform::from_scale(Vec3::splat(2.0)),
    ));
}
