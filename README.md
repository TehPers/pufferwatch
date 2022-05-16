# Pufferwatch

A CLI application for filtering and monitoring SMAPI logs.

For command usage, run `pufferwatch -h`.

## Installation

### GitHub releases

You can download a precompiled binary for your platform from [the releases page][releases]. There
are precompiled binaries for the following platforms:

- Windows x86-64 bit
- Linux x86-64 bit
- Mac OS X x86-64 bit

Currently, there is no precompiled binary for ARM. In this case, you can compile pufferwatch
yourself. See [building from source](#building-from-source) for more information.

After downloading the binary, copy it into any directory and open a terminal (Powershell, bash,
Terminal, etc). In the directory you copied the binary to, run the following command:

```sh
./pufferwatch --help
```

You can also add the installation directory to your `PATH` environment variable to make it easier
to execute.

### Building from source

To build pufferwatch from source, you will need to install the following dependency:

- [rustup](https://rustup.rs/): A tool for installing Rust and Cargo.

Building and installing Pufferwatch through Cargo is easy:

```bash
cargo install --git https://github.com/TehPers/pufferwatch
```

After cargo finishes installing pufferwatch, you can use `pufferwatch --help` to see how to use the
application.

## Usage

By default, pufferwatch will open your last SMAPI log. You can specify a different log file by
including the path to the file:

```sh
pufferwatch "path/to/your/SMAPI-latest.txt"
```

You can also specify a remote location by adding `--remote`:

```sh
pufferwatch --remote "https://smapi.io/log/yourlogurl?format=RawDownload"
```

The remote URL must be raw text, and cannot contain HTML. If you are using a log uploaded to
`smapi.io`, make sure to add `?format=RawDownload` to the end of the URL.

Pufferwatch can also follow an existing play session by using `--follow`. If you'd rather use
pufferwatch to launch SMAPI and use pufferwatch as your terminal instead of SMAPI's default
terminal, you can also use `--execute`:

```sh
pufferwatch --execute "path/to/your/StardewModdingAPI.exe"
# SMAPI arguments must come after --
pufferwatch --execute "path/to/your/StardewModdingAPI.exe" -- --mods-dir "your/mods/directory"
```

Run `pufferwatch --help` for more information on how to use the application.

## License

This repository is dual licensed under [The MIT License](./LICENSE-MIT) or
[Apache License v2.0](./LICENSE-APACHE). You may choose which license you wish to use.

[releases]: https://github.com/TehPers/pufferwatch/releases
