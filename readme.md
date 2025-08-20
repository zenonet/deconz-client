# Deconz Client

This is a minimal GTK desktop application for controlling smarthome lights connected to a deconz server.
The application is written in Rust with the `gtk-rs` GTK bindings.

The app fetches a list of lights connected to your server and allows you to control their state (on/off) and their color.

## Demo Mode

To use the Deconz client, you of course need a deconz server. If you just want to test the functionality of the client quickly though, you can use demo mode. This is a separate mode where light state is saved internally and you can see requests the client would make in standard output.

## About Windows Compatibility

Unfortunately, the windows build is unstable. For some, when opening the color picker, it crashes. I am not able to diagnose the issue. I already spend hours trying to diagnose this and I think at this point it's better to just say windows is not officially supported. You can try running deconz-client in [WSL 2](https://learn.microsoft.com/en-us/windows/wsl/tutorials/gui-apps)

## Features:

- Login using push-link button
- Listing all available lights
- Searching in the list of lights
- Reading on/off state and color of lights
- Turning lights on and off
- Changing lights colors

<img width="656" height="688" alt="Screenshot_20250819_001311" src="https://github.com/user-attachments/assets/d60f8e7c-1c7f-41d1-b34e-9d8d9db2ac24" />
<img width="791" height="579" alt="ColorPicker" src="https://github.com/user-attachments/assets/b5f3d3d9-f07f-4dad-983d-4ceec1c50962" />

https://github.com/user-attachments/assets/88b540c0-af45-4218-b413-4c6b2a95ca7e

