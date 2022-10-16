<div align="center">
    <h1>tmt</h1>
    <p>
        <sup>
            A cross-platform
                <b>T</b>emperature
                <b>M</b>onitoring
                <b>T</b>ool
            to manage and monitor your system's thermals.
        </sup>
    </p>
</div>

## What is this?

TMT can stand for two things: **T**emperature **M**onitoring **T**ool 
and **T**emperature **Management** **T**ool. It aims to support all
popular operating systems and hardware, and provide a unified interface
to manage and monitor your system's thermals.

This specific repository includes two things:

- The core interface, known as `tmt_core`: this is a Rust crate that
  provides a unified interface to manage and monitor your system's
  thermals, such as reading temperatures and setting fan speeds.
  You can directly use this in your own projects, or create your
  custom user interface on top of it.

- The TUI, or Terminal User Interface. This crate is housed in the root
  of this repository, and uses `tmt_core` to provide an extremely lightweight
  yet elegant interface in the terminal, along with a few other features such
  as setting a custom fan curve.

## What can TMT do?

At its core, TMT can:

- Read system temperatures, such as those from the CPU, GPU, and
  many other sensors provided by your system.
- Provide up-to-date statistics about your system apart from thermals:
  RAM, CPU usage, and Fan speeds are supported if your system provides
  these values.
- Monitor and manage your system's fans, including reading the current
  speed each individual fan is running at, and overriding fan speeds.
  - The TUI also supports setting a custom fan curve, which can be
    used to set a custom fan speed automatically based on the current 
    temperature.

## Limitations

What *can't* TMT do?

- If your system is virtualized, your system probably does not provide
  any sensor data what-so-ever to TMT, meaning that TMT likely will
  not be able to read any temperatures.
- In *most*, if not *all* cases, you must be running as root to use TMT 
  (on Windows, Administrator). Overriding fan speeds will *definitely* require root
  access.
- TMT may not support the newest hardware immediately, since many implementations
  for specific hardware must be hardcoded.
- TMT is still in heavy development. Expect bugs and crashes.

## What makes TMT stand out?

There are many other tools out there similar to TMT - so what makes TMT
stand out?

- **TMT is cross-platform.** TMT supports Windows, Linux, and macOS, and
  will likely support more platforms in the future.
- **TMT is lightweight.** TMT has a small binary size and uses very little
  memory, and is designed to be as lightweight as possible.
- **Full Windows support.** But you just stated that TMT is cross-platform.
  True, but many other similar tools are *also* cross-platform. However,
  TMT fully supports Windows hardware, 
