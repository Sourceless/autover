use git2::Repository;
use lazy_static::lazy_static;
use regex::Regex;
use semver::{Identifier, Version};
use clap::{Arg, App, ArgMatches};
use std::process;

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
        .version("0")
        .author("Laurence Pakenham-Smith <laurence@sourceless.org")
        .about("Automatic calculatable versions")
        .arg(Arg::with_name("count-patch")
             .short("c")
             .long("count-patch")
             .value_name("COUNT_METHOD")
             .help("Choose the counting method from merge (default), commit, or manual.")
             .takes_value(true))
        .subcommand(App::new("major")
                    .about("Increments the major version"))
        .subcommand(App::new("minor")
                    .about("Increments the minor version"))
        .subcommand(App::new("patch")
                    .about("Increments the patch version (manual COUNT_METHOD only)"))
        .subcommand(App::new("tag")
                    .about("Set or clear the prerelease tag")
                    .arg(Arg::with_name("NAME")
                         .help("Optional tag name. Leave blank to clear.")
                         .index(1)))
        .subcommand(App::new("clear")
                    .about("Clear the current commit of any manual version changes"))
        .get_matches();

    match app(matches) {
        Ok(_) => (),
        Err(code) => process::exit(code)
    }
}

fn app(matches: ArgMatches) -> Result<(), i32> {
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

    let repo = match Repository::discover(".") {
        Ok(repo) => repo,
        Err(e) => {
            eprintln!("Failed to open git repository: {}", e);
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

fn get_version(repo: &Repository, count_method: &CountMethod) -> Version {
    let head = repo.head().unwrap();
    let name = head.name().unwrap();
    let notes_ref = repo.note_default_ref().unwrap();

    let mut cmd_stack = Vec::<VersionCmd>::new();
    let mut top_commit = Some(
        head.peel_to_commit()
            .expect(&format!("Couldn't find a commit on ref {}", name)[..]),
    );

    // Traverse over every commit and note any place where we need to revise the version
    while let Some(commit) = top_commit {
        let num_parents = commit.parents().count();

        if *count_method == CountMethod::Commit && num_parents == 1 {
            cmd_stack.push(VersionCmd::IncPatch);
        }

        if *count_method == CountMethod::Merge && num_parents > 1 {
            cmd_stack.push(VersionCmd::IncPatch);
        } else {
            if let Ok(note) = repo.find_note(Some(&notes_ref), commit.id()) {
                if let Some(message) = note.message() {
                    if let Some(command) = match_message_to_cmd(&count_method, &message) {
                        cmd_stack.push(command);
                    }
                }
            }
        }

        top_commit = commit.parents().next();
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
    } else if let Some(version_str) = SET_VERSION_MATCHER.find(message) {
        return Some(VersionCmd::SetVersion(String::from(version_str.as_str())));
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
