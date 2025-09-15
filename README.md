 
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
sudo udevadm control --reload-rules
sudo udevadm trigger

# Make driver executable
chmod +x target/release/v1600p


# References
Marcus Vinícius Belfort, https://github.com/marvinbelfort.

Tool that enables expanded mode for the tablet, by DigiMend. https://github.com/DIGImend/10moons-tools

Learning about the possibility of creating user-space drivers. https://github.com/alex-s-v/10moons-driver

This code is a combination of the two above, with some improvements.
