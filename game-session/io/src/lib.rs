#![no_std]

use gmeta::{InOut, Metadata};
use gstd::{prelude::*, ActorId};

pub struct GameSessionMetadata;

impl Metadata for GameSessionMetadata {
    type Init = ();
    type Handle = InOut<Action, Event>;
    type Others = ();
    type Reply = ();
    type Signal = ();
    type State = ();
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum Action {
    // todo
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum Event {
    // todo
}