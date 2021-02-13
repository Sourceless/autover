use git2::{Repository, Commit, Oid};

fn main() {
    let repo = match Repository::discover(".") {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open git repository: {}", e),
    };

    let head = repo.head().unwrap();
    let name = head.name().unwrap();

    println!("{}", name);

    // TODO: get commits, and recurse down popping any relevant notes on to a
    //       stack, as well as noting any merge commits, because we want to
    //       count them as well
    let top_commit = head.peel_to_commit().expect(&format!("Couldn't find a commit on ref {}", name)[..]);
    let mut version_stack = Vec::new();

    traverse_commits(&top_commit, &mut version_stack);
}

fn traverse_commits(commit: &Commit, version_stack: &mut Vec<Oid>) {
    let commit_id = commit.id();
    version_stack.push(commit_id);

    let parents = commit.parents();

    for parent in parents {
        println!("{:?}", parent);
    }
}
