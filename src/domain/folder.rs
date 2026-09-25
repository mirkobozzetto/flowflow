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

/// One visible row in a theme navigator. Search results retain their parent
/// path even when those parents are closed in the normal tree.
#[derive(Debug, Clone, PartialEq)]
pub struct FolderNavigationRow {
    pub folder: Folder,
    pub depth: u32,
    pub path: String,
    pub has_children: bool,
}

pub fn folder_navigation_rows(
    folders: &[Folder],
    closed: &std::collections::HashSet<String>,
    query: &str,
) -> Vec<FolderNavigationRow> {
    let query = query.trim().to_lowercase();
    let parents: std::collections::HashSet<&str> = folders
        .iter()
        .filter_map(|f| f.parent_id.as_deref())
        .collect();
    let mut ancestors: Vec<(String, String)> = Vec::new();
    let mut rows = Vec::new();
    for (folder, depth) in flatten_tree(folders) {
        ancestors.truncate(depth as usize);
        let hidden = ancestors.iter().any(|(id, _)| closed.contains(id));
        let path = ancestors
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>()
            .join(" › ");
        ancestors.push((folder.id.clone(), folder.name.clone()));
        let visible = if query.is_empty() {
            !hidden
        } else {
            let name = folder.name.to_lowercase();
            query.split_whitespace().all(|word| name.contains(word))
        };
        if visible {
            rows.push(FolderNavigationRow {
                has_children: parents.contains(folder.id.as_str()),
                folder,
                depth: if query.is_empty() { depth } else { 0 },
                path,
            });
        }
    }
    rows
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
