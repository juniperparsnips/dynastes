//! A state machine thing for animating sprites with bevy.
// #![deny(warnings)]
// #![warn(missing_docs)]

use std::{marker::PhantomData, time::Duration};

use bevy::{
    app::{Plugin, Update},
    asset::{AssetApp, Assets, Handle},
    log::{debug, error, trace},
    prelude::{Commands, Component, Deref, DerefMut, Entity, Query, Res, Without},
    reflect::TypePath,
    sprite::Sprite,
    time::{Time, Timer, TimerMode},
};
use state_machine::{AnimationStateMachine, NextFrame, State};
/// The base logic for switching between animation states
pub mod state_machine;

pub struct DynastesPlugin<State> {
    phantom: PhantomData<State>,
}

impl<S> Default for DynastesPlugin<S> {
    fn default() -> Self {
        DynastesPlugin {
            phantom: PhantomData::default(),
        }
    }
}

impl<S> Plugin for DynastesPlugin<S>
where
    S: State + Send + Sync + TypePath + 'static,
{
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_asset::<AnimationStateMachine<S>>();
        app.add_systems(Update, run_animations::<S>);
        app.add_systems(Update, render_on_load::<S>);
    }
}

#[derive(Component)]
pub struct Dynastes<S>(pub Handle<AnimationStateMachine<S>>)
where
    S: Send + Sync + TypePath;

#[derive(Component)]
struct StateName(String);

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn run_animations<S>(
    time: Res<Time>,
    mut query: Query<(
        &Dynastes<S>,
        &mut StateName,
        &mut AnimationTimer,
        &mut Sprite,
    )>,
    aseprite_assets: Res<Assets<AnimationStateMachine<S>>>,
) where
    S: State + Send + Sync + TypePath,
{
    for (dynastes, mut state_name, mut timer, mut sprite) in &mut query {
        let Some(state_machine) = aseprite_assets.get(&dynastes.0) else {
            error!("Dynastes state machine '{:?}' was not loaded!", dynastes.0);
            continue;
        };

        let Some(state_info) = state_machine.states.get(&state_name.0) else {
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

            match state_info.state.next_frame(atlas) {
                NextFrame::NextState => {
                    if let Some(next_state_name) = &state_info.next_state {
                        trace!("Next state: {next_state_name}");

                        let Some(next_info) = state_machine.states.get(next_state_name) else {
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
                        let start = state_info.first();
                        atlas.index = start;

                        // yes, this is the exact same as below but it's not worth making a function imo
                        let Some(duration) = state_info.duration(start) else {
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
                    let Some(duration) = state_info.duration(index) else {
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

fn render_on_load<S>(
    mut commands: Commands,
    mut unloaded: Query<(Entity, &Dynastes<S>), Without<Sprite>>,
    aseprite_assets: Res<Assets<AnimationStateMachine<S>>>,
) where
    S: State + Send + Sync + TypePath,
{
    for (entity, dynastes) in &mut unloaded {
        let Some(state_machine) = aseprite_assets.get(&dynastes.0) else {
            // Not loaded
            continue;
        };

        let Some(state_info) = state_machine.default_state() else {
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
            Sprite::from_atlas_image(state_machine.image.clone(), state_info.atlas().clone()),
            StateName(state_machine.default_state_name.clone()),
            AnimationTimer(Timer::new(
                Duration::from_millis(first_duration),
                TimerMode::Repeating,
            )),
        ));
    }
}
