nvfancontrol
============

About
-----

**Nvfancontrol** provides dynamic fan control for NVidia graphic cards in Linux
and probably other unix systems that the NVidia Control API (XNVCtrl) supports.

Sometimes it is desirable to control the fan speed of the graphics card using a
custom response curve instead of the automatic setting that is built into the
card's BIOS. Especially in newer GPUs the fan does not kick in below 60°C or a
certain level of GPU utilization. This is a small toy project in Rust to
achieve a more elaborate control over this using the fairly extensive XNVCtrl
API for unix systems that NVidia provides. It is a work in progress so proceed
with caution!

You need at least driver version **352.09** to use this program. The program
currently supports single GPU configurations.

HowTo
-----

### Compile

You will need:
* recent **nightly** version of the Rust compiler
* static version of libXNVCtrl installed at /usr/lib (`libxvncrtl-dev` package
on Debian/Ubuntu)
* nvidia binary driver (like `nvidia-352`)
* xlib (`libx11-xcb-dev` package on Debian/Ubuntu)
* Xext (`libxext-dev` package on Debian/Ubuntu)

If libXNVCtrl.a is installed in a
different directory edit `src/nvctrl/Makefile` to point to the correct path.
The run `cargo build --release`. The executable can be found at
`target/release/nvfancontrol`. The XNVCtrl, xlib and Xext libraries are required
only for building the program, not running it.

### Use and configure

To run the program just execute the `nvfancontrol` binary. Add the `-d` or
`--debug` argument for more output. To add a custom (temperature, fan speed %)
curve create a configuration file `nvfancontrol.conf` under the XDG
configuration directory. On linux this is typically `~/.config/` or `/etc/xdg/`
for per-user and system-wide basis respectively. The configuration file should
contain pairs of whitespace delimited parameters (Temperature degrees Celsius,
Fan Speed %). For example

    30    20
    40    30
    50    40
    60    50
    70    60
    80    80

Lines starting with `#` are ignored. The custom parameters must contain at
least **two** pairs of values.

Bear in mind that for most GPUs the fan speed can't be below 20% or above 80%
when in manual control, even if you use greater values. Also note that the
program by default will not use the custom curve if the fan is already spinning
in automatic control. This is the most conservative configuration for GPUs that
turn their fans off below a certain temperature threshold. If you want to
always use the custom curve provide the additional `-f` or `--force` argument.

License
-------
This project is licensed under the
[GPLv3](https://www.gnu.org/licenses/gpl-3.0.html) or any newer.
