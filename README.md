nvfancontrol
============

About
-----

**Nvfancontrol** provides dynamic fan control for NVidia graphic cards on Linux
and Windows.

Sometimes it is desirable to control the fan speed of the graphics card using a
custom response curve instead of the automatic setting that is built into the
card's BIOS. Especially in newer GPUs the fan does not kick in below 60°C or a
certain level of GPU utilization. This is a small toy project in Rust to
achieve a more elaborate control over this using either XNVCtrl in Linux or
NVAPI in Windows. It is a work in progress so proceed with caution!

The minimum supported driver version is **352.09**. For GPUs with **multiple
independent cooler control** nvfancontrol will autodetect and apply the provided
response curve to each of the available fans separately.

HowTo
-----

### Building

Pre-built binaries for the latest release are provided however if you want to
build the project from source read along.

#### Prerequisites for Linux

You will need:

* the Rust compiler toolchain, stable >=1.34 or nightly (build)
* XNVCtrl; static (build only) or dynamic (build and runtime)
* Xlib (build and runtime)
* Xext (build and runtime)

Since XNVCtrl supports FreeBSD in addition to Linux these instructions should
also work for FreeBSD without further modifications. However nvfancontrol is
completely untested on FreeBSD (bug reports are welcome).

#### Prerequisites for Windows

You will need:

* the Rust compiler toolchain, stable >=1.15 or nightly. Be adviced that you
need the **MSVC ABI** version of the toolchain not GNU. In order to target the
MSVC ABI for Rust you will also need the [Visual C++ build
tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2017)
package or any recent version of Visual Studio (2015+). If you are using
[rustup](https://www.rustup.rs/) (which you should) you will be warned about
this (build only)
* the [NVAPI libraries](https://developer.nvidia.com/nvapi) (build only).
Depending on which version you are building (x86, x64 or both) place
`nvapi.lib`, `nvapi64.lib` or both in the root of the repository. As `nvapi` is
linked statically there are no runtime dependencies apart from the NVidia
driver.

For both platforms run `cargo build --release`. Upon successful compilation the
executable can be found in `target/release/nvfancontrol`. On Linux the build
tool expects the libraries installed in `/usr/lib` or `/usr/local/lib`. In case
you have libraries installed in different locations export them using the
`LIBRARY_PATH` environment variable (colon separated paths). By default
`libXNVCtrl` will be linked statically. If a static version of `libXNVCtrl` is
not available or you explicitly want it to be linked dynamically add
`--features=dynamic-xnvctrl` to the `cargo` incantation.

### Enable Coolbits (Linux only)

For Linux ensure that Coolbits is enabled from your X11 server settings. To
do so create a file named `20-nvidia.conf` within `/etc/X11/xorg.conf.d/` or
`/usr/share/X11/xorg.conf.d/` (depends on distribution) containing the
following

    Section "Device"
        Identifier "Device 0"
        Driver     "nvidia"
        VendorName "NVIDIA Corporation"
        BoardName  "IDENTIFIER FOR YOUR GPU"
        Option     "Coolbits" "4"
    EndSection

The important bit is the `Coolbits` option. Valid Coolbits values for dynamic
fan control are `4`, `5` and `12`. A sample configuration file is provided.

### Run X11 as root

As of version 465 NVIDIA decided that is a security risk for non-root users
to have access to cooler control and overclocking capabilities. So you will
have to start X11 as root despite almost no distributions and desktop
environment doing so for the past years because it is a massive... security
risk. Trading one security risk for another! To start X11 as root you
will have to add this to your `/etc/X11/Xwrapper.config` (create the file if
it doesn't exist).

```
allowed_users=anybody
needs_root_rights=yes
```

Depending on how your distribution packages X11 you might have to setuid
`/usr/lib/Xorg.wrap` as well. You can do so by running

```
sudo chmod u+s /usr/lib/Xorg.wrap
```

### Use and configure

To run the program just execute the `nvfancontrol` binary. Add the `-d` or
`--debug` argument for more output. To add a custom curve you can provide a
custom configuration file. On Linux create a file named `nvfancontrol.conf`
under the XDG configuration directory (`~/.config` or `/etc/xdg` for per-user
and system-wide basis respectively). On Windows create the file in
`C:\Users\[USERNAME]\AppData\Roaming` instead. The configuration file should
contain pairs of whitespace delimited parameters (Temperature degrees Celsius,
Fan Speed %).
For example

    30    20
    40    30
    50    40
    60    50
    70    60
    80    80

Lines starting with `#` are ignored. You need at least **two** pairs of values.

Bear in mind that for most GPUs the fan speed can't be below 20% or above 80%
when in manual control, even if you use greater values. However, since these
limits are arbitrary and vary among different VGA BIOS you can override it
using the `-l`, or `--limits` option. For example to change the limits to 10%
and 90% pass `-l 10,90`. To disable the limits effectively enabling the whole
range just pass `-l 0`. In addition note that the program by default will not
use the custom curve if the fan is already spinning in automatic control. This
is the most conservative configuration for GPUs that turn their fans off below
a certain temperature threshold. If you want to always use the custom curve
pass the additional `-f` or `--force` argument. To terminate nvfancontrol send
a SIGINT or SIGTERM on Linux or hit Ctrl-C in the console window on Windows.

Although presently nvfancontrol is limited to a single GPU, users can select
the card to modulate the fan operation using the `-g` or `--gpu` switch. GPUs
are indexed from `0`. To help with that option `-p` or `--print-coolers` will
list all available GPUs with their respective coolers.  On Windows coolers are
indexed from `0` for each GPU. On Linux each available cooler on the system is
assigned a unique id.

### Third party interfacing

nvfancontrol offers two ways to dump the output of the program for integration
with third party programs. Using the `-j` option a JSON represantation of the
current data is printed to `stdout`. As all other messages are printed to
`stderr` the data can be parsed by reading new-line delimited data from the
program's `stdout`. If this is not desirable a builtin TCP server is also
provided which can be enabled using the `-t` option. This option can optionally
be followed by a port number (default port is 12125). The server prints the
JSON data through the socket and immediately closes the connection. The message
is always terminated with a new-line character.

### Fan flicker prevention

Due to firmware issues in several RTX series GPUs fans will tend to rapidly
turn on and off at low speeds (fan flickering). To counter this `nvfancontrol`
includes a workaround which is accessible using the `-r` or `--fanflicker`
switch (config. option `fanflicker = [min, max]`). Fan flicker prevention will
attempt to gradually lower or increase the fan speed within a user-specified
range (the *flickering* zone). When the fan speed drops below the `min`
flickering prevention will kick in. As an example for `min = 11` and `max = 38`
curve changes are applied gradually. Above `38%` behaviour is normal and
arbitrary jumps are again allowed.

Bugs and known issues
---------------------
Although nvfancontrol should work with most Fermi or newer NVidia cards it has
been tested with only a handful of GPUs. So it is quite possible that bugs or
unexpected behaviour might surface. In that case please open an issue in the
bug tracker including the complete program output (use the `--debug` option).

RPM reporting for GPUs with multiple fans on Windows is incorrect or totally
wrong because the provided function `NvAPI_GPU_GetTachReading` is limited to a
single fan. There is nothing in the public NVAPI to suggest otherwise.
However, speed in % should work as expected. In any case multiple cooler
support on Windows is not thoroughly tested so bug reports are always welcome!

As mentioned before, nvfancontrol is limited to a single (but selectable) GPU.
The underlying code does support multiple GPUs but exposing this support to the
user-facing program will require possibly breaking alterations to the
configuration file. It will be added eventually.

License
-------
This project is licensed under the
[GPLv3](https://www.gnu.org/licenses/gpl-3.0.html) or any newer.
