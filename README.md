# Razer Blade Control Utlity

The goal of this project is to build a cross-platform tool for controlling Razer laptops BIOS settings without using Synapse. One of the biggest benefits is the ability to set your laptop to silent mode while on battery, which significantly improves battery life.

> **Note**: The `razer-tray` GUI application is currently not supported on Linux due to system tray library limitations.

# Current Support
- **2023 Blades** (14, 15, 16)
- **2024 Blades** (14, 16, 18)

<details>
<summary>Blade 18 requires additional setup steps:</summary>

#### Due to firmware changes in 2024 models, the Blade 18 requires these additional steps:

### 1. Essential Kernel Parameters
Add these parameters to your bootloader configuration:
```bash
acpi_enforce_resources=lax acpi_osi='!Windows 2022' acpi_backlight=vendor
```

- systemd-boot: Edit `/boot/loader/entries/linux-cachyos.conf`
- GRUB: Edit `/etc/default/grub` then run `sudo update-grub`

### systemd-boot full Example:
```bash
sudo nano /boot/loader/entries/linux-cachyos.conf

title Linux Cachyos
options root=UUID=a758-db363 rw rootflags=subvol=/@ zswap.enabled=0 nowatchdog splash acpi_enforce_resources=lax acpi_osi='!Windows 2022'
linux /vmlinuz-linux-cachyos
initrd /initramfs-linux-cachyos.img

sudo mkinitcpio -P
sudo bootctl update
```

### 2. udev Rules for Device Access
```bash
echo 'SUBSYSTEM=="usb", ATTR{idVendor}=="1532", MODE="0666"' | sudo tee /etc/udev/rules.d/99-razer.rules
sudo udevadm control --reload-rules
```
Or
```bash
sudo tee /etc/udev/rules.d/99-razer.rules <<EOF
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1532", MODE="0666"
SUBSYSTEM=="platform", DRIVER=="ec_sys", MODE="0660", GROUP="users"
EOF

sudo usermod -aG users $USER
sudo udevadm control --reload
sudo udevadm trigger
# Log out and back in
```

### 3. Permanent EC Access Service

```bash
sudo tee /etc/systemd/system/razerec.service >/dev/null <<END
[Unit]
Description=Razer EC Setup
After=sysinit.target

[Service]
Type=oneshot
ExecStart=/usr/bin/bash -c 'echo 1 > /sys/module/ec_sys/parameters/write_support'
ExecStart=/usr/bin/bash -c 'chmod g+rw /sys/kernel/debug/ec/ec*/io'
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
END

sudo systemctl daemon-reload
sudo systemctl enable --now razerec.service
```

### 4. Verefication
```bash
# Check EC write support
cat /sys/module/ec_sys/parameters/write_support  # Should be 1

# Test hardware access
razer-cli auto info
```

### Troubleshooting
- ACPI errors in dmesg: Verify kernel parameters
- Permission denied: Recheck udev rules and group membership
- No hardware effect: Test EC write manually:
```bash
echo -ne '\x03' | sudo dd of=/sys/kernel/debug/ec/ec0/io bs=1 seek=110 conv=notrunc
```
</details>

# Current Features
- Performance modes (including overclock)
- Fan control (RPM setting in manual mode)
- Lid logo modes (off, static, breathing)
- Keyboard brightness control
- Battery health optimizer
- Lighting always-on control

## Usage

```bash
Usage: razer-cli <COMMAND>

Commands:
  auto       Automatically detect supported Razer device and enable device specific features
  manual     Manually specify PID of the Razer device and enable all features
  enumerate  List discovered Razer devices
  help       print the help commands

Options:
  -h, --help     Print help
  -V, --version  Print version


# Detect and control automatically
razer-cli auto perf mode balanced
razer-cli auto fan manual
razer-cli auto fan rpm 4000

razer-cli auto fan auto

# Manual device selection
razer-cli manual -p 0x02B8 info

# List supported devices
razer-cli enumerate
```

## Reverse Engineering

Read about the reverse engineering process for Razer Blade 16 in [data/README.md](data/README.md). You can follow the steps and adjust the utility for other Razer laptops.

Run `razer-cli enumerate` to get PID.
Then `razer-cli -p 0xPID info` to check if the application works for your Razer device.

Special thanks to

- [tdakhran](https://github.com/tdakhran) who created the first version of the tool
- [razer-ctl](https://github.com/tdakhran/razer-ctl) the original project that did the absurd amount of work to get this going
- [openrazer](https://github.com/openrazer) for [Reverse-Engineering-USB-Protocol](https://github.com/openrazer/openrazer/wiki/Reverse-Engineering-USB-Protocol)
- [Razer-Linux](https://github.com/Razer-Linux/razer-laptop-control-no-dkms) for USB HID protocol implementation
