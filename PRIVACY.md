# Privacy Policy for Phaedra

No data about your device(s) or Phaedra usage leave your device.

## Data Maintained by Phaedra

Phaedra maintains some historical data, such as recent searches or action
usage, in some of its overlays such as the debug overlay and character
selector, in order to make your usage more convenient. It is used only
by the local process, and care is taken to limit access for the associated
files on disk to only your local user identity.

Phaedra tracks the output from the commands that you have executed in
a scrollback buffer.  At the time of writing, that scrollback buffer
is an in-memory structure that is not visible to other users of the machine.
In the future, if phaedra expands to offload scrollback information to
your local disk, it will do so in such a way that other users on the
same system will not be able to inspect it.

## macOS and Data permissions

On macOS, when a GUI application that has a "bundle" launches child processes
(eg: Phaedra, running your shell, and your shell running the programs which you
direct it to run), any permissioned resource access that may be attempted by
those child processes will be reported as though Phaedra is attempting to
access those resources.

The result is that from time to time you may see a dialog about Phaedra
accessing your Contacts if run a `find` command that happens to step through
the portion of your filesystem where the contacts are stored.  Or perhaps you
are running a utility that accesses your camera; it will appear as though
Phaedra is accessing those resources, but it is not: there is no logic within
Phaedra to attempt to access your contacts, camera or any other sensitive
information.

## Update Checking

By default, once every 24 hours, phaedra makes an HTTP request to GitHub's
release API in order to determine if a newer version is available and to
notify you if that is the case.

The content of that request is private between your machine and GitHub.  The
contributors to Phaedra cannot see inside that request and therefore cannot
infer any information from it.

If you wish, you can disable update checking. See
https://github.com/PaleRoses/phaedra/blob/main/docs/config/lua/config/check_for_updates.md for
more information on that.

When update checks are enabled, the request targets:
https://api.github.com/repos/PaleRoses/phaedra/releases/latest

## Third-Party Builds

The above is true of the phaedra source code and the binaries produced by
phaedra's CI and made available from https://github.com/PaleRoses/phaedra
and its releases.

If you obtained a pre-built phaedra binary from some other source be aware that
the person(s) building those versions may have modified them to behave
differently from the source version.
