# Running on windows using WSL

On windows 11, this works out of the box. In windows 10, you might need to install and start a display server to show the UI on windows.

## Guide

- Install wsl
  You can do this by running `wsl --install` in the command line.
- Install gtk dependencies
  Enter the WSL shell by running `wsl`.
  Then, run the command `sudo apt update && sudo apt install libgtk-4-1 libgtk-4-dev`
- Finally, download the linux binary from the releases section on this repo, locate it in wsl and run `./deconz-desktop-linux-x86`


<img width="1603" height="782" alt="image" src="https://github.com/user-attachments/assets/d7098a96-cd11-4987-941c-b34755a94d71" />
Deconz-Client running on windows 11 in wsl2.
