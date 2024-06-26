# gil

[TOC]

`gil` aims to ease interactions between a local git repository and its GitLab counterpart, and
to achieve that goal:

- it uses oauth2 authentication: meaning that if your browser is connected to GitLab no password is
  requested, and

- tries to infer reasonable default arguments based on repository information (project name,
  tag, ...) and the GitLab context (pipeline status, ...), so you don't have unnecessary words to type
  for the common cases,
  
- shows the least amount of information.

It offers for the moment only few commands that are part of my regular workflows and spared me a lot
of time from the web interface. It is easy to add new commands though as the application skeleton
is in place, so feel free to fork or send PR.

  - `archive`: allows extracting projects archives from a GitLab instance without having to install
    git or a shell or any tool chain (npm, pypi, ...), greatly reducing the surface attack and 
    execution speed. It's specially useful in containers that need to specialize really quickly at
    initialization and extract a defined set of archives.

  - `tags`: allows (un)protecting tags

  - `branches`: allows (un)protecting branches

  - `pipeline`: triggers a pipeline creation, see status, get log, cancel and retry
  
  - `project`: manage project (info, archive, unarchinve)

  - `token`: manage GitLab token (useful in scripts to auto-provision and renew tokens)

  - `keys`: manage GitLab ssh keys (useful in scripts to auto-provision and renew keys)

  - `users`: manage GitLab users (useful in scripts to get the username from oidc workflow)

It works on Windows and with multiple GitLab hosts.

## General use

```
gil 0.9.0

Usage: gil [-c <config>] [-v] [-o] [-u] [--color <color>] [--no-cache] <command> [<args>]

Interact with Gitlab API

Options:
  -c, --config      configuration file containing gitlab connection parameters
  -v, --verbose     more detailed output
  -o, --open        try to open links whenever possible
  -u, --url         show urls
  --color           color mode: auto (default), always or never
  --no-cache        don't save oidc login to cache
  --help            display usage information

Commands:
  tags              Manage project tags
  branches          Manage project branches
  pipeline          Manage project pipeline
  archive           Handle project archives
  project           Display information about project
  token             Manage user tokens
  keys              Manage user keys
  users             Manage users
```

## Modus operandi

If no clue is given from the command line arguments, `gil` tries to gather information by
first locating the git root directory, starting from the working directory and checking parents
directories if necessary :

1. The name of the GitLab project to work with is derived from a remote URL it finds in the git
   configuration. The authentication configuration is searched in the configuration file.

2. It then tries to find a remote reference to apply the operations upon using the following
   heuristic :

   1. in case several tags are pointing to the commit[^1], try to find the greatest SemVer tag (x.y.z)
   
   2. if this doesn't work, tries to find the latest tag with describe
   
   3. it this doesn't work then use the current branch name
  
[^1]: My containers build scripts (`Containerfile`) are generally just installing packages
(see [A better way to build containers images](https://itsufficient.me/blog/alpine-container/#containerfile-can-be-dumber)).
The version of the package to install is given by the CI/CD script with an `ARG` directive. As a
consequence the `Containerfile` is not changing very often, and I can end up having a lot of
different versions pointing to the same commit.

### Basic workflows

Say you are in a project repository hosted on your GitLab instance. The project has a CI/CD configuration
that triggers a build for each commit with a protected tag. If you push a new tag :

```bash
git commit -am 'chore(version): 0.10.7'
git tag 0.10.7
git push --tags
```

You can check immediately that the triggered pipeline is correctly running with :

```bash
gil pipeline status
```
```
 Pipeline 4089 (alpine / dendrite @ 0.10.7 = d1d77b7c) [4 seconds ago] - Pending
- Job 8151 build_apk (build) - Pending
- Job 8152 package_apk (deploy) - Created
- Job 8153 deploy_apk (deploy) - Created
- Job 8154 downstream_container (.post) - Created 
```

If you want to check log :

```bash
gil pipeline log
```
```
 Pipeline 4089 (alpine / dendrite @ 0.10.7 = d1d77b7c) [1 minute ago] - Running
- Job 8151 build_apk (build) [1:43s] - Running

Log for job 8151 - Running


$ sudo apk update
fetch https://apk.itsufficient.me/3.16/main/x86_64/APKINDEX.tar.gz
fetch https://dl-cdn.alpinelinux.org/alpine/v3.16/main/x86_64/APKINDEX.tar.gz
fetch https://dl-cdn.alpinelinux.org/alpine/v3.16/community/x86_64/APKINDEX.tar.gz
v3.16.2-410-g2acdfa21ca [https://dl-cdn.alpinelinux.org/alpine/v3.16/main]
v3.16.2-409-g65f55e662e [https://dl-cdn.alpinelinux.org/alpine/v3.16/community]
OK: 17631 distinct packages available 
```

Based on the latest pipeline status it will display the lasted failed, successful or
running job to show you only what matters.

Check the log with all headers collapsed :

```bash
gil pipeline log -H
```

```
Pipeline 4089 (alpine / dendrite @ 0.10.7 = d1d77b7c) [18 hours ago] - Success
- Job 8151 build_apk (build) [5:18s] - Success
- Job 8152 package_apk (deploy) [17s] - Success
- Job 8153 deploy_apk (deploy) [17s] - Success
- Job 8154 downstream_container (.post) [15s] - Success

Log for job 8154 - Success

Running with gitlab-runner 15.4.0 (43b2dc3d)
  on gitlab-runner-795d5987d8-c7785 8sD4PLPV

> Preparing the "kubernetes" executor [prepare_executor] < [0s]

> Preparing environment [prepare_script] < [10s]

> Getting source from Git repository [get_sources] < [1s]

> Downloading artifacts [download_artifacts] < [1s]

> Executing "step_script" stage of the job script [step_script] < [1s]

> Cleaning up project directory and file based variables [cleanup_file_variables] < [1s]
```

At least if you want to list the last 10 pipeline of the project

```bash
gil pipeline list
```

```
 Pipelines for alpine / dendrite
- Pipeline 4089 (alpine / dendrite @ 0.10.7 = d1d77b7c) [1 day ago] - Success
- Pipeline 4054 (alpine / dendrite @ 0.10.6 = 651195dd) [4 days ago] - Success
- Pipeline 3955 (alpine / dendrite @ 0.10.4 = 7f558e55) [2 weeks ago] - Success
- Pipeline 3921 (alpine / dendrite @ 0.10.3 = 0c7ac91c) [2 weeks ago] - Success
- Pipeline 3901 (alpine / dendrite @ 0.10.3 = a791cd67) [3 weeks ago] - Success
- Pipeline 3899 (alpine / dendrite @ 0.10.3 = cce3be6b) [3 weeks ago] - Success
- Pipeline 3881 (alpine / dendrite @ 0.10.2 = a5ac8e8e) [3 weeks ago] - Success
- Pipeline 3874 (alpine / dendrite @ 0.10.2 = 49120d34) [3 weeks ago] - Success
- Pipeline 3843 (alpine / dendrite @ 0.10.1 = acad0de7) [1 month ago] - Failed
- Pipeline 3842 (alpine / dendrite @ 0.10.1 = 5fb73da0) [1 month ago] - Canceled 
```

You can open the project page on a new tab in your browser :

```bash
gil -o project
```

For some reasons, I like to rewrite history to fix some typos and rewrite tags when code doesn't
change. For that you need first to unprotect the (`*`) tags from the project :

```bash
gil tags unprotect
```

Then force push, delete tag (local and remote), re-tag, re-protect and push.

To manually trigger a build :

```bash
gil pipeline create
```

Sometimes you want to quickly extract a project archive :

```bash
gil archive extract -r -p group/project 0.5.0
```

## Archive command

```
gil 0.9.0

Usage: gil archive extract [<ref_>] [-p <project>] [-b <batch>] [-s <strip>] [-r] [-d <dir>] [-k] [-u]

Get and extract archives

Positional Arguments:
  ref_              reference (tag or branch) to extract an archive from

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

In batch mode, a YAML configuration file is used to specify the list of project/tags to extract the
archives from:

```yaml
group1/project1: 0.1.0
group2/project2: 0.2.0
```

The archive extraction is done from the stream without needing to download and save the
archive on disk.

In update mode, a lock file containing the hash of the latest commit is used to decide if we need to
re-extract archives.

## Tags command

```
gil 0.9.0

Usage: gil tags <command> [<args>]

Manage project tags

Options:
  --help            display usage information

Commands:
  protect           Protect a project tag(s)
  unprotect         Unprotect a project tag(s)
```

Allow switching on and off tags protection. Without argument, it will (un)protect all tags (matching `*`).

## Pipeline command

```
gil 0.9.0

Usage: gil pipeline <command> [<args>]

Manage project pipeline

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
gil 0.9.0

Usage: gil pipeline log [<id>] [-p <project>] [-r <ref>] [-s <section>] [-j <job-id>] [-a] [-h] [-H]

Get log from a job

Positional Arguments:
  id                the pipeline id

Options:
  -p, --project     the project which owns the pipeline
  -r, --ref         reference (tag or branch)
  -s, --section     a name that partially match the section name(s) to show in
                    the log: step_script (default)
  -j, --job-id      the job id to extract the job log from
  -a, --all         show all sections
  -h, --headers     show section headers
  -H, --only-headers
                    show only section headers (all collapsed)
  --help            display usage information
```

By default, it shows only the section named `step_script` (which corresponds to the script section in
`.gitlab-ci.yml`). With `-h` sections headers appears as separated (colored) lines starting with `>`
an ending with `<` if they are collapsed. The section IDs are indicated between brackets.

To show all sections (uncollapsed) with headers :

```bash
gil pipeline log -h -a
```

To show only the `step_script` section with all other sections collapsed :

```bash
gil pipeline log -h
```

To show only the sections which names contain `prepare` :

```bash
gil pipeline log -h -s prepare
```

Depending on the `color` mode, all colors (ANSI codes) may be striped out from the log.

There is no follow option, but you can achieve a similar effect with

```bash
watch -c 'gil --color always pipeline log | tail --lines $((LINES))'
```

## Token command

A generic command used to automatically provision tokens in script.

```
gil 0.9.0

Usage: gil token <command> [<args>]

Manage user tokens

Options:
  --help            display usage information

Commands:
  list              List tokens
  create            Create a new token
  revoke            Delete a token
  rotate            Rotate token
```

## Keys command

A generic command used to automatically provision ssh keys in script.

```
gil 0.9.0

Usage: gil keys <command> [<args>]

Manage user keys

Options:
  --help            display usage information

Commands:
  list              List keys
  add               Add a key
  delete            Delete a key
```

## Users command

For now just a simple command to retrieve the username of the currently logged user

```
gil 0.9.0

Usage: gil users <command> [<args>]

Manage users

Options:
  --help            display usage information

Commands:
  current           Get current user name
```

## Project command

Show info about a project (mainly to open the project's page in browser), and archive/unarchive
a project

```
gil 0.9.0

Usage: gil project [-p <project>] <command> [<args>]

Manage projects

Options:
  -p, --project     the project to protect tags from
  --help            display usage information

Commands:
  info              Display information about project
  archive           Archive project
  unarchive         Unarchive project
```

## Configuration

The configuration is searched from these places :

1. `GIL_CONFIG` environment variable

2. `.gil_config.yaml` in the working directory

3. `config.yaml` inside the config directory (OS dependent). For Linux, it is
   `~/.config/gil/config.yaml`, For Windows it is
   `C:\Users\myuser\AppData\Roaming\ITSufficient\Gil\config`

For access token authentication for a host `git.mydomain.com`, the configuration file looks like :

```yaml
hosts:
  git.mydomain.com:
    ca: ca.crt
    token: xxxxxxxxxx
```

The token is a regular GitLab access token with API privilege.

For OIDC authentication, it looks like :

```yaml
hosts:
  git.mydomain.com:
    ca: ca.crt
    token:
      id: yyyyyy
      secret: zzzzzz
      redirect-port: 8888
```

Several hosts can be defined.

You need to define a new OAuth application inside your GitLab instance (at `/admin/applications`)
with an `api` scope and `http://localhost:8888` as the redirect URI (change to match `redirect-port`
in config file) and copy the ID and secret to the configuration file.

On successful login, the short-lived token is saved under the cache directory to speedup consecutive
command invocations unless you specified `--no-cache`. When expired it is renewed automatically
by following the OIDC authentication flow, without requesting a password if your browser is still
connected to GitLab.

---
