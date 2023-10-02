pkgdatadir = join_paths(get_option('prefix'), get_option('datadir'), meson.project_name())

if get_option('flatpak') == false  
  subdir('sys_info_v2/gatherer')
endif

gnome = import('gnome')
libudev = dependency('libudev', required: true)

conf = configuration_data()
conf.set_quoted('VERSION', meson.project_version())
conf.set_quoted('GETTEXT_PACKAGE', 'missioncenter')
conf.set_quoted('LOCALEDIR', join_paths(get_option('prefix'), get_option('localedir')))
conf.set_quoted('PKGDATADIR', pkgdatadir)

configure_file(
    input: 'config.rs.in',
    output: 'config.rs',
    configuration: conf
)

cargo_bin  = find_program('cargo')
cargo_env = [ ]
cargo_opt  = [ ]
target_dir = ''

if get_option('flatpak') == false
    cargo_env += [ 'CARGO_HOME=' + meson.project_build_root()  / 'cargo-home' ]
endif

if get_option('sanitize') != 'none'
    cargo_env += [ 'RUSTFLAGS=-Zsanitizer=' + get_option('sanitize') ]
    cargo_env += [ 'CFLAGS=-fsanitize=' + get_option('sanitize') ]
    cargo_env += [ 'CXXFLAGS=-fsanitize=' + get_option('sanitize') ]
    cargo_opt += [ '-Zbuild-std', '--target=x86_64-unknown-linux-gnu' ]
    target_dir = 'x86_64-unknown-linux-gnu'
endif

cargo_opt += [ '--manifest-path', meson.project_source_root() / 'Cargo.toml' ]
cargo_opt += [ '--target-dir', meson.project_build_root()  / 'src' ]
cargo_env += [ 'BUILD_ROOT=' + meson.project_build_root() ]

if get_option('buildtype') == 'release'
  cargo_opt += [ '--release' ]
  rust_target = 'release'
else
  rust_target = 'debug'
endif

cargo_cmd = [
  'env', cargo_env, cargo_bin
]

if get_option('sanitize') != 'none'
    cargo_cmd += [ '+nightly' ]
endif

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: meson.project_name(),
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    'env', cargo_env,
    cargo_cmd, 'build', cargo_opt, '&&',
    'cp', 'src' / target_dir / rust_target / meson.project_name(), '@OUTPUT@',
  ]
)
