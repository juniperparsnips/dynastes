//! A state machine thing for animating sprites with bevy.
// #![deny(warnings)]
// #![warn(missing_docs)]

use std::{marker::PhantomData, time::Duration};

use bevy::{
    app::{Plugin, Update},
    asset::{Asset, AssetApp, Assets, Handle},
    log::{debug, error},
    prelude::{Commands, Component, Entity, Query, Res, Without},
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

fn run_animations<S, T>(
    time: Res<Time>,
    mut query: Query<(
        &Dynastes<T>,
        &mut StateName,
        &mut StateControl,
        &mut AnimationTimer,
        &mut Sprite,
    )>,
    state_machines: Res<Assets<AnimationStateMachine<T>>>,
    state_systems: Res<Assets<T>>,
) where
    S: State + Send + Sync + TypePath,
    T: StateSystem<State = S> + Asset,
{
    let _: Vec<()> = query
        .iter_mut()
        .filter_map(
            |(dynastes, mut state_name, mut state_control, mut timer, mut sprite)| {
                let state_machine = state_machines.get(&dynastes.0).or_else(|| {
                    error!("Dynastes state machine '{:?}' is not loaded!", dynastes.0);
                    None
                })?;

                let state_system = state_systems.get(&state_machine.states).or_else(|| {
                    debug!("State system is not loaded");
                    None
                })?;

                timer.tick(time.delta());

                if !timer.just_finished() {
                    return None;
                }

                if timer.times_finished_this_tick() > 1 {
                    debug!(
                        "Dynastes missed {} frames",
                        timer.times_finished_this_tick()
                    );
                }

                state_system.set_next_frame(
                    &mut state_name,
                    &mut state_control,
                    &state_machine.edges,
                    &mut sprite,
                    &mut timer,
                )
            },
        )
        .collect();
}

// fn run_animation<S, T>() {}

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
            StateControl { new_state: None },
            AnimationTimer(Timer::new(
                Duration::from_millis(first_duration),
                TimerMode::Repeating,
            )),
        ));
    }
}
