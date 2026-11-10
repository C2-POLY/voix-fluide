# dmgbuild settings for Voix-Fluide.dmg, read by bundle.sh:
#   dmgbuild -s dmg-settings.py -D app=<Voix Fluide.app> -D assets=<assets dir> "<volume name>" <out.dmg>
# dmgbuild writes the Finder layout itself (no AppleScript), and joins background.png
# with background@2x.png into one Retina image.
import os.path

app = defines["app"]  # noqa: F821 (given by dmgbuild)
assets = defines["assets"]  # noqa: F821

format = "UDZO"
files = [app]
symlinks = {"Applications": "/Applications"}
icon = os.path.join(app, "Contents", "Resources", "icon.icns")

# The window: 660x420 of content under the title bar (Finder counts the title bar in the
# height), with no toolbar or sidebar. The path, status and tab bars follow the person's
# global Finder settings, which a .dmg cannot override.
background = os.path.join(assets, "dmg", "background.png")
window_rect = ((200, 140), (660, 420 + 30))
default_view = "icon-view"
show_toolbar = False
show_sidebar = False
show_pathbar = False
show_status_bar = False
show_tab_view = False

# Icons sit on the tiles of the background, either side of the wave (see background.svg).
icon_size = 128
text_size = 12
icon_locations = {
    os.path.basename(app): (160, 215),
    "Applications": (500, 215),
}
