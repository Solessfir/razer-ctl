#!/bin/bash
set -e

# 1. Create razer group if not exists
if ! getent group razer >/dev/null; then
    sudo groupadd razer
fi

# 2. Add user to group if not already member
if ! id -nG "$USER" | grep -qw razer; then
    sudo usermod -aG razer "$USER"
    echo "Added $USER to razer group - requires logout/login"
fi

# 3. Install udev rules
#     Remove need for sudo with razer-cli
#     Grant read/write access to Razer HID devices
sudo tee /etc/udev/rules.d/99-razer.rules > /dev/null <<EOL
# Razer Blade 18 (2024)
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1532", ATTRS{idProduct}=="02b8", MODE="0660", GROUP="razer"

# Generic Razer devices
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1532", MODE="0660", GROUP="razer"
EOL

# 4. Install EC service
#    Allows direct Embedded Controller access
#    Needed for low-level hardware control
sudo tee /etc/systemd/system/razerec.service >/dev/null <<EOL
[Unit]
Description=Razer EC Setup
After=sysinit.target

[Service]
Type=oneshot
ExecStart=/bin/sh -c "echo 1 > /sys/module/ec_sys/parameters/write_support"
ExecStart=/bin/sh -c "chmod g+rw /sys/kernel/debug/ec/ec*/io"
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
EOL

# 5. Kernel parameters instructions
echo "MANUAL STEP REQUIRED: Kernel parameters configuration"
echo "Please add these parameters to your bootloader:"
echo "acpi_enforce_resources=lax acpi_osi='!Windows 2022' acpi_backlight=vendor"
echo ""
echo "For systemd-boot (CachyOS):"
echo "1. Edit your boot config:"
echo "   sudo nano /boot/loader/entries/linux-cachyos.conf"
echo "2. Add to the 'options' line:"
echo "   acpi_enforce_resources=lax acpi_osi='!Windows 2022' acpi_backlight=vendor"
echo "3. Example result:"
echo "   options root=... acpi_enforce_resources=lax acpi_osi='!Windows 2022' acpi_backlight=vendor"
echo ""

# 6. Apply changes
sudo udevadm control --reload-rules
sudo udevadm trigger
sudo systemctl daemon-reload
sudo systemctl enable --now razerec.service >/dev/null 2>&1 || true

echo "Installation complete."
echo "NOTE: Some changes require REBOOT to take effect"