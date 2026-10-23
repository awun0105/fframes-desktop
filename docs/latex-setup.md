# LaTeX setup for fframes-kit formulas

The `latex` feature of `fframes-kit` (`MathTex`, `Tex`, `ChemEq`, see
[components/components.md](components/components.md#formulas)) runs the TeX installed on
your machine. It needs:

- `latex` (or `xelatex` / `lualatex` for `TexCompiler::XeLatex` / `LuaLatex`);
- `dvisvgm`.

MiKTeX and TeX Live both ship them. Formulas that are already in a video's TeX cache render
without TeX, so only the person who adds or changes a formula needs it.

## Windows: MiKTeX

1. Install MiKTeX from <https://miktex.org/download>, or run `choco install miktex`.
2. Enable automatic package installation, so a missing package installs itself instead of
   opening a dialog that stalls the render until the timeout:

   ```powershell
   initexmf --set-config-value "[MPM]AutoInstall=1"
   ```

3. Check it in a new terminal:

   ```powershell
   latex --version
   dvisvgm --version
   ```

If the terminal does not find `latex`, the kit still looks in the default MiKTeX folders:

- `%LOCALAPPDATA%\Programs\MiKTeX\miktex\bin\x64` (an install for the current user);
- `C:\Program Files\MiKTeX\miktex\bin\x64`.

### Without a working installer

If the installer cannot download (proxies, a broken Chocolatey package), MiKTeX's
command-line setup works offline in two steps. Get `miktexsetup_standalone.exe` from the
same download page, then:

```powershell
miktexsetup_standalone --remote-package-repository=https://mirrors.mit.edu/CTAN/systems/win32/miktex/tm/packages/ --local-package-repository=C:\miktex-repo --package-set=basic download
miktexsetup_standalone --local-package-repository=C:\miktex-repo --package-set=basic --shared=no install
```

- Any mirror listed on <https://ctan.org/mirrors> works for `--remote-package-repository`.
  MiKTeX only accepts mirrors that are registered, so pick one from that list.
- `--shared=no` installs for the current user.
- The first formula that uses a package outside the basic set installs it. This takes
  20 to 30 s once, for example for mhchem and chemfig.

## Ubuntu: TeX Live (or MiKTeX)

```bash
sudo apt install texlive-latex-extra texlive-science dvisvgm
latex --version && dvisvgm --version
```

`texlive-science` brings mhchem, siunitx and physics. Add `texlive-pictures` for circuitikz,
and `texlive-xetex` or `texlive-luatex` for the other compilers. MiKTeX for Linux works too
(<https://miktex.org/download>). Enable its automatic installation as on Windows.

## macOS: MacTeX

```bash
brew install --cask mactex-no-gui
eval "$(/usr/libexec/path_helper)"   # or open a new terminal
latex --version && dvisvgm --version
```

## Pointing the kit at a TeX folder

`FFRAMES_TEX_BIN=<folder>` makes the kit look for the tools only in that folder. This is
useful when `PATH` is not refreshed yet, or to pick one of several installations.

Pointing it at an empty folder hides TeX. Use that to check that a video renders from its
committed cache alone.

## The cache

- `TexEngine::new(dir)` writes one small `.svg` per formula into `dir`.
  - The file name is a hash of the compiler, the preamble and the source, so a changed
    formula gets a new file.
  - Old files stay until you delete them.
- Commit the directory. Then CI and other machines render without TeX, and the same TeX
  version reproduces the files byte for byte.
- When TeX is missing and a formula is not in the cache, `typeset` returns
  `TexError::NotInstalled`. The error names the formula and says how to install TeX for
  the current system.

## Errors

- **`TexError::Compile`**: the TeX log from the first `!` line, and the whole `.tex` file
  the kit wrote, for an error in the formula.
- **`TexError::Timeout`**: a run took longer than the timeout (120 s,
  `TexEngine::with_timeout`). With MiKTeX this usually means a package dialog is waiting.
  Enable automatic installation (step 2 above).
- **`TexError::Convert`**: `dvisvgm` failed. Its output is in the error.

The kit runs TeX with `-interaction=nonstopmode -halt-on-error` and never with
`--shell-escape`, so a formula cannot run commands.
