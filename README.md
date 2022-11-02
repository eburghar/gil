# glctl

`glctl` aims to ease interactions between a local git repository and its Gitlab counterpart. As such
it is not a [glab](https://gitlab.com/gitlab-org/cli) replacement, nor a tool to administrate Gitlab
from the command line.

To achieve that goal (WIP):

- it uses oauth2 authentication: meaning that if your browser is connected to Gitlab no password is
  requested, and

- try to infer reasonable default arguments based on  repository informations (project name,
  tag, ...) and the Gitlab context (pipeline status, ...) to always have the minimal words to type
  for the common cases.

It offers for the moment only 3 commands that are part of my regular workflows, but it is easy to
add new ones. Feel free to fork or send PR.

  - `archive`: allows to extract projects archives from a Gitlab instance whithout having to install
    git or a shell or any toolchain (npm, pypi, ...), greatly reducing the surface attack and the
    execution speed. It's specially useful in containers that need to specialize really quickly at
    initialization and extract a defined set of arquives.

  - `tags`: allows to protect and unprotect tags

  - `pipeline`: triggers a pipeline creation, see status, get log, cancel and retry

## General use

```
glctl 0.5.0

Usage: glctl [-c <config>] [-v] [-o] [--color <color>] [--no-cache] <command> [<args>]

Interact with Gitlab API

Options:
  -c, --config      configuration file containing gitlab connection parameters
  -v, --verbose     more detailed output
  -o, --open        try to open links whenever possible
  --color           color mode: auto (default), always or never
  --no-cache        don't save oidc login to cache
  --help            display usage information

Commands:
  tags              Manage project tags
  pipeline          Manage project pipeline
  archive           Handle project archives
```

## Examples

`glctl` try to find a remote reference to apply the operations upon, and unless you are giving
explicitely a reference with a command line argument, it will try to find one using the folowing
heuristic :

1. in case several tags are pointing to the commit[^1], try to find the greatest semver tag (x.y.z)

1. if this doesn't work, tries to find the latest tag with describe

1. it this doesn't work then use the branch name

```bash
gctl pipeline create
```

Will create a new pipeline and shows immediately its status

```bash
glctl -o pipeline status
```

Will show the status of the latest pipeline along its jobs and try to open the pipeline page in
the browser

```bash
glctl -o pipeline log
```

Depending of the state of the latest pipeline, this will show the latest log in the same state. By
default it shows only the `script` part of the job (a section named `step_script`), and hides the
collapsed sections.

```bash
glctl tags protect
```
  
Will protect all the tags (`*`) on the project

[^1]: My containers build scripts (`Containerfile`) are generally just installing packages
(see [A better way to build containers images](https://itsufficient.me/blog/alpine-container/#containerfile-can-be-dumber)).
The version of the package to install is given by the CI/CD script with an `ARG` directive. As a
consequence the `Containerfile` is not changing very often, and I can endup having a lot of
different versions pointing to the same commit.

## Archive command

```
glctl 0.5.0

Usage: glctl archive extract [<tag>] [-p <project>] [-b <batch>] [-s <strip>] [-r] [-d <dir>] [-k] [-u]

Get and extract archives

Positional Arguments:
  tag               tag to extract archive from

Options:
  -p, --project     the project to extract archive from
  -b, --batch       batch mode: yaml file containing a list of project and tag
                    to extract
  -s, --strip       strip first n path components of every entries in archive
                    before extraction
  -r, --rename      rename first directory of the archive to the name of the
                    project
  -d, --dir         destination directory
  -k, --keep        skip extraction of projects if a directory with same name
                    already exists. by default destination directory is removed
                    before extraction
  -u, --update      update based on packages.lock file
  --help            display usage information
```

In batch mode, a yaml configuration file is used to specify the list of project/tags to extract the
arquives from:

```yaml
group1/project1: 0.1.0
group2/project2: 0.2.0
```

The archive extraction is done from the stream whithout needing to preliminary download and save the
archive on disk.

In update mode, a lock file containing the hash of latest commit is used to decide if we need to
reextract archives.


## Tags command

```
glctl 0.5.0

Usage: glctl tags <command> [<args>]

Manage project tags

Options:
  --help            display usage information

Commands:
  protect           Protect a project tag(s)
  unprotect         Unprotect a project tag(s)
```

Allow to switch on and off tags protection. Without argument it will (un)protect all tags (matching `*`).

## Pipeline command

```
glctl 0.5.0

Usage: glctl pipeline <command> [<args>]

Manage pro. If no tag
is found it will use the HEAD commit id.ject pipeline

Options:
  --help            display usage information

Commands:
  status            Get pipeline status
  create            Create a new pipeline
  cancel            Cancel a pipeline
  retry             Retry a pipeline
  log               Get log from a job
```

### log sub command

```
glctl 0.5.0

Usage: glctl pipeline log [<id>] [-p <project>] [-s <step>] [-a]

Get log from a job

Positional Arguments:
  id                the job id to extract the job log from

Options:
  -p, --project     the project which owns the pipeline
  -s, --step        the section to show in the log: step_script (default)
  -a, --all         show all sections
  --help            display usage information
```

The log command shows only the section `step_script` and hide collapsed sections. To
show everything do

```bash
glctl pipeline log -a
```

The name of the section is indicated in the log between brackets. Depending on the `color` mode, all
colors may be removed from the log.

## Configuration

The configuration is looked from theses places in that order :

- `GLCTL_CONFIG` environment variable

- `.glctl_. If no tag
is found it will use the HEAD commit id.config.yaml` in the working directory

- `config.yaml` inside the config directory (OS dependent). For Linux it is
   `~/.config/glctl/config.yaml`

For access token authentication, the configuration file looks like

```yaml
host: git.mydomain.com
token: xxxxxxxxxx
```

The token is a regular Gitlab access token with api privilege.

For oidc authentication, it looks like

```yaml
host: git.mydomain.com
token:
  id: yyyyyy
  secret: zzzzzz
  redirect-port: 8888
```

You need to define a new OAuth application inside your Gitlab instance (at `/ admin/applications`)
with an `api` scope and `http://localhost:8888` as the redirect URI (change to match `redirect-port`
in config file) and copy the id and secret to the configuration file.

On successful login, the short-lived token is saved under the cache directory to speedup consecutive
command invocations unless you specified `--no-cache`. When expired it is renewed automatically
by folowing the oidc authentication flow, without requesting a password if your browser is still
connected to Gitlab.

---