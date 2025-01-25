use std::collections::HashMap;

use bevy::prelude::*;
use dynastes::{State, StateSystem};

#[derive(Debug, TypePath)]
pub struct SomeState {
    pub name: String,
    pub first: usize,
    pub last: usize,
    pub durations: Vec<u64>,
    pub atlas: TextureAtlas,
}

impl State for SomeState {
    fn name(&self) -> &str {
        &self.name
    }

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

#[derive(Debug, TypePath, Asset)]
pub struct SomeStateSystem {
    pub image: Handle<Image>,
    pub states: HashMap<String, SomeState>,
}

impl StateSystem for SomeStateSystem {
    type State = SomeState;

    fn image(&self) -> &Handle<Image> {
        &self.image
    }

    fn states(&self) -> &HashMap<String, Self::State> {
        &self.states
    }
}
