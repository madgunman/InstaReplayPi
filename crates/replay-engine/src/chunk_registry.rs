//! Register buffer chunks from splitmuxsink fragment-opened/closed signals.

use gstreamer as gst;
use tracing::debug;

use crate::chunk_index_actor::ChunkIndexHandle;

pub fn handle_element_message(structure: &gst::StructureRef, handle: &ChunkIndexHandle) {
    if structure.has_name("splitmuxsink-fragment-opened") {
        if let Ok(location) = structure.get::<String>("location") {
            let path = std::path::PathBuf::from(&location);
            handle.note_fragment_opened(&path);
            handle.post_fragment_opened(path.clone());
            debug!(file = %path.display(), "Fragment opened");
        }
    } else if structure.has_name("splitmuxsink-fragment-closed") {
        if let Ok(location) = structure.get::<String>("location") {
            let path = std::path::PathBuf::from(&location);
            let duration_ms = structure
                .get::<u64>("fragment-duration")
                .ok()
                .map(|ns| ns / 1_000_000);
            handle.post_fragment_closed(path.clone(), duration_ms);
            debug!(file = %path.display(), "Fragment closed");
        }
    }
}
