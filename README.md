<div align="center">

# Chiaroscuro

An iRacing telemetry viewer built with Rust and the Iced GUI framework.

</div>

Chiaroscuro is a real-time telemetry viewer designed exclusively for iRacing. It
provides a desktop dashboard for live vehicle, session, lap, and tyre telemetry.

## Project Scope

Chiaroscuro intentionally supports **iRacing only**. Supporting Assetto Corsa,
Assetto Corsa Competizione, Assetto Corsa EVO, GT7, or other simulators is out
of scope. New protocol, telemetry, UI, and data-model work should be designed
around the iRacing SDK instead of introducing multi-simulator abstractions.

The existing `chiaro-ac-udp` crate and Assetto Corsa-oriented telemetry code are
legacy prototype components. They do not represent the target product and will
be replaced or removed as the iRacing integration is implemented.

## Features

- Real-time telemetry data visualization
- Customizable dashboard with multiple widgets
- iRacing SDK telemetry integration
- Lightweight and fast performance
- iRacing-focused vehicle, session, lap, and tyre data model

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any bugs or feature requests.

## Implementation Status

The desktop shell and telemetry dashboard are under active development. The
iRacing SDK data source has not replaced all legacy prototype paths yet; code
that depends on Assetto Corsa shared memory or `chiaro-ac-udp` should be treated
as migration work, not as a supported simulator integration.

## License

This project is licensed under the GNU General Public License, version 3 or later.

- [GPL-3.0-or-later](LICENSE)
