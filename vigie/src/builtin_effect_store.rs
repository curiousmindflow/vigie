use vigie_core::EffectStore;

#[derive(Debug)]
pub struct BuiltinEffectStore {}

impl EffectStore for BuiltinEffectStore {
    fn pop(&mut self) -> Option<vigie_core::Effect> {
        todo!()
    }

    fn push(&mut self, effect: vigie_core::Effect) {
        todo!()
    }
}
