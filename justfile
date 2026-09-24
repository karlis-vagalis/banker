placeholder_version := "0.0.0"

migrate name:
    atlas migrate diff "{{name}}" --dir "file://db/migrations" --to "file://db/schema.sql" --dev-url "sqlite://file?mode=memory&_fk=1"

link:
    mkdir -p .agents/skills
    ln -sfn ../../skills/banker .agents/skills/banker
    mkdir -p .claude/skills
    ln -sfn ../../skills/banker .claude/skills

# Replace the package version temporarily (used by the tagged-release workflow).
set-version version:
    @sed -i.bak 's/^version = "{{placeholder_version}}".*/version = "{{version}}"/' Cargo.toml
    @rm Cargo.toml.bak
    cargo check

# Restore the source placeholder after local versioned builds.
reset-version version:
    @sed -i.bak 's/^version = "{{version}}".*/version = "{{placeholder_version}}"/' Cargo.toml
    @rm Cargo.toml.bak
    cargo check

# Print the next patch version derived from Git tags (requires doxxer).
next-version:
    @doxxer next patch

# Build a development binary with the next Git-derived version, then restore 0.0.0.
build version=`doxxer next patch`:
    just set-version "{{version}}"
    cargo build
    just reset-version "{{version}}"
