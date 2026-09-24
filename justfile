migrate name:
    atlas migrate diff "{{name}}" --dir "file://db/migrations" --to "file://db/schema.sql" --dev-url "sqlite://file?mode=memory&_fk=1"

link:
    mkdir -p .agents/skills
    ln -sfn ../../skills/banker .agents/skills/banker
    mkdir -p .claude/skills
    ln -sfn ../../skills/banker .claude/skills/banker