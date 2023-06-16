cargo_project = 'proc_reader'

cargo_bin  = find_program('cargo')
cargo_opt  = [ '--manifest-path', meson.project_source_root() / cargo_project / 'Cargo.toml' ]
cargo_opt += [ '--target-dir', meson.project_build_root()  / cargo_project / 'src' ]
cargo_opt += [ '--target', 'x86_64-unknown-linux-gnu' ]
cargo_env  = [ 'CARGO_HOME=' + meson.project_build_root()  / cargo_project/ 'cargo-home' ]
cargo_env += [ 'RUSTFLAGS=' + '-C' + 'target-feature=+crt-static' ]

if get_option('buildtype') == 'release'
  cargo_opt += [ '--release' ]
  rust_target = 'release'
else
  rust_target = 'debug'
endif

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: 'missioncenter_' + cargo_project,
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    'env', cargo_env,
    cargo_bin, 'build',
    cargo_opt, '&&', 'cp', cargo_project/ 'src' / 'x86_64-unknown-linux-gnu' / rust_target / cargo_project, '@OUTPUT@',
  ]
)
