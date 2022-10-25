# glctl

`glctl` allows you to interact with gitlab api using an access token or using oauth2
authentication. It offers for the moment only 3 commands but it is easy to add new ones:

  - `archive`: allows to get and extract projects archives from a gitlab instance whithout
  having to install git or a shell or any toolchain (npm, pypi, ...), greatly reducing the
  surface attack and the execution speed. It's specially usefull in containers that needs to
  specialize (really quickly) at initialization and extract a defined set of plugins. We use
  that tool for odoo container.

  - `tags`: allows to protect and protect tags for a defined projects

  - `build`: trigger a pipeline creation for a project and a defined tag

## General use

```
```

## Archive command

```
```

For each projects specified in the config file, connect to a gitlab instance
with a given token and print the latest commit hash of a given branch.

For each projects specified in the config file, connect to a gitlab instance 
and extract the archive.tar.gz of a given tag. The extraction is done from the stream
whithout needing to preliminary download and save the archive on disk.

In update mode, a lock file containing hash of latest commit is used to decide if we need to
extract again archives

## Tags cmd

```
```

## Build cmd

```
```

## Configuration

The config file looks like

```yaml
host: git.mydomain.com
token: xxxxxxxxxx 
oauth2:
  id: yyyyyy
  secret: zzzzzz
```

The token is a regular gitlab access token with read privilege.  For the the oauth2, you need
to define a new OAuth application inside your gitlab instance `/admin/applications`
