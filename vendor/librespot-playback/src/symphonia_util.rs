use symphonia::core::meta::Metadata;
use symphonia::core::probe::ProbeResult;

pub fn get_latest_metadata(probe_result: &mut ProbeResult) -> Option<Metadata<'_>> {
    let mut metadata = probe_result.format.metadata();

    // If we can't get metadata from the container, fall back to other tags found by probing.
    // Note that this is only relevant for local files.
    if metadata.current().is_none() {
        if let Some(inner_probe_metadata) = probe_result.metadata.get() {
            metadata = inner_probe_metadata;
        }
    }

    _ = metadata.skip_to_latest();

    Some(metadata)
}
