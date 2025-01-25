use std::{collections::HashMap, marker::PhantomData, time::Duration};

use bevy::{
    asset::{Asset, AssetLoader, AsyncReadExt, Handle, LoadContext},
    image::Image,
    log::{error, trace},
    prelude::{Component, Deref, DerefMut},
    reflect::{Reflect, TypePath},
    sprite::{Sprite, TextureAtlas},
    time::{Timer, TimerMode},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Component, Deref, DerefMut)]
pub struct StateName(pub String);

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

#[derive(Debug, Component, Asset, Reflect)]
/// A finite state machine across animation states
pub struct AnimationStateMachine<T>
where
    T: Asset,
{
    pub default_state_name: String,
    pub edges: HashMap<String, Option<String>>,
    pub states: Handle<T>,
}

impl<S, T> AnimationStateMachine<T>
where
    S: State + Send + Sync + TypePath,
    T: StateSystem<State = S> + Asset,
{
    fn new(
        dynastes_serde: AnimationStateMachineSerde,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self, DynastesError> {
        let states = load_context.load::<T>(dynastes_serde.states_path);

        Ok(Self {
            default_state_name: dynastes_serde.default_state_name,
            edges: dynastes_serde.edges,
            states,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
/// A finite state machine across animation states
pub struct AnimationStateMachineSerde {
    pub default_state_name: String,
    pub states_path: String,
    pub edges: HashMap<String, Option<String>>,
}

pub struct DynastesLoader<T> {
    phantom: PhantomData<T>,
}

impl<T> Default for DynastesLoader<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData::default(),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum DynastesLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    /// A [Serde-JSON](serde_json) Error
    #[error("Could not parse RON: {0}")]
    Ron(#[from] ron::error::SpannedError),
    #[error("Error building state machine: {0}")]
    Dynastes(#[from] DynastesError),
}

#[derive(Debug, Error)]
pub enum DynastesError {}

impl<S, T> AssetLoader for DynastesLoader<T>
where
    S: State + Send + Sync + TypePath,
    T: StateSystem<State = S> + Asset,
{
    type Asset = AnimationStateMachine<T>;
    type Error = DynastesLoaderError;
    type Settings = ();

    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut asset_str = String::new();
        reader.read_to_string(&mut asset_str).await?;

        let dynastes_serde: AnimationStateMachineSerde = ron::from_str(&asset_str)?;

        Ok(AnimationStateMachine::new(dynastes_serde, load_context)?)
    }

    fn extensions(&self) -> &[&str] {
        &["dyn", "dynastes"]
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateInfo<S> {
    pub next_state: Option<String>,
    pub state: S,
    // TODO: "fluidity" -> i.e. decrease the frame rate but go more frames
}

impl<S> StateInfo<S>
where
    S: State,
{
    pub fn first(&self) -> usize {
        self.state.first()
    }

    pub fn last(&self) -> usize {
        self.state.last()
    }

    pub fn duration(&self, index: usize) -> Option<u64> {
        self.state.duration(index)
    }

    pub fn atlas(&self) -> &TextureAtlas {
        self.state.atlas()
    }
}

#[derive(Debug, Clone, Component)]
pub struct StateControl {
    pub new_state: Option<SetState>,
}

#[derive(Debug, Clone)]
pub enum SetState {
    /// Switch to this state on the next frame
    Immediate(String),
    /// Switch to this state when the current state is done
    OnTransition(String),
}

#[derive(Debug, Clone, Copy)]
pub enum NextFrame {
    NextState,
    FrameIndex(usize),
}

pub trait State {
    fn name(&self) -> &str;

    fn first(&self) -> usize;

    fn last(&self) -> usize;

    fn duration(&self, index: usize) -> Option<u64>;

    fn atlas(&self) -> &TextureAtlas;

    fn next_frame(&self, atlas: &mut TextureAtlas) -> NextFrame {
        if atlas.index == self.last() {
            return NextFrame::NextState;
        } else {
            return NextFrame::FrameIndex(atlas.index + 1);
        }
    }

    fn set_frame(
        &self,
        index: usize,
        atlas: &mut TextureAtlas,
        timer: &mut AnimationTimer,
    ) -> Option<()> {
        atlas.index = index;
        let duration = self.duration(index).or_else(|| {
            error!(
                "Frame {index} does not have duration for state {}",
                self.name()
            );
            None
        })?;

        timer.set_duration(Duration::from_millis(duration));
        Some(())
    }
}

pub trait StateSystem {
    type State: State;

    fn image(&self) -> &Handle<Image>;

    fn states(&self) -> &HashMap<String, Self::State>;

    fn state(&self, state_name: &str) -> Option<&Self::State> {
        self.states().get(state_name)
    }

    fn start_new_state(
        &self,
        next_state_name: &str,
        sprite: &mut Sprite,
        active_state_name: &mut StateName,
        timer: &mut AnimationTimer,
    ) -> Option<()> {
        trace!("Next state: {next_state_name}");

        let next_info = self.state(next_state_name).or_else(|| {
            error!("Dynastes state machine did not have next state {next_state_name}",);
            None
        })?;

        let first = next_info.first();
        let first_duration = next_info.duration(first).or_else(|| {
            error!("Frame {first} does not have duration for state {next_state_name}",);
            None
        })?;

        let mut new_atlas = next_info.atlas().clone();
        new_atlas.index = first;

        sprite.texture_atlas = Some(new_atlas);
        active_state_name.0 = next_state_name.to_string();
        timer.0 = Timer::new(Duration::from_millis(first_duration), TimerMode::Repeating);

        Some(())
    }

    fn set_next_frame(
        &self,
        active_state_name: &mut StateName,
        state_control: &mut StateControl,
        edges: &HashMap<String, Option<String>>,
        sprite: &mut Sprite,
        timer: &mut AnimationTimer,
    ) -> Option<()> {
        let state = self.state(&active_state_name.0).or_else(|| {
            error!(
                "Dynastes does not have current state {}",
                active_state_name.0
            );
            None
        })?;
        let atlas = &mut sprite.texture_atlas.as_mut().or_else(|| {
            error!("Sprite for dynastes does not have texture atlas");
            None
        })?;

        match (state.next_frame(atlas), &state_control.new_state.clone()) {
            (_, Some(SetState::Immediate(next_state))) => {
                state_control.new_state = None;
                trace!("Immediately switching to set next state {next_state}");
                self.start_new_state(next_state, sprite, active_state_name, timer)?;
            }
            (NextFrame::NextState, Some(SetState::OnTransition(overriden_next))) => {
                trace!("Switching to overridden next state {overriden_next}");
                state_control.new_state = None;
                self.start_new_state(overriden_next, sprite, active_state_name, timer)?;
            }
            (NextFrame::NextState, _) => {
                let edge = edges.get(&active_state_name.0).or_else(|| {
                    error!(
                        "Dynastes did not have edge for state {}",
                        active_state_name.0
                    );
                    None
                })?;

                if let Some(next_state_name) = edge {
                    trace!("Switching to default next state {next_state_name}");
                    self.start_new_state(next_state_name, sprite, active_state_name, timer)?;
                } else {
                    trace!("Repeat state: {}", active_state_name.0);
                    state.set_frame(state.first(), atlas, timer)?;
                }
            }
            (NextFrame::FrameIndex(index), _) => {
                state.set_frame(index, atlas, timer)?;
            }
        }

        Some(())
    }
}
