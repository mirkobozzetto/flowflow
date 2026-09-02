#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
source_dir="$repo_root/skills/flowflow-spaces"
target_root="${HERMES_HOME:-$HOME/.hermes}/skills/productivity"
target_dir="$target_root/flowflow-spaces"

if [ ! -f "$source_dir/SKILL.md" ]; then
  printf '%s\n' "FlowFlow skill source not found: $source_dir" >&2
  exit 1
fi

mkdir -p "$target_root"

if [ -e "$target_dir" ]; then
  backup_dir="$target_dir.backup.$(date -u +%Y%m%dT%H%M%SZ)"
  mv "$target_dir" "$backup_dir"
  printf '%s\n' "Backed up existing skill to $backup_dir"
fi

cp -R "$source_dir" "$target_dir"
printf '%s\n' "Installed FlowFlow skill at $target_dir"
printf '%s\n' "Reload Hermes skills or restart Hermes before using it."
