# glctl

`glctl` allows you to interact with gitlab api using using oauth2 authentication and tries to
be smart when working from a git repository to extract the project path and the tag when needed.

It offers for the moment only 3 commands but it is easy to add new ones:

  - `archive`: allows to extract projects archives from a gitlab instance whithout having
    to install git or a shell or any toolchain (npm, pypi, ...), greatly reducing the surface
    attack and the execution speed. It's specially usefull in containers that needs to specialize
    (really quickly) at initialization and extract a defined set of arquives (we use that tool
    for odoo container).

  - `tags`: allows to protect and unprotect tags

  - `pipeline`: triggers a pipeline creation

## General use

```
```

## Archive command

```
```

In batch mode, a configuration is used to specify the list of project/tags to extract the
arquives from:

```yaml
group1/project1: 0.1.0
group2/project2: 0.2.0
```

For each projects, connect to a gitlab instance and extract the archive.tar.gz of a given
tag. The extraction is done from the stream whithout needing to preliminary download and save
the archive on disk.

In update mode, a lock file containing the hash of latest commit is used to decide if we need
to reextract archives.


## Tags command

```
```

## Pipeline command

```
```

## Configuration

The config file looks like

```yaml
host: git.mydomain.com
# choose aither token or oauth2
token: xxxxxxxxxx 
oauth2:
  id: yyyyyy
  secret: zzzzzz
```

The token is a regular gitlab access token with read privilege. For the the oauth2, you need to
define a new OAuth application inside your gitlab instance `/admin/applications` with `api` scope.
