use std::collections::VecDeque;

use crate::{Effect, EffectStore};

#[derive(Debug, Default)]
pub struct BuiltinEffectStore(VecDeque<Effect>);

impl BuiltinEffectStore {
    pub fn new() -> Self {
        Self(VecDeque::default())
    }
}

impl EffectStore for BuiltinEffectStore {
    fn pop(&mut self) -> Option<crate::Effect> {
        self.0.pop_back()
    }

    fn push(&mut self, effect: crate::Effect) {
        self.0.push_front(effect);
    }
}
