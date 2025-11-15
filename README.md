# rusty-bvg 🚇

Real-time Berlin public transport departure board for Raspberry Pi + LED matrix.

## What it does

- Shows the next 3 departures from S+U Warschauer Str (U-Bahn, S-Bahn and trams)
- Updates every 20 seconds
- Cycles through departures every 10 seconds

## Hardware you need

- Raspberry Pi (I used a Pi 3, but anything with GPIO should work)
- 64x32 RGB LED Matrix Panel (get the bigger one :)
- 5V power supply for the matrix (don't skimp on this, it needs power)
- Some jumper wires

## Setup

First, install Rust on your Pi:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then clone RGB LED matrix library:

```bash
git clone https://github.com/hzeller/rpi-rgb-led-matrix.git
cd rpi-rgb-led-matrix
make -C lib
sudo make -C lib install
```

Install OpenSSL dev packages (needed for HTTPS):

```bash
sudo apt install libssl-dev pkg-config
```

Clone this repo and build it:

```bash
git clone https://github.com/gleb-gusev/rusty-bvg.git
cd rusty-bvg
cargo build --release --features display
```

Run it:

```bash
sudo ./target/release/rusty-bvg
```

You need sudo for GPIO access.

## Development

API Part could be tested separately.

```bash
cargo build
cargo test
cargo run  # prints departures to console
```

The display code is behind a feature flag so it only compiles on the Pi.

## How it works

- `src/api.rs` - talks to the VBB API
- `src/display.rs` - handles the LED matrix rendering
- `src/departure.rs` - data model for departures
- `src/main.rs` - ties everything together



## TODO
- [ ] Hardcoded to Warschauer Str. - should make it configurable

## Credits

- [rpi-rgb-led-matrix](https://github.com/hzeller/rpi-rgb-led-matrix) by Henner Zeller - the LED matrix library
- [VBB HAFAS API](https://v6.vbb.transport.rest/) - the public transport API
- The BVG for having actual useful data available via API