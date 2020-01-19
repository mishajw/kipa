# To test yaml render before commit:
#   drone starlark convert --stdout --format
#
# To run CI pipeline locally:
#   drone starlark convert --stdout --format > .drone.yml
#   drone exec --volume /var/run/docker.sock:/var/run/docker.sock

defcmd = 'cargo check'
defdeps = 'clang make automake libc-dev libclang-dev pkg-config gnupg protobuf-compiler libgmp-dev nettle-dev'

def main(ctx):
  return [
    cargo('format', cmd='cargo fmt -- --check', pre=['rustup component add rustfmt', 'cargo build']),
    cargo('test', cmd='cargo test'),
    # TODO(mishajw) fix tests and re-add --tests
    cargo('all', '--all', env={'RUSTFLAGS': '-D warnings'}),
    cargo('graph', feat='use-protobuf use-tcp use-unix-socket use-graph'),
    cargo('blackhole', feat='use-protobuf use-tcp use-unix-socket use-black-hole'),
    cargo('randomresp', feat='use-protobuf use-tcp use-unix-socket use-random-response'),
    # TODO: Enable clippy checks after fixing all issues.
    # TODO: Run python end-to-end tests.
  ]


# name      string, name of the pipeline. must be valid yaml word with no breaks
# args      string, arguments for provided cmd
# deps      string, list of packages to install with the package manager
# pre       list,   commands to run after installing packages, before cargo cmd
# feat      string, features to use instead of default in cargo cmd
# env       dict,   environment variables to pass to the container
def cargo(name, args='', cmd=None, deps=defdeps, pre=[], feat=None, env=None):
  step = {
    "name": "build-%s" % name,
    "image": "rust:slim-buster",    # Because rust is broken on musl at the moment:
    "commands": [],                 # https://github.com/rust-lang/rust/issues/40174
  }

  if env:
    step['environment'] = env
  if deps:
    step['commands'].insert(0, 'apt-get -qq update')
    step['commands'].insert(1, 'apt-get -qq install %s' % deps)
  if pre:
    step['commands'] += pre

  pipelinename = "cargo-%s" % name
  if not cmd:
    pipelinename = "cargo-build-%s" % name
    cmd = defcmd

  if feat:
    cmd += ' --no-default-features --features="%s"' % feat
  if args != '':
    cmd += ' ' + args

  step['commands'].append(cmd)

  return {
    "kind": "pipeline",
    "name": pipelinename,
    "platform": {
      "os": "linux",
    },
    "steps": [step],
  }

# vim: ft=python sw=2
