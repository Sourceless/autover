use git2::Repository;
use lazy_static::lazy_static;
use regex::Regex;
use semver::{Identifier, Version};
use clap::{Arg, App, ArgMatches};
use std::process;
use std::io::{self, Write};

#[derive(PartialEq, Debug)]
enum VersionCmd {
    IncMajor,
    IncMinor,
    IncPatch,
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

fn get_version(repo: &Repository, count_method: &CountMethod) -> Version {
    match repo.head() {
        Ok(head) => head,
        Err(_) => {
            eprintln!("HEAD has no commits - make sure there is at least 1 commit");
            process::exit(1);
        }
    };

    let notes_ref = repo.note_default_ref().unwrap();
    let mut cmd_stack = Vec::<VersionCmd>::new();
    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_head().unwrap();

    // We need to track if a patch has occurred on the previous commit,
    // if it has we need to delete it because it will cause an off by one error.
    let mut patch_flag = false;

    // Traverse over every commit and note any place where we need to revise the version
    for commit in revwalk {
        let commit = commit.unwrap();
        let commit = repo.find_commit(commit).unwrap();
        let num_parents = commit.parents().count();

        if let Ok(note) = repo.find_note(Some(&notes_ref), commit.id()) {
            if let Some(message) = note.message() {
                if let Some(command) = match_message_to_cmd(&count_method, &message) {
                    // TODO: find a better approach than this.

                    // There's a little complexity in here because there
                    // are some edge cases for merge counting and patches
                    // that can cause some off by one errors.

                    // If we are incrementing minor or major just before a merge commit,
                    // then we do not want to apply a patch for that merge commit.
                    if *count_method == CountMethod::Merge && patch_flag && (command == VersionCmd::IncMinor || command == VersionCmd::IncMajor) {
                        cmd_stack.pop();
                    }

                    // If the command is a manual patch inc, we should leave the patch_flag
                    // untouched, otherwise the behaviour is strange with the other patch methods
                    if *count_method == CountMethod::Merge && command != VersionCmd::IncPatch {
                        patch_flag = false;
                    }

                    cmd_stack.push(command);
                }
            }
        }

        if *count_method == CountMethod::Commit && num_parents == 1 {
            println!("Inc patch");
            cmd_stack.push(VersionCmd::IncPatch);
            patch_flag = true;
            continue;
        }

        if *count_method == CountMethod::Merge && num_parents > 1 {
            println!("Inc patch");
            cmd_stack.push(VersionCmd::IncPatch);
            patch_flag = true;
            continue;
        }
    }

    calculate_version(&mut cmd_stack)
}

lazy_static! {
    static ref SET_VERSION_MATCHER: Regex = Regex::new(
        r"autover-set-version ([0-9]+\.[0-9]+\.[0-9]+(?:-(?:[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?)"
    )
    .unwrap();
    static ref SET_PRERELEASE_LABEL_MATCHER: Regex =
        Regex::new(r"autover-set-prerelease-label ([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)").unwrap();
}

fn match_message_to_cmd(count_method: &CountMethod, message: &str) -> Option<VersionCmd> {
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
    } else if message.contains("autover-inc-patch") && *count_method == CountMethod::Manual {
        return Some(VersionCmd::IncPatch)
    }

    None
}

fn calculate_version(cmd_stack: &mut Vec<VersionCmd>) -> Version {
    let mut version = Version::new(0, 0, 0);
    while let Some(cmd) = cmd_stack.pop() {
        match cmd {
            VersionCmd::IncMajor => version.increment_major(),
            VersionCmd::IncMinor => version.increment_minor(),
            VersionCmd::IncPatch => version.increment_patch(),
            VersionCmd::SetPrereleaseLabel(label) => {
                version.pre = Vec::from([Identifier::AlphaNumeric(label)])
            }
            VersionCmd::SetVersion(version_str) => {
                version = Version::parse(&version_str.as_str()).expect("Could not parse version")
            }
            VersionCmd::ClearPrereleaseLabel => version.pre = Vec::<Identifier>::new(),
        }
    }

    version
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;

    fn expect_version_for_repo(expected_version: &str, repo_path: &str, count_method: &CountMethod) {
        let repo = Repository::discover(repo_path).unwrap();
        assert_eq!(Version::parse(expected_version).unwrap(),
                   get_version(&repo, &count_method));
    }

    macro_rules! repo_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (version, repo, count_method) = $value;
                expect_version_for_repo(version,
                                        repo,
                                        &count_method);
            }
        )*
        }
    }

    repo_tests! {
        single_commit_merge: ("0.0.0", "./test/single_commit", CountMethod::Merge),
        single_commit_commit: ("0.0.0", "./test/single_commit", CountMethod::Commit),
        single_commit_manual: ("0.0.0", "./test/single_commit", CountMethod::Manual),
        two_commits_merge: ("0.0.0", "./test/two_commits", CountMethod::Merge),
        two_commits_commit: ("0.0.1", "./test/two_commits", CountMethod::Commit),
        two_commits_manual: ("0.0.0", "./test/two_commits", CountMethod::Manual),
        merge_commit_merge: ("0.0.1", "./test/merge_commit", CountMethod::Merge),
        merge_commit_commit: ("0.0.1", "./test/merge_commit", CountMethod::Commit),
        merge_commit_manual: ("0.0.0", "./test/merge_commit", CountMethod::Manual),
        updated_from_master_merge: ("0.0.2", "./test/updated_from_master", CountMethod::Merge),
        updated_from_master_commit: ("0.0.3", "./test/updated_from_master", CountMethod::Commit),
        updated_from_master_manual: ("0.0.0", "./test/updated_from_master", CountMethod::Manual),
        minor_update_merge: ("0.1.0", "./test/minor_update", CountMethod::Merge),
        minor_update_commit: ("0.1.0", "./test/minor_update", CountMethod::Commit),
        minor_update_manual: ("0.1.0", "./test/minor_update", CountMethod::Manual),
        major_update_merge: ("1.0.0", "./test/major_update", CountMethod::Merge),
        major_update_commit: ("1.0.0", "./test/major_update", CountMethod::Commit),
        major_update_manual: ("1.0.0", "./test/major_update", CountMethod::Manual),
        manual_patch_after_major_update_merge: ("1.0.1", "./test/manual_patch_after_major_update", CountMethod::Merge),
        manual_patch_after_major_update_commit: ("1.0.1", "./test/manual_patch_after_major_update", CountMethod::Commit),
        manual_patch_after_major_update_manual: ("1.0.1", "./test/manual_patch_after_major_update", CountMethod::Manual),
    }
}
