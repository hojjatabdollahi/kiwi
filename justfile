set dotenv-load := true

name := 'kiwi'
export APPID := 'dev.hojjat.kiwi'

rootdir := ''
prefix := '/usr'

base-dir := absolute_path(clean(rootdir / prefix))

export INSTALL_DIR := base-dir / 'share'

daemon-src := 'target' / 'release' / 'kiwi-daemon'
daemon-dst := base-dir / 'bin' / 'kiwi-daemon'

applet-src := 'target' / 'release' / 'kiwi-applet'
applet-dst := base-dir / 'bin' / 'kiwi-applet'

desktop-src := 'data' / 'dev.hojjat.kiwi.applet.desktop'
desktop-dst := base-dir / 'share' / 'applications' / 'dev.hojjat.kiwi.applet.desktop'

service-src := 'data' / 'kiwi-daemon.service'
service-dst := base-dir / 'lib' / 'systemd' / 'user' / 'kiwi-daemon.service'

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

# Runs applet with debug profile
run-applet *args:
    cargo run -p kiwi-applet {{args}}

# Install files
install:
    install -Dm0755 {{daemon-src}} {{daemon-dst}}
    install -Dm0755 {{applet-src}} {{applet-dst}}
    install -Dm0644 {{desktop-src}} {{desktop-dst}}
    install -Dm0644 {{service-src}} {{service-dst}}

# Uninstall files
uninstall:
    rm -f {{daemon-dst}}
    rm -f {{applet-dst}}
    rm -f {{desktop-dst}}
    rm -f {{service-dst}}
