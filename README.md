 
# Linux Driver for VINSA 1600 Plus Drawing Tablet

Native Linux driver for the VINSA 1600 Plus drawing tablet with full pressure sensitivity and button support.

## ✨ Features
- ✅ Full pressure support (8192 levels)
- ✅ Adjustable sensitivity settings
- ✅ Mouse/Tablet mode toggle (B button)
- ✅ Dynamically adjustable work area ([ ] buttons)
- ✅ All programmable buttons
- ✅ No sudo required (udev rules included)
- ✅ Desktop launcher with custom icon

## 📦 Installation

```bash
# Clone the repository
git clone https://github.com/your-username/vinsa-1600-driver.git
cd vinsa-1600-driver

# Build the driver
cargo build --release

# Install udev rules (no sudo needed)
sudo cp 99-vinsa-tablet.rules /etc/udev/rules.d/

SUBSYSTEM=="usb", ATTR{idVendor}=="08f2", ATTR{idProduct}=="6811", MODE="0666"
SUBSYSTEM=="input", GROUP="input", MODE="0666"
KERNEL=="uinput", MODE="0666", GROUP="input"


sudo udevadm control --reload-rules
sudo udevadm trigger

# Make driver executable
chmod +x target/release/v1600p
```


## References
- [marvinbelfort](https://github.com/marvinbelfort) - Initial research
- [DIGImend/10moons-tools](https://github.com/DIGImend/10moons-tools) - Expanded mode enablement
- [alex-s-v/10moons-driver](https://github.com/alex-s-v/10moons-driver) - User-space driver approach
