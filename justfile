set dotenv-load := true

name := 'kiwi'
export APPID := 'dev.hojjat.kiwi'

rootdir := ''
prefix := '/usr'

base-dir := absolute_path(clean(rootdir / prefix))

export INSTALL_DIR := base-dir / 'share'

daemon-src := 'target' / 'release' / 'kiwi-daemon'
daemon-dst := base-dir / 'bin' / 'kiwi-daemon'

app-src := 'target' / 'release' / 'kiwi'
app-dst := base-dir / 'bin' / 'kiwi'

desktop-src := 'data' / 'dev.hojjat.kiwi.desktop'
desktop-dst := base-dir / 'share' / 'applications' / 'dev.hojjat.kiwi.desktop'

service-src := 'data' / 'kiwi-daemon.service'
service-dst := base-dir / 'lib' / 'systemd' / 'user' / 'kiwi-daemon.service'

icon-dir := base-dir / 'share' / 'icons' / 'hicolor'
icon-src-dir := 'data' / 'icons'

default: build-release

# Compiles in debug mode
build-debug *args:
    cargo build {{args}}

# Compiles in release mode
build-release *args:
    cargo build --release {{args}}

# Check with cargo
check *args:
    cargo check {{args}}

# Cleans build artifacts
clean:
    cargo clean

# Runs daemon with debug profile
run-daemon *args:
    cargo run -p kiwi-daemon {{args}}

# Runs app with debug profile
run-app *args:
    cargo run -p kiwi-app {{args}}

# Install files
install:
    install -Dm0755 {{daemon-src}} {{daemon-dst}}
    install -Dm0755 {{app-src}} {{app-dst}}
    install -Dm0644 {{desktop-src}} {{desktop-dst}}
    install -Dm0644 {{service-src}} {{service-dst}}
    # Install PNG icons at various sizes for tray icon
    install -Dm0644 {{icon-src-dir}}/kiwi-on-16.png {{icon-dir}}/16x16/apps/kiwi-on.png
    install -Dm0644 {{icon-src-dir}}/kiwi-off-16.png {{icon-dir}}/16x16/apps/kiwi-off.png
    install -Dm0644 {{icon-src-dir}}/kiwi-on-22.png {{icon-dir}}/22x22/apps/kiwi-on.png
    install -Dm0644 {{icon-src-dir}}/kiwi-off-22.png {{icon-dir}}/22x22/apps/kiwi-off.png
    install -Dm0644 {{icon-src-dir}}/kiwi-on-24.png {{icon-dir}}/24x24/apps/kiwi-on.png
    install -Dm0644 {{icon-src-dir}}/kiwi-off-24.png {{icon-dir}}/24x24/apps/kiwi-off.png
    install -Dm0644 {{icon-src-dir}}/kiwi-on-32.png {{icon-dir}}/32x32/apps/kiwi-on.png
    install -Dm0644 {{icon-src-dir}}/kiwi-off-32.png {{icon-dir}}/32x32/apps/kiwi-off.png
    install -Dm0644 {{icon-src-dir}}/kiwi-on-48.png {{icon-dir}}/48x48/apps/kiwi-on.png
    install -Dm0644 {{icon-src-dir}}/kiwi-off-48.png {{icon-dir}}/48x48/apps/kiwi-off.png
    install -Dm0644 {{icon-src-dir}}/kiwi-on-64.png {{icon-dir}}/64x64/apps/kiwi-on.png
    install -Dm0644 {{icon-src-dir}}/kiwi-off-64.png {{icon-dir}}/64x64/apps/kiwi-off.png
    install -Dm0644 {{icon-src-dir}}/kiwi-on-128.png {{icon-dir}}/128x128/apps/kiwi-on.png
    install -Dm0644 {{icon-src-dir}}/kiwi-off-128.png {{icon-dir}}/128x128/apps/kiwi-off.png
    install -Dm0644 {{icon-src-dir}}/kiwi-on-256.png {{icon-dir}}/256x256/apps/kiwi-on.png
    install -Dm0644 {{icon-src-dir}}/kiwi-off-256.png {{icon-dir}}/256x256/apps/kiwi-off.png
    # Install SVG only for desktop file icon (not tray)
    install -Dm0644 {{icon-src-dir}}/kiwi-on.svg {{icon-dir}}/scalable/apps/kiwi.svg

# Uninstall files
uninstall:
    rm -f {{daemon-dst}}
    rm -f {{app-dst}}
    rm -f {{desktop-dst}}
    rm -f {{service-dst}}
    # Remove icons
    rm -f {{icon-dir}}/16x16/apps/kiwi-on.png {{icon-dir}}/16x16/apps/kiwi-off.png
    rm -f {{icon-dir}}/22x22/apps/kiwi-on.png {{icon-dir}}/22x22/apps/kiwi-off.png
    rm -f {{icon-dir}}/24x24/apps/kiwi-on.png {{icon-dir}}/24x24/apps/kiwi-off.png
    rm -f {{icon-dir}}/32x32/apps/kiwi-on.png {{icon-dir}}/32x32/apps/kiwi-off.png
    rm -f {{icon-dir}}/48x48/apps/kiwi-on.png {{icon-dir}}/48x48/apps/kiwi-off.png
    rm -f {{icon-dir}}/64x64/apps/kiwi-on.png {{icon-dir}}/64x64/apps/kiwi-off.png
    rm -f {{icon-dir}}/128x128/apps/kiwi-on.png {{icon-dir}}/128x128/apps/kiwi-off.png
    rm -f {{icon-dir}}/256x256/apps/kiwi-on.png {{icon-dir}}/256x256/apps/kiwi-off.png
    rm -f {{icon-dir}}/scalable/apps/kiwi.svg
