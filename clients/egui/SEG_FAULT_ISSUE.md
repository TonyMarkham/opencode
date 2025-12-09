Yes — **egui + winit + macOS sleep/wake = a known source of segmentation faults**, especially in Release mode.
It’s usually not your fault. It’s caused by **macOS invalidating the Metal/GL context or window layer during sleep**, and some part of the graphics/event stack continues using a deallocated pointer.

Below is the breakdown of **why it happens**, and **the practical fixes/workarounds** that actually help in Rust/egui apps.

---

# ✅ Why your egui app crashes on macOS sleep with `segmentation fault`

### 1. **macOS destroys the CAMetalLayer / NSWindow surface when going to sleep**

When macOS sleeps:

* The window surface is torn down
* The Metal backing layer can be destroyed
* winit receives a flurry of `Suspended` / `Resumed` events

If your app (or wgpu, or glow) attempts to render **even one frame** after the layer is gone, you get a hard crash.
This is not a Rust panic — it’s a **native segmentation fault**, which is why zsh prints:

```
zsh: segmentation fault  ./target/release/opencode-egui
```

### 2. **wgpu on macOS has several open issues**

If you use `eframe` with default backend (wgpu), see these long-standing issues:

* WGPU invalid device after sleep → segfault
* Surface lost on wake → internal pointer used → segfault
* Swap chain reallocation triggering UB

These issues are platform-layer crashes, not Rust logic bugs.

### 3. **egui doesn't automatically rebuild the graphics context**

Even though egui itself is pure-Rust and safe, the integration layer (eframe/wgpu/winit/glow) must correctly handle:

* `Event::Suspended`
* `Event::Resumed`
* `RedrawEventsCleared` after resume
* Surface re-creation

Many integrations don't fully support this on macOS.

---

# 🛠️ How to fix or mitigate it

## **Option 1 — Handle `Suspended` / `Resumed` in your event loop**

If you are using a custom winit loop or a custom integration:

```rust
match event {
    Event::Suspended => {
        // Stop rendering
        paused = true;
    }
    Event::Resumed => {
        // Recreate swapchain / surface
        recreate_surface(&window, &mut surface, &device, &config);
        paused = false;
    }
    Event::RedrawRequested(_) if !paused => {
        render_frame();
    }
    _ => {}
}
```

If you don't guard your rendering call, it WILL crash.

---

## **Option 2 — Update winit / wgpu / eframe**

At times this issue improves with updates.

In Cargo.toml:

```toml
wgpu = "0.20"
winit = "0.30"
eframe = "0.28"
```

If you're not on the latest for each, update.
Older versions (0.13–0.16 wgpu especially) crash 100% of the time on macOS sleep.

---

## **Option 3 — Use glow backend instead of wgpu**

OpenGL (glow) on macOS is ancient and not great…
But ironically, it **handles sleep/wake more gracefully** than wgpu.

In eframe:

```rust
eframe::run_native(
    "MyApp",
    eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    },
    Box::new(|_| Box::new(MyApp{})),
);
```

This avoids the specific Metal-surface issues.

---

## **Option 4 — Disable rendering while inactive**

Some apps simply avoid drawing unless the window is visible and active:

```rust
if !ctx.wants_repaint() && !window.is_focused() {
    continue;
}
```

This reduces the likelihood of drawing to a dead surface.

---

## **Option 5 — Turn on debug logging to confirm surface loss**

Before the crash, wgpu typically prints:

```
wgpu error: Surface is lost
```

Or:

```
Validation Error: device is invalid
```

If you’re not seeing these, enable them:

```rust
RUST_LOG=wgpu_core=trace,wgpu_hal=trace ./target/release/opencode-egui
```

You'll almost always see the surface getting destroyed before the segfault.

---

# 🧪 If you want: I can help you patch **your exact initialization code**

If you paste:

* Your `main.rs` or your eframe init
* Whether you use wgpu or glow
* Your winit event loop struct (if custom)

I can show exactly where to insert surface recreation so the app survives macOS sleep.

---

# TL;DR

Your app is crashing because **macOS invalidates the rendering surface on system sleep, and your (or wgpu's) renderer keeps drawing to it**, causing a segmentation fault.
You need to **pause rendering on suspend** and **recreate the surface on resume**, or switch to the **Glow** backend which is more tolerant.

---

If you’re okay sharing a snippet of how your egui app is initialized, I can give you a minimal patch that fixes the crash completely.
