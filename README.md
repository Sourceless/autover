# avtover

Automatic versioning software

## How does it work?

`avtover` is an app that **calculates** the version of your project using its git
history.

There are a two simple rules:
* The version starts at 0.0.0
* Every merge commit increments the revision number (e.g. 0.0.0 -> 0.0.1)

All other tracking is done using git notes, a lesser known feature of git. Unlike
commit messages, notes are not an intrinsic part of the commit; they are purely
supplemental - meaning if you every switch away from `avtover`, there won't be
a trail of shame throughout your commit history.

## Assumptions
`avtover` is designed to work with the following assumptions in mind:
* You want to increment your revision count when a branch is merged
* You create a merge commit when you merge a branch (e.g. merging a PR)

## Usage
### Getting the current version

Calling avtover with no args will return the current version
``` sh
$ avtover
v1.2.3-alpha
```

### Manually setting the version
If you're using `avtover` for the first time, and it's taking over versioning
responsibilities from your team's humans, you probably already have a version
number.

You can use `avtover set` to tell avtover to just start counting from that
version.

``` sh
$ avtover set v1.2.3-alpha
```

### Help

``` sh
$ avtover --help
...help text goes here...
```

### Updating the version
#### Increment minor version
``` sh
$ avtover minor
v1.3.0-alpha
```

#### Increment major version
``` sh
$ avtover major
v2.0.0-alpha
```

#### Change the tag
``` sh
$ avtover tag rc1
v2.0.0-rc1
```

#### Clear the tag

``` sh
$ avtover tag -r
v2.0.0
```

`

### Strict updates
`avtover` forces idempotence by default - that is, if on the same commit you
`avtover major` twice in a row, the result will be the same as if it were only
applied once.

You can override this behaviour with the `-f` flag.
