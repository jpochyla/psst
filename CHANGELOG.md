# Changelog

## [Unreleased] - 2025-11-22

### Fixed
- **macOS Audio Stability**: Resolved SIGSEGV crash on macOS by strictly using `device.default_output_config()` instead of enumerating configurations in `psst-core/src/audio/output/cpal.rs`.
- **Artist Image Cropping**: Implemented consistent square cropping for artist images fetched from the WebAPI and loaded from disk cache to ensure proper aspect fill within circular UI elements. Extracted cropping logic into `psst-gui/src/data/utils::crop_to_square` for reusability.
- **Artist Name Alignment**: Centered artist names displayed under thumbnails in `psst-gui/src/ui/artist.rs` for improved visual consistency.
- **Artist Bio Visibility**: Ensured artist image and biography section are hidden when an artist has no bio, preventing empty spaces on the artist detail page.
- **Show Info Layout**: Refactored `psst-gui/src/ui/show.rs` to remove dependency on `InfoLayout` and `stat_row`, utilizing standard `Flex` layouts for show description and stats, aligning with new UI conventions.
- **Home and Artist Section Headers**: Standardized styling and alignment for headers such as "Your top tracks", "Your top artists", and "Related Artists" to match the centered style of other prominent sections ("Popular", "Albums").
- **Compilation Errors**: Resolved various compilation errors in `psst-gui/src/ui/home.rs`, `psst-gui/src/ui/show.rs`, and `psst-gui/src/ui/artist.rs` caused by refactoring and missing imports.
- **Unused Code Cleanup**: Removed unused imports (`Data`, `Insets`, `LabelText`, `CrossAxisAlignment`, `Point`, `WithCtx`, `Image`) and unused functions (`format_number_with_commas`, `InfoLayout`, `stat_row`) across `psst-gui` modules for a cleaner codebase.

### Changed
- **Artist Stats Display**: Removed misleading "followers", "monthly listeners", and "world rank" statistics from the artist profile page, as accurate data is not available through the current API.
- **Biography Scroll Behavior**: Constrained the height of the biography scroll area in the artist detail page to match the artist image height, ensuring a compact and consistent header section.
