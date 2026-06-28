#!/usr/bin/env python3
"""Interactive serial console for the luposbox VirtualBox VM.

Lupos only reads console input from the serial port (no runtime PS/2 keyboard
driver), so interactive login happens over ttyS0, not the VBox graphical
window. The VM's serial port is configured as a host UNIX-socket server
(`VBoxManage modifyvm luposbox --uartmode1 server <path>`); VirtualBox creates
the socket once the VM is running. This bridges your terminal to that socket in
raw mode so the login prompt behaves like a normal TTY.

Usage:
    1. Start the VM:  VBoxManage startvm luposbox        (or the GUI)
    2. In a terminal: python3 scripts/lupos-console.py
    3. Log in:        root / lupos        (also user: lupos / lupos)
    Press Ctrl-] to detach.
"""
import os
import select
import socket
import sys
import termios
import tty

SOCK = os.environ.get("LUPOS_SERIAL_SOCK", "/tmp/lupos-vbox-serial.sock")
DETACH = 0x1D  # Ctrl-]


def main() -> int:
    if not os.path.exists(SOCK):
        sys.stderr.write(
            f"socket {SOCK} not found - is luposbox running?\n"
            "Start it first: VBoxManage startvm luposbox\n"
        )
        return 1
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        s.connect(SOCK)
    except OSError as e:
        sys.stderr.write(f"could not connect to {SOCK}: {e}\n")
        return 1

    sys.stdout.write("[connected to luposbox serial - press Ctrl-] to detach]\r\n")
    sys.stdout.flush()
    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    try:
        tty.setraw(fd)
        while True:
            r, _, _ = select.select([s, fd], [], [])
            if s in r:
                data = s.recv(4096)
                if not data:
                    break
                os.write(sys.stdout.fileno(), data)
            if fd in r:
                data = os.read(fd, 4096)
                if not data or data[0] == DETACH:
                    break
                s.sendall(data)
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)
        s.close()
        sys.stdout.write("\r\n[detached]\r\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
