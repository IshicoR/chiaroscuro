# Directory Structure Memo

This note records the intended module boundaries for the Rust/Iced desktop
application as Chiaroscuro grows. It is a direction, not a requirement to create
all modules immediately.

## Current Shape

```text
chiaro-rs/
  telemetry/
    src/
      lib.rs
      sample.rs
      wire.rs
  ac-udp/
    src/
      context.rs
      error.rs
      setting.rs
      shared_memory.rs
      udp.rs
  desktop/
    src/
      action.rs
      app.rs
      appearance.rs
      desktop.rs
      menu.rs
      navigation.rs
      screen.rs
      screen/
        about.rs
        dashboard.rs
        settings.rs
      session.rs
      telemetry.rs
      widget.rs
      widget/
        telemetry/
          card.rs
          style.rs
          time_series.rs
        window_controls.rs
      window.rs
```

## Target Shape

```text
chiaro-rs/desktop/src/
  desktop.rs          # binary entrypoint only
  app.rs              # State, Message, update, subscription, theme
  action.rs           # requests from child modules to the application root
  navigation.rs       # current page and navigation history
  session.rs          # shared runtime and telemetry state
  screen.rs           # screen module exports
  screen/
    dashboard.rs      # main telemetry view
    settings.rs       # user and runtime settings
    about.rs          # project/app information
  window.rs           # window events, close/minimize/maximize/drag commands
  menu.rs             # menu bar state, menu actions, accelerators
  appearance.rs       # theme tokens, colors, spacing, reusable style choices
  icon.rs             # app-level icon helpers
  widget.rs           # shared widget exports
  widget/
    window_controls.rs
    status_bar.rs
    telemetry_card.rs
  telemetry/
    mod.rs
    source.rs         # telemetry source abstraction
    state.rs          # UI-facing telemetry state
    packet.rs         # decoded telemetry payloads
  config.rs           # persisted configuration
  key_bind.rs         # keyboard shortcuts and command mapping
```

## Boundaries

- `desktop.rs` should stay thin: allocator setup, application construction, and
  initial runtime wiring.
- `app.rs` owns the main application state machine: messages, update logic,
  subscriptions, and task routing.
- `navigation.rs` owns the current `Page` and the minimal history needed for
  back navigation. It is intentionally named after its responsibility instead
  of acting like a web router.
- `session.rs` owns runtime state shared by multiple screens, such as telemetry
  connection state and received data. Window and menu state remain in their own
  modules instead of becoming part of a generic context.
- `action.rs` defines requests that a child cannot complete locally, such as
  navigation, changing shared session state, or closing the window.
- `screen/` owns whole-page views. A screen may define its own local `Message`,
  then map it into the root `Message` from `app.rs`.
- `widget/` owns reusable visual pieces. Widgets should not own application-wide
  state or perform telemetry IO.
- `window.rs` owns platform/window commands such as close, minimize, maximize,
  drag, and close-request handling.
- `menu.rs` owns the hamburger toggle and the `iced_aw` desktop-style drop-down
  menus. Menu actions map into root messages instead of directly mutating
  unrelated state.
- `telemetry/` owns domain data and conversion into UI-facing state. It should
  not depend on Iced widgets.
- `chiaro-rs/telemetry` owns the simulator-neutral sample and versioned wire
  format shared by relay binaries and the desktop. Simulator-specific shared
  memory layouts remain in their source crate.
- `appearance.rs` centralizes visual decisions so custom title bars, menu bars,
  and dashboard widgets do not drift into unrelated styles.

## Message Flow

```text
child view
  -> child Message
  -> Element::map(root Message)
  -> App::update
  -> child update
  -> Task::map(root Message) + optional Action
  -> App::handle_action
  -> Navigation / Session / Window
```

Screen-local changes stay in the screen state. Cross-screen or application-wide
requests are returned as an `Action` and handled by `App`; screens do not emit or
mutate the root `Message` or `App` directly.

## Growth Rule

The initial split now includes `app.rs`, `action.rs`, `navigation.rs`,
`session.rs`, `window.rs`, `menu.rs`, `appearance.rs`, and three screens. Add
deeper folders only when there are at least two real users of the boundary or a
file has a clear single responsibility that no longer fits in the current
module.

## References

- Icebreaker: screen/widget split in a compact Iced app
  <https://github.com/hecrj/icebreaker>
- Halloy: larger Iced app with screen, widget, appearance, modal, and window
  modules
  <https://github.com/squidowl/halloy>
- Sniffnet: domain modules plus GUI pages/components/styles split
  <https://github.com/GyulyVGC/sniffnet>
- Iced examples: official examples for app, task, subscription, and window
  patterns
  <https://github.com/iced-rs/iced/tree/0.14.0/examples>
- iced_aw: overlay menu widgets used by the desktop menu bar
  <https://github.com/iced-rs/iced_aw>
- iced_plot: GPU-accelerated telemetry plots used by the dashboard
  <https://github.com/donkeyteethUX/iced_plot>
