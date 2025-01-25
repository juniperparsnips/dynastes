use std::collections::HashMap;

use bevy::{
    asset::{Asset, Handle},
    image::Image,
    prelude::Component,
    reflect::{Reflect, TypePath},
    sprite::TextureAtlas,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Component, Asset, Reflect)]
/// A finite state machine across animation states
pub struct AnimationStateMachine<S>
where
    S: Send + Sync + TypePath,
{
    pub image: Handle<Image>,
    pub default_state_name: String,
    pub states: HashMap<String, StateInfo<S>>,
}

impl<S> AnimationStateMachine<S>
where
    S: Send + Sync + TypePath,
{
    pub fn default_state(&self) -> Option<&StateInfo<S>> {
        self.states.get(&self.default_state_name)
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
