# autover

Automatic versioning software

## How does it work?

`autover` is an app that **calculates** the version of your project using its git
history.

There are a two simple rules:
* The version starts at 0.0.0
* Every merge commit increments the revision number (e.g. 0.0.0 -> 0.0.1)

All other tracking is done using git notes, a lesser known feature of git. Unlike
commit messages, notes are not an intrinsic part of the commit; they are purely
supplemental - meaning if you every switch away from `autover`, there won't be
a trail of shame throughout your commit history.

## Assumptions
`autover` is designed to work with the following assumptions in mind:
* You want to increment your revision count when a branch is merged
* You create a merge commit when you merge a branch (e.g. merging a PR)

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
$ autover --help
...help text goes here...
```

### Updating the version
#### Increment minor version
``` sh
$ autover minor
v1.3.0-alpha
```

#### Increment major version
``` sh
$ autover major
v2.0.0-alpha
```

#### Change the tag
``` sh
$ autover tag rc1
v2.0.0-rc1
```

#### Clear the tag

``` sh
$ autover tag -r
v2.0.0
```

`

### Strict updates
`autover` forces idempotence by default - that is, if on the same commit you
`autover major` twice in a row, the result will be the same as if it were only
applied once.

You can override this behaviour with the `-f` flag.
