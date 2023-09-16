
import re
import fileinput

# Read the current version from the Cargo.toml file
current_version = ""
with open("devprofiler/Cargo.toml", "r") as cargo_file:
    for line in cargo_file:
        match = re.search(r'^version\s*=\s*"(.*?)"', line)
        if match:
            current_version = match.group(1)
            break

# Generate a new version number (increment the patch version)
version_parts = current_version.split('.')
new_patch = int(version_parts[2]) + 1
new_version = f"{version_parts[0]}.{version_parts[1]}.{new_patch}"

# Update the Cargo.toml file with the new version number
in_package_section = False
for line in fileinput.input("devprofiler/Cargo.toml", inplace=True):
    if line.strip() == "[package]":
        in_package_section = True
    elif line.strip().startswith("["):
        in_package_section = False

    if in_package_section:
        line = re.sub(r'^version\s*=\s*".*?"', f'version = "{new_version}"', line.rstrip())
    print(line)

