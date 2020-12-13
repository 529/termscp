# Changelog

- [Changelog](#changelog)
  - [0.1.2](#012)
  - [0.1.1](#011)
  - [0.1.0](#010)

---

## 0.1.2

Released on 13/12/2020

- General performance and code improvements
- Improved symlinks management
- Possibility to abort file transfers
- Enhancements:
  - File explorer:
    - When file index is at the end of the list, moving down will set the current index to the first element and viceversa.
    - Selected file has now colourful background, instead of foreground, for a better readability.
- Keybindings:
  - `E`: Delete file (Same as `DEL`); added because some keyboards don't have `DEL` (hey, that's my MacBook Air's keyboard!)
  - `Ctrl+C`: Abort transfer process

## 0.1.1

Released on 10/12/2020

- enhancements:
  - password prompt: ask before performing terminal clear
  - file explorer:
    - file names are now sorted ignoring capital letters
    - file names longer than 23, are now cut to 20 and followed by `...`
    - paths which exceed tab size in explorer are elided with the following formato `ANCESTOR[1]/.../PARENT/DIRNAME`
- keybindings:
  - `I`: show info about selected file or directory
  - Removed `CTRL`; just use keys now.
- bugfix:
  - prevent panic in set_progress, for progress values `> 100.0 or < 0.0`
  - Fixed FTP get, which didn't finalize the reader
- dependencies:
  - updated `textwrap` to `0.13.0`
  - updated `ftp4` to `4.0.1`

## 0.1.0

Released on 06/12/2020

- First release