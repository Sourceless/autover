use std::collections::HashMap;
use git2::{Repository, Commit, Oid, Note};
use semver::{Version, Identifier};

enum VersionCmd {
    IncMajor,
    IncMinor,
    IncPatch,
    SetVersion(String),
    SetPrereleaseLabel(String)
}

fn main() {
    let repo = match Repository::discover(".") {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open git repository: {}", e),
    };

    let head = repo.head().unwrap();
    let name = head.name().unwrap();
    let head_tree_id = head.peel_to_tree().unwrap().id();

    let notes_ref = repo.note_default_ref().unwrap();
    let notes = repo.notes(Some(&notes_ref));
    let mut commit_to_node_map = HashMap::<Oid, Note>::new();

    for note in notes.unwrap() {
        let (note_id, commit_id) = note.unwrap();
        let note_obj = repo.find_note(Some(&notes_ref), commit_id).unwrap();
        commit_to_node_map.insert(commit_id, note_obj);
    }

    let mut cmd_stack = Vec::<VersionCmd>::new();
    let mut top_commit = Some(head.peel_to_commit().expect(&format!("Couldn't find a commit on ref {}", name)[..]));

    while let Some(commit) = top_commit.clone() {
        if commit.parents().count() > 1 {
            // This is merge commit, so increment revision
            cmd_stack.push(VersionCmd::IncPatch);
        } else {
            if let Ok(note) = repo.find_note(Some(&notes_ref), commit.id()) {
                println!("{:?}", note);
            }
        }

        top_commit = get_ancestor(commit, &head_tree_id).clone();
    }

    let version = calculate_version(&mut cmd_stack);
    println!("{}", version);
}

fn calculate_version(cmd_stack: &mut Vec<VersionCmd>) -> Version {
    let mut version = Version::new(0, 0, 0);
    while let Some(cmd) = cmd_stack.pop() {
        match cmd {
            VersionCmd::IncMajor => version.increment_major(),
            VersionCmd::IncMinor => version.increment_minor(),
            VersionCmd::IncPatch => version.increment_patch(),
            VersionCmd::SetPrereleaseLabel(label) => version.pre = Vec::from([Identifier::AlphaNumeric(label)]),
            VersionCmd::SetVersion(version_str) => version = Version::parse(&version_str.as_str()).expect("Could not parse version")
        }
    }

   version
}

fn get_ancestor<'a>(commit: Commit<'a>, tree_id: &Oid) -> Option<Commit<'a>> {
    let parents = commit.parents();

    for parent in parents {
        if parent.tree_id() == *tree_id {
            return Some(parent.clone());
        }
    }

    return None;
}
