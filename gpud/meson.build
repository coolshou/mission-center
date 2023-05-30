libnvtop = dependency(
  'nvtop',
  fallback : ['nvtop', 'nvtop'],
  default_options: ['default_library=static']
)

cargo_bin  = find_program('cargo')
cargo_opt  = [ '--manifest-path', meson.project_source_root() / 'gpud' / 'Cargo.toml' ]
cargo_opt += [ '--target-dir', meson.project_build_root() / 'gpud' / 'src' ]
cargo_env  = [ 'CARGO_HOME=' + meson.project_build_root() / 'cargo-home' ]
cargo_env  = [ 'BUILD_ROOT=' + meson.project_build_root() ]

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
  output: 'gpud',
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    'env', cargo_env,
    cargo_bin, 'build',
    cargo_opt, '&&', 'cp', 'gpud' / 'src' / rust_target / meson.project_name() + '_gpud', '@OUTPUT@',
  ]
)
