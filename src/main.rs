use git2::Repository;
use lazy_static::lazy_static;
use regex::Regex;
use semver::{Identifier, Version};
use clap::{Arg, App};

enum VersionCmd {
    IncMajor,
    IncMinor,
    IncPatch,
    SetVersion(String),
    SetPrereleaseLabel(String),
    ClearPrereleaseLabel,
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
                    .index(1)));

    get_version();
}

fn get_version() {
    let repo = match Repository::discover(".") {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open git repository: {}", e),
    };

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

        if num_parents > 1 {
            // This is merge commit, so increment revision
            cmd_stack.push(VersionCmd::IncPatch);
        } else {
            if let Ok(note) = repo.find_note(Some(&notes_ref), commit.id()) {
                if let Some(message) = note.message() {
                    if let Some(command) = match_message_to_cmd(&message) {
                        cmd_stack.push(command);
                    }
                }
            }
        }

        top_commit = commit.parents().next();
    }

    // Run the calculation using the cmd stack we have built
    let version = calculate_version(&mut cmd_stack);
    println!("{}", version);
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
    } else if let Some(version_str) = SET_VERSION_MATCHER.find(message) {
        return Some(VersionCmd::SetVersion(String::from(version_str.as_str())));
    } else if let Some(prerelease_label) = SET_PRERELEASE_LABEL_MATCHER.find(message) {
        return Some(VersionCmd::SetPrereleaseLabel(String::from(
            prerelease_label.as_str(),
        )));
    } else if message.contains("autover-clear-prerelease-label") {
        return Some(VersionCmd::ClearPrereleaseLabel);
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
