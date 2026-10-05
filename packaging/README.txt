repository-intelligence (repository-intelligence)
=================================================

Searches local code and returns verbatim source lines for a question. This archive holds the `repository-intelligence` command-line tool, built from the tagged release.

Install
-------
Put the `repository-intelligence` binary (`repository-intelligence.exe` on Windows) in a directory on your PATH,
or use the installer, which does that for you and checks the checksum:

  macOS / Linux:  curl -fsSL https://raw.githubusercontent.com/rustfuture/repository-intelligence/main/install.sh | sh
  Windows:        irm https://raw.githubusercontent.com/rustfuture/repository-intelligence/main/install.ps1 | iex

Then:

  repository-intelligence --version
  repository-intelligence --help

Verify this download
--------------------
Each archive has a matching .sha256 file on the release page:

  sha256sum -c repository-intelligence-<version>-<target>.tar.gz.sha256      (Linux)
  shasum -a 256 -c repository-intelligence-<version>-<target>.tar.gz.sha256  (macOS)

Docs and source: https://github.com/rustfuture/repository-intelligence
License: see LICENSE
