pub mod ffmpeg;
pub mod ffmpeg_path;
pub mod metadata;
pub mod pipeline;
pub mod status;
pub mod webcast;

pub use pipeline::{start, StreamHandle};
pub use status::{emit as emit_status, StreamStatus};
pub use webcast::MetadataSink;
