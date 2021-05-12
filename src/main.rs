use git2::Repository;
use lazy_static::lazy_static;
use regex::Regex;
use semver::{Identifier, Version};
use clap::{Arg, App, ArgMatches};
use std::process;
use std::io::{self, Write};
use std::collections::VecDeque;

#[derive(PartialEq, Debug)]
enum VersionCmd {
    IncMajor,
    IncMinor,
    IncPatchMerge,
    IncPatchCommit,
    IncPatchManual,
    SetVersion(String),
    SetPrereleaseLabel(String),
    ClearPrereleaseLabel,
}

#[derive(PartialEq)]
enum CountMethod {
    Merge,
    Commit,
    Manual
}

fn main() {
    let matches = App::new("autover")
        .author("Laurence Pakenham-Smith <laurence@sourceless.org>")
        .about("Automatic calculatable versions")
        .arg(Arg::with_name("count-patch")
             .short("c")
             .long("count-patch")
             .value_name("COUNT_METHOD")
             .help("Choose the counting method from merge (default), commit, or manual.")
             .takes_value(true))
        .arg(Arg::with_name("path")
             .short("p")
             .long("path")
             .value_name("REPO_PATH")
             .help("Path to a dir that contains a .git or is a descendant of such a directory.")
             .takes_value(true))
        .subcommand(App::new("major")
                    .about("Increments the major version"))
        .subcommand(App::new("minor")
                    .about("Increments the minor version"))
        .subcommand(App::new("patch")
                    .about("Increments the patch version (manual COUNT_METHOD only)"))
        .subcommand(App::new("set")
                    .about("Override the current version")
                    .arg(Arg::with_name("SEMVER")
                         .required(true)
                         .help("Version to change to")
                         .index(1)))
        .subcommand(App::new("tag")
                    .about("Set or clear the prerelease tag")
                    .arg(Arg::with_name("NAME")
                         .help("Optional tag name. Leave blank to clear.")
                         .index(1)))
        .subcommand(App::new("clear")
                    .about("Clear the current commit of any manual version changes"))
        .subcommand(App::new("push")
                    .about("Push version changes to the remote repository")
                    .arg(Arg::with_name("REMOTE")
                         .help("Optional remote name (defaults to 'origin')")
                         .index(1)))
        .subcommand(App::new("fetch")
                    .about("Fetch version changes from the remote repository")
                    .arg(Arg::with_name("REMOTE")
                         .help("Optional remote name (defaults to 'origin')")
                         .index(1)))
        .subcommand(App::new("init")
                    .about("Set up repository to auto-push version changes")
                    .arg(Arg::with_name("REMOTE")
                         .help("Optional remote name (defaults to 'origin')")
                         .index(1)))
        .get_matches();

    match app(matches) {
        Ok(_) => (),
        Err(code) => process::exit(code)
    }
}

fn app(matches: ArgMatches) -> Result<(), i32> {
    // Select the count method, or default to merge counting
    let count_method = match matches.value_of("count-patch") {
        Some("merge") => CountMethod::Merge,
        Some("commit") => CountMethod::Commit,
        Some("manual") => CountMethod::Manual,
        Some(m) => {
            eprintln!("{} is not a valid COUNT_METHOD (merge, commit, manual)", m);
            return Err(1);
        },
        None => CountMethod::Merge
    };

    // If a path is specified, use it, otherwise default to current dir
    let repo_path = matches.value_of("path").unwrap_or(".");

    let repo = match Repository::discover(repo_path) {
        Ok(repo) => repo,
        Err(_) => {
            eprintln!("Could not find a git repository in {}", repo_path);
            return Err(1);
        },
    };

    let start_version = get_version(&repo, &count_method);

    if matches.subcommand_matches("major").is_some() {
        set_note(&"autover-inc-major");
        let new_version = get_version(&repo, &count_method);
        println!("{} -> {}", start_version, new_version);
    } else if matches.subcommand_matches("minor").is_some() {
        set_note(&"autover-inc-minor");
        let new_version = get_version(&repo, &count_method);
        println!("{} -> {}", start_version, new_version);
    } else if matches.subcommand_matches("patch").is_some() {
        set_note(&"autover-inc-patch");
        let new_version = get_version(&repo, &count_method);
        println!("{} -> {}", start_version, new_version);
    } else if let Some(matches) = matches.subcommand_matches("tag") {
        if let Some(name) = matches.value_of("NAME") {
            set_note(&format!("{} {}", "autover-set-prerelease-label", name));
        } else {
            set_note(&"autover-clear-prerelease-label");
        }
        let new_version = get_version(&repo, &count_method);
        println!("{} -> {}", start_version, new_version);
    } else if matches.subcommand_matches("clear").is_some() {
        clear_note();
        let new_version = get_version(&repo, &count_method);
        println!("{} -> {}", start_version, new_version);
    } else if let Some(matches) = matches.subcommand_matches("push") {
        let mut remote = "origin";
        if let Some(remote_arg) = matches.value_of("REMOTE") {
            remote = remote_arg
        }
        push(&remote);
        println!("Remote {} is now at {}", &remote, start_version);
    } else if let Some(matches) = matches.subcommand_matches("fetch") {
        let mut remote = "origin";
        if let Some(remote_arg) = matches.value_of("REMOTE") {
            remote = remote_arg
        }
        fetch(&remote);
        let new_version = get_version(&repo, &count_method);
        println!("Local repo is now at {} (from {})", new_version, &remote);
    } else if let Some(matches) = matches.subcommand_matches("init") {
        let mut remote = "origin";
        if let Some(remote_arg) = matches.value_of("REMOTE") {
            remote = remote_arg
        }
        init(&repo, &remote);
    } else if let Some(matches) = matches.subcommand_matches("set") {
        let version_str = matches.value_of("SEMVER").expect("SEMVER value required");
        if let Ok(_) = Version::parse(version_str) {
            set_note(&format!("autover-set-version {}", version_str).as_str());
            let new_version = get_version(&repo, &count_method);
            println!("{} -> {}", start_version, new_version);
        } else {
            eprintln!("{} is not a valid semver string", version_str);
            return Err(1);
        }
    } else {
        println!("{}", start_version);
    }

    Ok(())
}

fn set_note(message: &str) {
    // TODO: use git2 instead of calling out here
    // TODO: maybe don't destroy existing notes?
    process::Command::new("git")
        .args(&["notes", "add", "-f", "-m", &message])
        .output()
        .expect("git failed to execute");
}

fn clear_note() {
    // TODO: use git2 instead of calling out here
    // TODO: maybe don't destroy existing notes?
    process::Command::new("git")
        .args(&["notes", "remove"])
        .output()
        .expect("git failed to execute");
}

fn push(remote: &str) {
    let output = process::Command::new("git")
        .args(&["push", &remote, "refs/notes/*"])
        .output()
        .expect("git failed to execute");

    println!("Pushing version to {}", remote);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
}

fn fetch(remote: &str) {
    let output = process::Command::new("git")
        .args(&["fetch", &remote, "refs/notes/*:refs/notes/*"])
        .output()
        .expect("git failed to execute");

    println!("Fetching version information from {}", remote);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
}

fn init(repo: &Repository, remote: &str) {
    repo.remote_add_fetch(&remote, &"refs/notes/*:refs/notes/*").expect("argh");
    repo.remote_add_push(&remote, &"refs/notes/*").expect("argh");
}

// TODO: needs a full rewrite
fn get_version(repo: &Repository, count_method: &CountMethod) -> Version {
    match repo.head() {
        Ok(head) => head,
        Err(_) => {
            eprintln!("HEAD has no commits - make sure there is at least 1 commit");
            process::exit(1);
        }
    };

    let notes_ref = repo.note_default_ref().unwrap();
    let mut cmds = VecDeque::<VersionCmd>::new();
    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_head().unwrap();
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL).unwrap();

    // Traverse over every commit and note any place where we need to revise the version
    for commit in revwalk {
        let commit = commit.unwrap();
        let commit = repo.find_commit(commit).unwrap();
        let num_parents = commit.parents().count();

        println!("{:?}", commit);

        if let Ok(note) = repo.find_note(Some(&notes_ref), commit.id()) {
            if let Some(message) = note.message() {
                if let Some(command) = match_message_to_cmd(&message) {
                    cmds.push_back(command);
                }
            }
        } else if num_parents == 1 {
            cmds.push_back(VersionCmd::IncPatchCommit);
        } else if num_parents > 1 {
            cmds.push_back(VersionCmd::IncPatchMerge);
        }

        println!("{:?}", cmds);
    }

    calculate_version(&mut cmds, &count_method)
}

lazy_static! {
    static ref SET_VERSION_MATCHER: Regex = Regex::new(
        r"autover-set-version ([0-9]+\.[0-9]+\.[0-9]+(?:-(?:[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?)"
    )
    .unwrap();
    static ref SET_PRERELEASE_LABEL_MATCHER: Regex =
        Regex::new(r"autover-set-prerelease-label ([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)").unwrap();
}

fn match_message_to_cmd(message: &str) -> Option<VersionCmd> {
    if message.contains("autover-inc-major") {
        return Some(VersionCmd::IncMajor);
    } else if message.contains("autover-inc-minor") {
        return Some(VersionCmd::IncMinor);
    } else if let Some(captures) = SET_VERSION_MATCHER.captures(message) {
        return Some(VersionCmd::SetVersion(String::from(&captures[1])));
    } else if let Some(captures) = SET_PRERELEASE_LABEL_MATCHER.captures(message) {
            let prerelease_label = &captures[1];
        return Some(VersionCmd::SetPrereleaseLabel(String::from(
            prerelease_label,
        )));
    } else if message.contains("autover-clear-prerelease-label") {
        return Some(VersionCmd::ClearPrereleaseLabel);
    } else if message.contains("autover-inc-patch") {
        return Some(VersionCmd::IncPatchManual)
    }

    None
}

fn calculate_version(cmd_stack: &mut VecDeque<VersionCmd>, count_method: &CountMethod) -> Version {
    let mut version = Version::new(0, 0, 0);
    println!("{:}", version);
    let mut skip_next_patch_merge = false;
    while let Some(cmd) = cmd_stack.pop_back() {
        print!("\n{:?}", cmd);
        match cmd {
            VersionCmd::IncMajor => {
                skip_next_patch_merge = true;
                version.increment_major();
            },
            VersionCmd::IncMinor => {
                skip_next_patch_merge = true;
                version.increment_minor()
            },
            VersionCmd::IncPatchMerge => {
                if skip_next_patch_merge {
                    skip_next_patch_merge = false;
                } else if count_method == &CountMethod::Merge {
                    version.increment_patch()
                }
            },
            VersionCmd::IncPatchCommit => {
                if count_method == &CountMethod::Commit {
                    version.increment_patch()
                }
            },
            VersionCmd::IncPatchManual => {
                if count_method == &CountMethod::Manual || count_method == &CountMethod::Commit {
                    version.increment_patch()
                }
            },
            VersionCmd::SetPrereleaseLabel(label) => {
                version.pre = Vec::from([Identifier::AlphaNumeric(label)])
            }
            VersionCmd::SetVersion(version_str) => {
                version = Version::parse(&version_str.as_str()).expect("Could not parse version")
            }
            VersionCmd::ClearPrereleaseLabel => version.pre = Vec::<Identifier>::new(),
        }
        print!(" -> {:}\n", version);
    }

    version
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;
    use tempfile::{tempdir, TempDir};
    use std::process::Command;
    use std::path::Path;

    fn expect_version_for_repo(expected_version: &str, repo_path: &str, count_method: &CountMethod) {
        let repo = Repository::discover(repo_path).unwrap();
        assert_eq!(Version::parse(expected_version).unwrap(),
                   get_version(&repo, &count_method));
    }

    fn setup_repo() -> (TempDir, Repository) {
        // Create a repo and give it an initial commit
        let dir = tempdir().unwrap();
        let repo = Repository::init(&dir).unwrap();
        let path = &dir.path();

        Command::new("git")
            .current_dir(path)
            .args(&["commit", "--allow-empty", "-m", "initial commit"])
            .output()
            .expect("Git command failed");

        return (dir, repo);
    }

    fn write_empty_commit(path: &Path, message: &str) {
        Command::new("git")
            .current_dir(path)
            .args(&["commit", "--allow-empty", "-m", message])
            .output()
            .expect("Git command failed");
    }

    fn new_branch(path: &Path, branch: &str) {
        Command::new("git")
            .current_dir(path)
            .args(&["checkout", "-b", branch])
            .output()
            .expect("Git command failed");
    }

    fn use_branch(path: &Path, branch: &str) {
        Command::new("git")
            .current_dir(path)
            .args(&["checkout", branch])
            .output()
            .expect("Git command failed");
    }

    fn merge_branch(path: &Path, branch: &str) {
        Command::new("git")
            .current_dir(path)
            .args(&["merge", "--no-ff", branch])
            .output()
            .expect("Git command failed");
    }

    fn add_note(path: &Path, note: &str) {
        Command::new("git")
            .current_dir(path)
            .args(&["notes", "add", "-m", note])
            .output()
            .expect("Git command failed");
    }

    // Functions to set up test envs
    fn single_commit(_: &Path) {}

    fn two_commits(path: &Path) {
        write_empty_commit(path, "second commit");
    }

    fn merge_commit(path: &Path) {
        new_branch(path, "other_branch");
        write_empty_commit(path, "commit on other_branch");
        use_branch(path, "master");
        merge_branch(path, "other_branch");
    }

    fn updated_from_master(path: &Path) {
        new_branch(path, "other_branch");
        write_empty_commit(path, "a");
        use_branch(path, "master");
        write_empty_commit(path, "b");
        use_branch(path, "other_branch");
        merge_branch(path, "master");
        write_empty_commit(path, "c");
        use_branch(path, "master");
        merge_branch(path, "other_branch");
    }

    fn minor_update(path: &Path) {
        new_branch(path, "feature-branch-1");
        write_empty_commit(path, "a");
        use_branch(path, "master");
        merge_branch(path, "feature-branch-1");
        new_branch(path, "feature-branch-2-minor");
        write_empty_commit(path, "minor increment");
        add_note(path, "autover-inc-minor");
        use_branch(path, "master");
        merge_branch(path, "feature-branch-2-minor");
    }

    fn major_update(path: &Path) {
        new_branch(path, "feature-branch-1");
        write_empty_commit(path, "a");
        use_branch(path, "master");
        merge_branch(path, "feature-branch-1");
        new_branch(path, "feature-branch-2-minor");
        write_empty_commit(path, "minor increment");
        add_note(path, "autover-inc-minor");
        use_branch(path, "master");
        merge_branch(path, "feature-branch-2-minor");
        new_branch(path, "feature-branch-3-major");
        write_empty_commit(path, "major increment");
        add_note(path, "autover-inc-major");
        use_branch(path, "master");
        merge_branch(path, "feature-branch-3-major");
    }

    fn manual_patch_after_major_update(path: &Path) {
        new_branch(path, "feature-branch-1");
        write_empty_commit(path, "a");
        use_branch(path, "master");
        merge_branch(path, "feature-branch-1");
        new_branch(path, "feature-branch-2-minor");
        write_empty_commit(path, "minor increment");
        add_note(path, "autover-inc-minor");
        use_branch(path, "master");
        merge_branch(path, "feature-branch-2-minor");
        new_branch(path, "feature-branch-3-major");
        write_empty_commit(path, "major increment");
        add_note(path, "autover-inc-major");
        use_branch(path, "master");
        merge_branch(path, "feature-branch-3-major");
        new_branch(path, "feature-branch-4-manual-patch");
        write_empty_commit(path, "manual patch");
        add_note(path, "autover-inc-patch");
        use_branch(path, "master");
        merge_branch(path, "feature-branch-4-manual-patch");
    }

    macro_rules! repo_tests {
        ($($name:ident: $setup_fn:ident, $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (dir, repo) = setup_repo();
                let repo_path = repo.path().to_str().unwrap();
                let (version, count_method) = $value;
                $setup_fn(&dir.path());
                expect_version_for_repo(version,
                                        &repo_path,
                                        &count_method);
                dir.close().unwrap();
            }
        )*
        }
    }

    repo_tests! {
        single_commit_merge: single_commit, ("0.0.0", CountMethod::Merge),
        single_commit_commit: single_commit, ("0.0.0", CountMethod::Commit),
        single_commit_manual: single_commit, ("0.0.0", CountMethod::Manual),
        two_commits_merge: two_commits, ("0.0.0", CountMethod::Merge),
        two_commits_commit: two_commits, ("0.0.1", CountMethod::Commit),
        two_commits_manual: two_commits, ("0.0.0", CountMethod::Manual),
        merge_commit_merge: merge_commit, ("0.0.1", CountMethod::Merge),
        merge_commit_commit: merge_commit, ("0.0.1", CountMethod::Commit),
        merge_commit_manual: merge_commit, ("0.0.0", CountMethod::Manual),
        updated_from_master_merge: updated_from_master, ("0.0.2", CountMethod::Merge),
        updated_from_master_commit: updated_from_master, ("0.0.3", CountMethod::Commit),
        updated_from_master_manual: updated_from_master, ("0.0.0", CountMethod::Manual),
        minor_update_merge: minor_update, ("0.1.0", CountMethod::Merge),
        minor_update_commit: minor_update, ("0.1.0", CountMethod::Commit),
        minor_update_manual: minor_update, ("0.1.0", CountMethod::Manual),
        major_update_merge: major_update, ("1.0.0", CountMethod::Merge),
        major_update_commit: major_update, ("1.0.0", CountMethod::Commit),
        major_update_manual: major_update, ("1.0.0", CountMethod::Manual),
        manual_patch_after_major_update_merge: manual_patch_after_major_update, ("1.0.1", CountMethod::Merge),
        manual_patch_after_major_update_commit: manual_patch_after_major_update, ("1.0.1", CountMethod::Commit),
        manual_patch_after_major_update_manual: manual_patch_after_major_update, ("1.0.1", CountMethod::Manual),
    }
}
