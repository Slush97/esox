# esox

A prototype GPU-accelerated UI toolkit for native Linux applications, written in Rust.

Esox renders without a webview or garbage-collected runtime. It currently depends on a Linux desktop stack, a Vulkan driver, and system fonts resolved through fontconfig.

## Why

Esox explores a small, Rust-native alternative to browser-based and traditional desktop UI stacks. Accessibility is a design goal, but screen-reader integration is not functional yet.

## Features

- **Immediate-mode API** — no hidden state, no framework magic. Your app owns all its data.
- **wgpu/Vulkan rendering** — GPU-accelerated with damage tracking, MSAA, instanced draw calls
- **35+ widgets** — buttons, text inputs, tables, trees, virtual scroll (10k+ items), drag-and-drop, modals, tabs, split panes, and more
- **Text pipeline** — rustybuzz shaping, swash rasterization, system font fallback via fontconfig, rich text support
- **Accessibility metadata** — widgets emit a preliminary semantic tree; the optional AT-SPI2 bridge compiles but does not yet expose it to screen readers
- **Theming** — dark/light themes with smooth transitions, per-widget style overrides
- **Native binaries** — no bundled browser engine or garbage-collected runtime

## Quick look

```rust
impl AppDelegate for MyApp {
    fn on_redraw(&mut self, gpu: &GpuContext, resources: &mut RenderResources,
                 frame: &mut Frame, _perf: &PerfMonitor) {
        let mut ui = Ui::begin(frame, gpu, resources, &mut self.text,
                               &mut self.ui_state, &self.theme, viewport);

        ui.padding(24.0, |ui| {
            ui.heading("Hello");

            if ui.button(id!("greet"), "Click me").clicked {
                println!("clicked");
            }

            ui.text_input(id!("name"), &mut self.name_input, "Your name");

            ui.checkbox(id!("agree"), &mut self.agree_state, "I agree");
        });

        ui.finish();
    }
}
```

## Building

Requires the pinned Rust toolchain, a Vulkan-capable GPU and driver, a Wayland or X11 desktop, and fontconfig (`fc-match`) for system font discovery.

```sh
# run the demo
cargo run -p demo --release
```

System dependencies (Arch):
```sh
pacman -S vulkan-icd-loader fontconfig
```

## Project structure

```
crates/
  esox_ui/        # widget library and layout engine
  esox_gfx/       # GPU rendering, shaders, atlas management
  esox_font/      # font loading, shaping, rasterization
  esox_platform/  # windowing, input, clipboard, a11y bridge
  esox_input/     # platform-independent input types
examples/
  demo/           # widget and layout showcase
```

## Roadmap

See the [architecture review and stabilization plan](ARCHITECTURE_REVIEW_PLAN.md)
for the current implementation sequence and [ROADMAP.md](ROADMAP.md) for the
long-term product direction. The short version:

1. **Accessibility & i18n** — finish AT-SPI2 integration, keyboard nav for all widgets, RTL/BiDi text
2. **Modern UX** — spring animations, design tokens, subpixel text, more widgets
3. **Developer experience** — docs, devtools overlay, widget gallery
4. **Ecosystem** — crates.io, CI/CD, example app templates

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
