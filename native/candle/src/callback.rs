use std::collections::HashMap;
pub struct Context {
    pub channel: u8,
}
pub type Contexts = HashMap<String, Context>;
pub fn remove(contexts: &mut Contexts, name: &str) -> bool {
    contexts.remove(name).is_some()
}
