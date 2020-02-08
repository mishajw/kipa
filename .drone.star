# To test yaml render before commit:
#   drone starlark convert --stdout --format
#
# To run CI pipeline locally:
#   drone starlark convert --stdout --format > .drone.yml
#   drone exec --volume /var/run/docker.sock:/var/run/docker.sock

image = "rust:slim-buster"
defcmd = "cargo check"
# TODO rename
defdeps = "clang make automake libc-dev libclang-dev pkg-config gnupg protobuf-compiler libgmp-dev nettle-dev"

def main(ctx):
  return [
    {
      "kind": "pipeline",
      "name": "cargo",
      "steps": [
        {
          "name": "install-deps",
          "image": image,
          "commands": [
            "apt-get -qq update",
            "apt-get -qq install %s" % defdeps,
          ],
        },
        {
          "name": "build",
          "image": image,
          "commands": ["cargo build"],
          "depends_on": ["install-deps"],
        },
        {
          "name": "test",
          "image": image,
          "commands": ["cargo test"],
          "depends_on": ["build"],
        },
        {
          "name": "check-all",
          "image": image,
          "env": {"RUSTFLAGS": "-D warnings"},
          "commands": ["cargo check --all --target-dir check-all-target"],
          "depends_on": ["install-deps"],
        },
        {
          "name": "features-blackhole",
          "image": image,
          "commands": cargo_check_features(
            "blackhole",
            "use-protobuf use-tcp use-unix-socket use-black-hole"
          ),
          "depends_on": ["install-deps"],
        },
        {
          "name": "features-random-response",
          "image": image,
          "commands": cargo_check_features(
            "blackhole",
            "use-protobuf use-tcp use-unix-socket use-random-response"
          ),
          "depends_on": ["install-deps"],
        },
      ]
    },

    # cargo("format", cmd="cargo fmt -- --check", pre=["rustup component add rustfmt", "cargo build"]),
    # cargo("test", cmd="cargo test"),
    # # TODO(mishajw) fix tests and re-add --tests
    # cargo("all", "--all", env={"RUSTFLAGS": "-D warnings"}),
    # cargo("graph", feat="use-protobuf use-tcp use-unix-socket use-graph"),
    # cargo("blackhole", feat="use-protobuf use-tcp use-unix-socket use-black-hole"),
    # cargo("randomresp", feat="use-protobuf use-tcp use-unix-socket use-random-response"),
    # TODO: Enable clippy checks after fixing all issues.
    # TODO: Run python end-to-end tests.
    # {
    #   "kind": "pipeline",
    #   "name": "docker-publish",
    #   "steps": [
    #     {
    #       "name": "build",
    #       "image": "spritsail/docker-build",
    #       "pull": "always",
    #       "settings": {
    #         "repo": "kipa",
    #       },
    #     },
    #     publish_step(
    #       "publish-branch",
    #       [ctx.build.branch],
    #       {"event": ["push"]}
    #     ),
    #     publish_step(
    #       "publish-tag",
    #       [get_tag(ctx) + " | %rempre v | %auto 2", "latest"],
    #       {"event": ["tag"]}
    #     ),
    #   ]
    # },
    # cargo(
    #   "publish",
    #   cmd="cargo publish --allow-dirty",
    #   pre=[
    #     # Run cargo build to generate the protobuf source.
    #     "cargo build",
    #     # Remove build.rs so that protobuf source isn't generated on docs.rs.
    #     "echo 'fn main() {}' > build.rs",
    #     # Remove generated protobuf source from .gitignore so it's included in release.
    #     "sed 's/.*proto_api.rs$//g' .gitignore -i",
    #     "cargo login $CARGO_SECRET",
    #   ],
    #   env={"CARGO_SECRET": {"from_secret": "cargo_secret"}},
    #   when={"event": ["tag"]},
    # ),
  ]

def cargo_check_features(name, features):
  return [
    "cargo check --target-dir %s-target --no-default-features --features '%s'" % (name, features)
  ]

# name      string, name of the pipeline. must be valid yaml word with no breaks
# args      string, arguments for provided cmd
# deps      string, list of packages to install with the package manager
# pre       list,   commands to run after installing packages, before cargo cmd
# feat      string, features to use instead of default in cargo cmd
# env       dict,   environment variables to pass to the container
# when      dict,   when clause for the pipeline's step.
def cargo(name, args="", cmd=None, deps=defdeps, pre=[], feat=None, env=None, when=None):
  step = {
    "name": "build-%s" % name,
    "image": "rust:slim-buster",    # Because rust is broken on musl at the moment:
    "commands": [],                 # https://github.com/rust-lang/rust/issues/40174
  }

  if env:
    step["environment"] = env
  if deps:
    step["commands"].insert(0, "apt-get -qq update")
    step["commands"].insert(1, "apt-get -qq install %s" % deps)
  if pre:
    step["commands"] += pre

  pipelinename = "cargo-%s" % name
  if not cmd:
    pipelinename = "cargo-build-%s" % name
    cmd = defcmd

  if feat:
    cmd += " --no-default-features --features='%s'" % feat
  if args != "":
    cmd += " " + args

  step["commands"].append(cmd)

  if when != None:
    step["when"] = when

  return {
    "kind": "pipeline",
    "name": pipelinename,
    "platform": {
      "os": "linux",
    },
    "steps": [step],
  }

# Step for publishing a built docker image.
#
# name      string, name of the pipeline.
# tags      list,   list of tag commands for docker-publish.
# when      dict,   when block for the pipeline.
def publish_step(name, tags, when):
  return {
    "name": name,
    "image": "spritsail/docker-publish",
    "pull": "always",
    "settings": {
      "from": "kipa",
      "repo": "mishajw/kipa",
      "tags": tags
      },
    "environment": {
      "DOCKER_USERNAME": {
        "from_secret": "docker_username",
        },
      "DOCKER_PASSWORD": {
        "from_secret": "docker_password",
        },
      },
    "when": when,
  }

# Extract the tag from the context's git ref.
#
# ctx       dict, the drone context.
def get_tag(ctx):
  ref = ctx.build.ref.split("/")
  ref_type = ref[1]
  ref_name = ref[2]

  if ref_type != "tags":
    return "no tag"
  return ref_name

# vim: ft=python sw=2
