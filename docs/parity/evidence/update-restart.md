# Update progress and restart

Review of the merged updater identified hidden-notification polling, save ordering,
stale startup arguments and Linux executable-renaming problems (issue #32).

Busy update work now requests a repaint at the end of every app frame, regardless
of balloon visibility. The typed `app.prepareRestart` command saves all dirty
documents and then the current workspace. Save failures prevent process launch;
spawn failures keep the current window open and report an error in the status bar.
Restart uses the installation target captured before binary replacement and the
current project root, letting workspace restoration reopen tabs.

Automated save/error evidence:

```sh
cargo test -p archipel-ide-core restart_preparation
cargo test -p archipel-ide-core welcome_session_can_prepare_for_restart
```

Manual release-build checks still required: silently check for updates while idle,
dismiss a download balloon and verify it completes without input; edit a file,
switch projects, install an update and restart; check that saved text and the
current project return and Help/About reports the new version. Repeat on Linux
to verify the restart executes `archipel-ide`, not `archipel-ide.old`.

No live self-update was installed during this change's validation. The separate
release-tag publication bug is tracked in issue #33.
