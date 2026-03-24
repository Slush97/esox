# esox

A GPU-accelerated UI toolkit for native Linux applications, written in Rust.

~8MB binaries. No runtime dependencies beyond your system's Vulkan driver. No webview. No garbage collector. No framework tax.

## Why

Every Linux UI toolkit either ships a browser engine (Electron), depends on a sprawling C runtime (GTK/Qt), or asks you to give up on accessibility. esox is an attempt at something better: a small, fast, accessible toolkit that produces standalone native binaries.

## Features

- **Immediate-mode API** — no hidden state, no framework magic. Your app owns all its data.
- **wgpu/Vulkan rendering** — GPU-accelerated with damage tracking, MSAA, instanced draw calls
- **35+ widgets** — buttons, text inputs, tables, trees, virtual scroll (10k+ items), drag-and-drop, modals, tabs, split panes, and more
- **Text pipeline** — rustybuzz shaping, swash rasterization, system font fallback via fontconfig, rich text support
- **Accessibility** — AT-SPI2 integration for screen reader support (in progress)
- **Theming** — dark/light themes with smooth transitions, per-widget style overrides
- **Tiny binaries** — release builds around 8MB with everything included

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

Requires Rust 2024 edition and a Vulkan-capable GPU.

```sh
# run the demo
cargo run -p demo --release

# run other examples
cargo run -p layout_showcase --release
cargo run -p material_showcase --release
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
  demo/             # full widget showcase
  layout_showcase/  # layout system examples
  material_showcase/ # themed component gallery
```

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the full plan. The short version:

1. **Accessibility & i18n** — finish AT-SPI2 integration, keyboard nav for all widgets, RTL/BiDi text
2. **Modern UX** — spring animations, design tokens, subpixel text, more widgets
3. **Developer experience** — docs, devtools overlay, widget gallery
4. **Ecosystem** — crates.io, CI/CD, example app templates

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
