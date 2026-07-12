<div align="center">

# Chiaroscuro

A real-time telemetry viewer for racing simulators, built with Rust and the Iced GUI framework.

</div>

Chiaroscuro is a real-time telemetry viewer for racing simulators, built with Rust and the Iced GUI framework. It allows you to monitor various in-game data such as speed, RPM, lap times, and more. Currently, it supports Assetto Corsa, with plans to expand to other simulators like Assetto Corsa Competizione and iRacing in the future.

## Features

- Real-time telemetry data visualization
- Customizable dashboard with multiple widgets
- Support for Assetto Corsa shared memory (with plans to support other simulators)
- Lightweight and fast performance
- Extensible architecture for future simulator support

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any bugs or feature requests.

## Architecture Notes

- [Directory structure memo](docs/directory-structure.md)

## Assetto Corsa Telemetry

`chiaro-ac-udp` runs on the Windows machine hosting Assetto Corsa. It reads the
`Local\\acpmf_physics` and `Local\\acpmf_graphics` shared-memory pages and sends
versioned telemetry packets to registered desktop clients.

1. Set `bind_addr` in `settings.toml` to the relay's UDP listen address.
2. Set `server_addr` to that address from the desktop machine's point of view.
3. Start Assetto Corsa, then run `cargo run -p chiaroscuro-ac-udp` on Windows.
4. Run `cargo run -p chiaroscuro-desktop` and press **Connect**.

The live dashboard includes speed, RPM, gear, fuel, lap timing, pedal traces,
longitudinal/lateral G, and four-wheel tyre core temperatures. If the two
programs run on different machines, allow the configured UDP port through the
Windows firewall.

### Mock telemetry

The full relay and desktop flow can be tested without Assetto Corsa, including
on Linux. Set the following value in `settings.toml`:

```toml
mock_telemetry = true
```

Then start the relay and desktop in separate terminals:

```shell
cargo run -p chiaroscuro-ac-udp
cargo run -p chiaroscuro-desktop
```

Press **Connect** in the desktop application. The dashboard will receive
animated speed, pedal, G-force, tyre-temperature, fuel, and lap data over the
same UDP registration and packet path used by live telemetry. Set
`mock_telemetry = false` before connecting to Assetto Corsa shared memory.

## License

This project is licensed under the GNU General Public License, version 3 or later.

- [GPL-3.0-or-later](LICENSE)

## Future Plans

We plan to expand support to other racing simulators, including:
- Assetto Corsa Competizione
- Assetto Corsa EVO
- iRacing
- GT7
- And more!
