//! A state machine thing for animating sprites with bevy.
// #![deny(warnings)]
// #![warn(missing_docs)]

use std::{marker::PhantomData, ops::Deref, time::Duration};

use bevy::{
    app::{Plugin, Update},
    asset::{Asset, AssetApp, Assets, Handle},
    log::{debug, error, trace},
    prelude::{Commands, Component, Deref, DerefMut, Entity, Query, Res, Without},
    reflect::TypePath,
    sprite::Sprite,
    time::{Time, Timer, TimerMode},
};
pub use state_machine::*;

/// The base logic for switching between animation states
mod state_machine;

pub struct DynastesPlugin<T> {
    phantom: PhantomData<T>,
}

// Don't use derive because it requires S, M : Default
impl<T> Default for DynastesPlugin<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData::default(),
        }
    }
}

impl<S, T> Plugin for DynastesPlugin<T>
where
    S: State + Send + Sync + TypePath,
    T: StateSystem<State = S> + Asset,
{
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_asset::<AnimationStateMachine<T>>()
            .init_asset_loader::<DynastesLoader<T>>()
            .add_systems(Update, run_animations::<S, T>)
            .add_systems(Update, render_on_load::<S, T>);
    }
}

#[derive(Component)]
pub struct Dynastes<T>(pub Handle<AnimationStateMachine<T>>)
where
    T: Asset;

#[derive(Component, Deref, DerefMut)]
pub struct StateName(String);

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn run_animations<S, T>(
    time: Res<Time>,
    mut query: Query<(
        &Dynastes<T>,
        &mut StateName,
        &mut AnimationTimer,
        &mut Sprite,
    )>,
    state_machines: Res<Assets<AnimationStateMachine<T>>>,
    state_metadata: Res<Assets<T>>,
) where
    S: State + Send + Sync + TypePath,
    T: StateSystem<State = S> + Asset,
{
    for (dynastes, mut state_name, mut timer, mut sprite) in &mut query {
        let Some(state_machine) = state_machines.get(&dynastes.0) else {
            error!("Dynastes state machine '{:?}' was not loaded!", dynastes.0);
            continue;
        };

        let Some(metadata) = state_metadata.get(&state_machine.states) else {
            debug!("State metadata was not loaded");
            continue;
        };

        let Some(state) = metadata.state(&state_name.0) else {
            error!(
                "Dynastes state machine did not have current state {}",
                state_name.0
            );
            continue;
        };

        timer.tick(time.delta());

        if timer.just_finished() {
            let Some(atlas) = &mut sprite.texture_atlas else {
                debug!("Sprite for dynastes did not have texture atlas");
                continue;
            };

            if timer.times_finished_this_tick() > 1 {
                debug!(
                    "Dynastes missed {} frames",
                    timer.times_finished_this_tick()
                );
            }

            match state.next_frame(atlas) {
                NextFrame::NextState => {
                    let Some(edge) = state_machine.edges.get(&state_name.0) else {
                        error!(
                            "Dynastes state machine did not have edge for state {}",
                            state_name.0
                        );
                        continue;
                    };

                    if let Some(next_state_name) = edge {
                        trace!("Next state: {next_state_name}");

                        let Some(next_info) = metadata.state(next_state_name) else {
                            error!(
                                "Dynastes state machine did not have next state {next_state_name}",
                            );
                            continue;
                        };

                        let first = next_info.first();
                        let Some(first_duration) = next_info.duration(first) else {
                            error!(
                                "Frame {first} does not have duration for state {next_state_name}",
                            );
                            continue;
                        };

                        let mut new_atlas = next_info.atlas().clone();
                        new_atlas.index = first;

                        sprite.texture_atlas = Some(new_atlas);
                        state_name.0 = next_state_name.clone();
                        timer.0 =
                            Timer::new(Duration::from_millis(first_duration), TimerMode::Repeating);
                    } else {
                        trace!("Repeat state: {}", state_name.0);
                        let start = state.first();
                        atlas.index = start;

                        // yes, this is the exact same as below but it's not worth making a function imo
                        let Some(duration) = state.duration(start) else {
                            error!(
                                "Frame {start} does not have duration for state {}",
                                state_name.0
                            );
                            continue;
                        };

                        if timer.times_finished_this_tick() > 1 {
                            debug!(
                                "Dynastes missed {} frames",
                                timer.times_finished_this_tick()
                            );
                        }

                        timer.set_duration(Duration::from_millis(duration));
                    }
                }
                NextFrame::FrameIndex(index) => {
                    atlas.index = index;
                    let Some(duration) = state.duration(index) else {
                        error!(
                            "Frame {index} does not have duration for state {}",
                            state_name.0
                        );
                        continue;
                    };

                    timer.set_duration(Duration::from_millis(duration));
                }
            }
        }
    }
}

fn render_on_load<S, M>(
    mut commands: Commands,
    mut unloaded: Query<(Entity, &Dynastes<M>), Without<Sprite>>,
    state_machines: Res<Assets<AnimationStateMachine<M>>>,
    state_metadata: Res<Assets<M>>,
) where
    S: State + Send + Sync + TypePath,
    M: StateSystem<State = S> + Asset,
{
    for (entity, dynastes) in &mut unloaded {
        let Some(state_machine) = state_machines.get(&dynastes.0) else {
            // Not loaded
            continue;
        };

        let Some(metadata) = state_metadata.get(&state_machine.states) else {
            debug!("State metadata was not loaded");
            continue;
        };

        let Some(state_info) = metadata.state(&state_machine.default_state_name) else {
            error!(
                "Dynastes did not have default state {}",
                state_machine.default_state_name
            );
            continue;
        };

        let first = state_info.first();
        let Some(first_duration) = state_info.duration(first) else {
            println!("No frames in state");
            continue;
        };

        commands.entity(entity).insert((
            Sprite::from_atlas_image(metadata.image().clone(), state_info.atlas().clone()),
            StateName(state_machine.default_state_name.clone()),
            AnimationTimer(Timer::new(
                Duration::from_millis(first_duration),
                TimerMode::Repeating,
            )),
        ));
    }
}
