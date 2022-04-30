.DEFAULT_GOAL := help

copy_macapp_contents: mac_build_app_release
# Once done, copy the generated executable and put it inside the .app package.
	cp target/release/psst-gui psst-gui/mac/psst-gui.app/Contents/MacOS/

# Ok, we have the app, copy it to target.
	cp -r psst-gui/mac/psst-gui.app target/release/

mac_build_app_release:
# Create the release build.
	cargo build --release

help:
	@echo Available Targets:
	@echo "\tmac_build_app_release - Creates the MacOS .app bundle."
	@echo "\tclean - Cleans the cargo and the executable to bundle."
	@echo "\tcopy_macapp_contents - Copies contents from the compiled executable to the .app bundle."

clean:
# Regular rust cleanup
	cargo clean

# Clear the executable from the app
	rm psst-gui/mac/psst-gui.app/Contents/MacOS/psst-gui