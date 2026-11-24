# DiffusionDemo: reaction-diffusion teaching tools

Browser tools for an undergraduate computational systems biology class,
written in Rust with [egui](https://github.com/emilk/egui) and compiled to
WebAssembly. Served from GitHub Pages:

- Landing page: `https://<owner>.github.io/DiffusionDemo/`
- Diffusion Painter: `https://<owner>.github.io/DiffusionDemo/diffusion2d/`

The full spec is in `Spec 2D Diffusion Painter (Rust + egui, GitHub Pages).md`.

## Layout

```
Cargo.toml              workspace
crates/rd-core/         grid, Laplacian, boundaries, brush, colormap (no GUI)
apps/diffusion2d/       eframe app: UI and input only (index.html is the trunk entry)
site/index.html         landing page linking to each tool
.github/workflows/      test, build and deploy to Pages on push to main
```

## Develop

```
cargo test -p rd-core                 # physics tests
cargo run -p diffusion2d --release    # native window
cd apps/diffusion2d && trunk serve    # web, at the printed localhost address
```

Web builds need `rustup target add wasm32-unknown-unknown` and
`cargo install --locked trunk`.

On Windows, Git Bash rewrites arguments that look like paths, which breaks
`--public-url /DiffusionDemo/diffusion2d/`. Prefix the command with
`MSYS_NO_PATHCONV=1`, or run it from PowerShell.

## Deploy

In the repository settings, set **Pages > Source** to **GitHub Actions**.
Every push to `main` then runs the tests, builds each app under `apps/` with
trunk and publishes the result. To add a tool, add its folder under `apps/`,
list it in the workspace `Cargo.toml` and in `APPS` in
`.github/workflows/pages.yml`, and link it from `site/index.html`.
