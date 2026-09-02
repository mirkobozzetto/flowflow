#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
source_dir="$repo_root/skills/flowflow-spaces"
hermes_home="${HERMES_HOME:-$HOME/.hermes}"
target_root="$hermes_home/skills/productivity"
backup_root="$hermes_home/backups/skills/productivity"
target_dir="$target_root/flowflow-spaces"

if [ ! -f "$source_dir/SKILL.md" ]; then
  printf '%s\n' "FlowFlow skill source not found: $source_dir" >&2
  exit 1
fi

mkdir -p "$target_root" "$backup_root"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)-$$"

# Hermes discovers every SKILL.md below skills, including backup directories.
for legacy_backup in "$target_dir".backup*; do
  [ -e "$legacy_backup" ] || continue
  legacy_name=${legacy_backup##*/}
  destination="$backup_root/$legacy_name"
  if [ -e "$destination" ]; then
    destination="$destination.$timestamp"
  fi
  mv "$legacy_backup" "$destination"
  printf '%s\n' "Moved legacy backup to $destination"
done

if [ -e "$target_dir" ]; then
  backup_dir="$backup_root/flowflow-spaces.backup.$timestamp"
  mv "$target_dir" "$backup_dir"
  printf '%s\n' "Backed up existing skill to $backup_dir"
fi
cp -R "$source_dir" "$target_dir"
printf '%s\n' "Installed FlowFlow skill at $target_dir"
printf '%s\n' "Reload Hermes skills or restart Hermes before using it."
