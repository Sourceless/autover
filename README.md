# autover

Automatic semantic versioning for your git project.

## Installation
1. Download one of the latest releases, or compile it yourself using `cargo build`.
2. Run `autover init` in the repo that you want to start versioning in.
3. (Optionally) set your app version with `autover set <version>`
4. Push to remote!

## How does it work?
`autover` is an app that **calculates** the version of your project using its git history, specifically by putting some data in the project's `git notes`.

Since it uses git notes, you do need to push the notes refs. `autover init` sets your git config such that it will push 

## Usage
### Getting the current version

Calling autover with no args will return the current version
``` sh
$ autover
v1.2.3-alpha
```

### Manually setting the version
If you're using `autover` for the first time, and it's taking over versioning
responsibilities from your team's humans, you probably already have a version
number.

You can use `autover set` to tell autover to just start counting from that
version.

``` sh
$ autover set v1.2.3-alpha
```

### Help

``` sh
$ autover help
autover 
Laurence Pakenham-Smith <laurence@sourceless.org>
Automatic calculatable versions

USAGE:
    autover [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --count-patch <COUNT_METHOD>    Choose the counting method from merge (default), commit, or manual.

SUBCOMMANDS:
    clear    Clear the current commit of any manual version changes
    fetch    Fetch version changes from the remote repository
    help     Prints this message or the help of the given subcommand(s)
    init     Set up repository to auto-push version changes
    major    Increments the major version
    minor    Increments the minor version
    patch    Increments the patch version (manual COUNT_METHOD only)
    push     Push version changes to the remote repository
    set      Override the current version
    tag      Set or clear the prerelease tag
```

### Updating the version
#### Increment patch version
There's a few different ways to do this, two automatic and one manual.
##### Increment patch on merge commit (default)
By default, `autover` will increment the patch number when it sees a merge commit.

##### Increment patch on non-merge commits
Invoking `autover -c commit` will tell `autover` to count regular commits as patch increments.

##### Increment patches manually

``` sh
$ autover patch
1.2.3-alpha -> 1.2.4-alpha
```

#### Increment minor version
``` sh
$ autover minor
1.2.4-alpha -> 1.3.0-alpha
```

#### Increment major version
``` sh
$ autover major
1.3.0-alpha -> 2.0.0-alpha
```

#### Change the tag
``` sh
$ autover tag rc1
2.0.0-alpha -> 2.0.0-rc1
```

#### Clear the tag
``` sh
$ autover tag
2.0.0-rc1 -> 2.0.0
```

#### Undoing changes to the current commit

``` sh
$ autover clear
```

### Other commands
#### Ensuring notes get fetched and pushed automatically

``` sh
$ autover init
```

This command adds a couple of lines to `.git/config` which ensure that any `git notes` added by `autover` are pushed and fetched with your normal workflow. If you're having any problems with versions not updating on remote or not pulling them down, this is probably the issue.

#### Manually pushing/fetching version/notes
`autover` offers the `fetch` and `push` commands, if you need to fetch or push note refs manually, for instance during a ci run.

### Caveats
Currently autover only supports *one* command per note (and thus per commit)

It also depends on a full git history being available, currently, so if you are using `autover` in ci and fetching with depth 0, then it will not function as expected.

## Rationale
### Why use `git notes`?
The original mvp of this app used markers in commits, but there were a few things that I didn't like about that approach:
* It leaves a lot of rubbish in commits
* It encourages people to make empty commits that do nothing but increment the version.
