use std::{collections::HashMap, marker::PhantomData};

use bevy::{
    asset::{Asset, AssetLoader, AsyncReadExt, Handle, LoadContext},
    image::Image,
    prelude::Component,
    reflect::{Reflect, TypePath},
    sprite::TextureAtlas,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

#[derive(Default)]
pub struct DynastesLoader<T> {
    phantom: PhantomData<T>,
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

#[derive(Debug, Clone, Copy)]
pub enum NextFrame {
    NextState,
    FrameIndex(usize),
}

pub trait State {
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
}

pub trait StateSystem {
    type State: State;

    fn image(&self) -> &Handle<Image>;

    fn states(&self) -> &HashMap<String, Self::State>;

    fn state(&self, state_name: &str) -> Option<&Self::State> {
        self.states().get(state_name)
    }
}
