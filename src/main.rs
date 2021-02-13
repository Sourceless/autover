use git2::Repository;

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
}
