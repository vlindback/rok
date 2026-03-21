// io_capabilities.rs

pub struct IoCapabilities {
    // how many operations you can have in-flight simultaneously
    pub max_sq_entries: u32,

    // how many completions can pile up before you must drain them
    pub max_cq_entries: u32,

    // only relevant on windows
    pub emulated: bool,
}
