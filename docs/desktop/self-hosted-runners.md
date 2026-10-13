# Self-hosted runners

The heavy desktop checks run on two self-hosted runners, one Ubuntu and one Windows, in
[Desktop self-hosted validation](../../.github/workflows/desktop-self-hosted.yml). Both keep
their build folder between runs, so only changed crates compile.

## What runs where

| Check | Where | When |
| --- | --- | --- |
| fmt, tests, clippy, packaging tests, ledgers | self-hosted Linux and Windows | every push |
| Real worker, SDK, managed, portable, preview, timeline, audio parity | self-hosted Linux and Windows | every push |
| Package (test build) and isolated-account presentation, input and source click | self-hosted Linux | every push |
| Package (test build) | self-hosted Windows | every push |
| M2 virtual audio stress (advisory) | self-hosted Linux | every push |
| fmt, tests, clippy for pull requests from other repositories | hosted Ubuntu ([Desktop CI](../../.github/workflows/desktop.yml)) | those pull requests |

The repository is public, so the self-hosted workflow runs only on `push` and **Run workflow**.
Never add a `pull_request` trigger to it: anyone could then run code on these machines, as root.
A branch you push gets the self-hosted checks whether or not it has a pull request.

## Ubuntu runner

Use Ubuntu 24.04 (x86-64). An admin installs the packages once; packages installed with `apt`
are system-wide, so the runner user sees them.

1. Install the packages as root:

   ```sh
   sudo apt-get update
   sudo apt-get install -y build-essential cmake pkg-config git nasm clang libclang-dev \
     libssl-dev libfontconfig1-dev libasound2-dev libxkbcommon-x11-dev libwayland-dev \
     libegl1-mesa-dev libgl1-mesa-dev libxcb1-dev libxcb-shape0-dev libxcb-xfixes0-dev \
     libx11-xcb-dev libzstd-dev mesa-vulkan-drivers xvfb xdotool python3 python3-pil \
     pulseaudio pulseaudio-utils curl unzip
   ```

   - `nasm`, `make`, `gcc` and `git` build the pinned static FFmpeg (once per change of
     `desktop/scripts/build-native-ffmpeg.py`).
   - `xvfb`, `xdotool` and `python3-pil` run the presentation check on a virtual display.
   - `pulseaudio` and `pulseaudio-utils` are only for the advisory audio stress. The test
     starts its own PulseAudio as the runner user, so no audio service has to be running.
   - `iproute2` and `util-linux` (`ip`, `unshare`, `setpriv`) are part of a standard Ubuntu
     install; the isolated-account check uses them.

2. Create the runner user and register the runner. In the repository, open **Settings →
   Actions → Runners → New self-hosted runner → Linux x64** and follow the download and
   `./config.sh` steps as that user. Keep the default labels `self-hosted`, `Linux` and `X64`.
   Then install it as a service, as root, from the runner folder:

   ```sh
   sudo ./svc.sh install <runner-user>
   sudo ./svc.sh start
   ```

   Rust needs no manual install: the job installs the pinned toolchain for the runner user.

3. Give the runner user passwordless sudo. The isolated-account presentation check runs as root
   to create the account and network namespaces, then starts the app as `nobody` without
   network:

   ```sh
   echo "<runner-user> ALL=(ALL) NOPASSWD:ALL" | sudo tee /etc/sudoers.d/github-runner
   sudo chmod 440 /etc/sudoers.d/github-runner
   sudo visudo -c
   ```

   This means every push runs repository code that can become root on this machine, so use a
   machine dedicated to CI and keep write access to the repository to people you trust. The
   check also makes the parent folders of the runner's work folder readable by all users, and
   leaves its evidence owned by `nobody`; the job removes that with `sudo` when it ends.

4. Check the machine as the runner user. This is the same check the job runs first; it
   prints nothing when everything is there:

   ```sh
   for tool in nasm cmake make git pkg-config gcc python3 Xvfb xdotool ip unshare setpriv pulseaudio pactl; do
     command -v "$tool" >/dev/null || echo "missing: $tool"
   done
   for package in libclang-dev libssl-dev libasound2-dev; do
     dpkg -s "$package" >/dev/null 2>&1 || echo "missing: $package"
   done
   python3 -c "import PIL" 2>/dev/null || echo "missing: python3-pil"
   sudo -n /usr/bin/python3 -c "" 2>/dev/null || echo "missing: passwordless sudo"
   ```

   If a package is missing, the job stops at **Check Linux native prerequisites** and prints the
   `apt-get install` line to run.

## Windows runner

The Windows job expects, for the runner's service account: `rustup` in `%USERPROFILE%\.cargo\bin`,
LLVM in `C:\Program Files\LLVM` (for `libclang.dll`), Python 3 reachable through the `py`
launcher, and the Visual Studio C++ build tools. It downloads the pinned FFmpeg build itself and
checks its SHA-256. Register it with the labels `self-hosted`, `Windows` and `X64`.

## Disk and housekeeping

Both runners keep state in the runner's tool cache folder (`<runner>/_work/_tool` on Linux):

- `fframes-target`: the build folder. It grows with every toolchain or dependency change. When
  disk runs low, delete it; the next run rebuilds everything once.
- `fframes-ffmpeg/<key>` (Linux): the FFmpeg install for one version of its build script. Old
  keys can be deleted.

If a Linux run is killed while the presentation check runs, `/tmp/.X94-lock` can stay behind and
the next check refuses to start. It is owned by root: remove it with `sudo rm /tmp/.X94-lock` when
no `Xvfb :94` process is running.

## Test builds

Every self-hosted run uploads a test build, kept for 14 days:

- `studio-test-x86_64-pc-windows-msvc`: the packaged Windows app with the SDK.
- `studio-test-x86_64-unknown-linux-gnu`: the packaged Linux app with the SDK, plus the
  presentation and audio evidence.

The run's summary page links the zip directly and shows a `gh run download` command for it. To
download the newest Windows test build from `main`:

```sh
gh run download "$(gh run list -R khanhthanhdev/fframes-desktop -w "Desktop self-hosted validation" -b main -s success -L 1 --json databaseId -q '.[0].databaseId')" -R khanhthanhdev/fframes-desktop -n studio-test-x86_64-pc-windows-msvc -D studio-test
```

Replace `main` with a branch name to get that branch's build. Test builds are named
`studio-test-*` so a release never picks them up: [Desktop Release](../../.github/workflows/desktop-release.yml)
collects only `fframes-studio-*` packages, which are built on hosted runners.
