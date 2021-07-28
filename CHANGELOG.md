# Smithay Changelog

## Unreleased

### Bugfixes

- EGLBufferReader now checks if buffers are alive before using them.

## version 0.3.0 (2021-07-25)

Large parts of Smithay were changed with numerous API changes. It is thus recommended to
approach version 0.3 as if it was a new crate altogether compared to 0.2.

The most notable changes are:

- Deep refactor of the graphics backends around a workflows centered on allocating graphics buffers,
  and a Gles2-based renderer abstraction is provided.
- Support for DRM atomic modesetting as well as client-provided DMABUF
- Most backends are now `calloop` event sources generating events. The recommended organization for
  your smithay-based compositor is thus to centralize most of your logic on a global state struct,
  and delegate event handling to it via the shared data mechanism of `calloop`. Most of the callbacks
  you provide to Smithay are given mutable access to this shared data.
- The `wayland::compositor` handling logic now automatically handles state tracking and delayed commit
  for wayland surfaces.

Many thanks to the new contributors to Smithay, who contributed the following:

- Support for [`libseat`](https://sr.ht/~kennylevinsen/seatd/) as a session backend, by
  @PolyMeilex
- Support for graphics tablets via the `tablet` protocol extension, by @PolyMeilex
- Support for running Smithay on `aarch64` architectures, by @cmeissl
- A rework of the `xdg-shell` handlers to better fit the protocol logic and correctly track configure
  events, by @cmeissl
- Basic Xwayland support, by @psychon

## version 0.2.0 (2019-01-03)

### General

- **[Breaking]** Upgrade to wayland-rs 0.21
- **[Breaking]** Moving the public dependencies to a `reexports` module
- Migrate the codebase to Rust 2018

### Backends

- **[Breaking]** WinitBackend: Upgrade to winit 0.18
- **[Breaking]** Global refactor of the DRM & Session backends
- **[Breaking]** Restructuration of the backends around the `calloop` event-loop

### Clients & Protocol

- Basic XWayland support
- Data device & Drag'n'Drop support
- Custom client pointers support

## version 0.1.0 (2017-10-01)

### Protocol handling

- Low-level handling routines for several wayland globals:
  - `wayland::shm` handles `wl_shm`
  - `wayland::compositor` handles `wl_compositor` and `wl_subcompositor`
  - `wayland::shell` handles `wl_shell` and `xdg_shell`
  - `wayland::seat` handles `wl_seat`
  - `wayland::output` handles `wl_output`

### Backend

- Winit backend (EGL context & input)
- DRM backend
- libinput backend
- glium integration