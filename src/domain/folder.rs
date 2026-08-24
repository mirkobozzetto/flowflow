use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
    // Space plane (proposal 0002). `space_id` marks a folder mirrored from a
    // collaborative space; `mode` is the mode it DECLARES there. The right to
    // write is the effective mode, resolved by walking the ancestor chain, so
    // never read `mode` on its own to decide anything.
    #[serde(default)]
    pub space_id: Option<String>,
    #[serde(default)]
    pub remote_id: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    pub created_at: String,
    pub modified_at: String,
}

pub struct NewFolder {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
}

pub struct UpdateFolder {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<Option<String>>,
}

/// Flatten a folder forest into display order: roots then children (each level
/// sorted by name, case-insensitive), tagged with depth for indentation.
pub fn flatten_tree(folders: &[Folder]) -> Vec<(Folder, u32)> {
    fn push_level(
        folders: &[Folder],
        parent: Option<&str>,
        depth: u32,
        out: &mut Vec<(Folder, u32)>,
    ) {
        let mut level: Vec<&Folder> = folders
            .iter()
            .filter(|f| f.parent_id.as_deref() == parent)
            .collect();
        level.sort_by_key(|f| f.name.to_lowercase());
        for f in level {
            out.push((f.clone(), depth));
            push_level(folders, Some(&f.id), depth + 1, out);
        }
    }
    let mut out = Vec::with_capacity(folders.len());
    push_level(folders, None, 0, &mut out);
    // Orphans (parent deleted mid-sync) still need to show up somewhere: append at root level.
    for f in folders {
        if !out.iter().any(|(o, _)| o.id == f.id) {
            out.push((f.clone(), 0));
        }
    }
    out
}

/// Ids of `folder_id` and every folder below it. Used to forbid moving a folder
/// into itself or one of its descendants (would detach the subtree into a cycle).
pub fn subtree_ids(folders: &[Folder], folder_id: &str) -> Vec<String> {
    let mut out = vec![folder_id.to_string()];
    let mut i = 0;
    while i < out.len() {
        let current = out[i].clone();
        for f in folders {
            if f.parent_id.as_deref() == Some(current.as_str())
                && !out.contains(&f.id)
            {
                out.push(f.id.clone());
            }
        }
        i += 1;
    }
    out
}
