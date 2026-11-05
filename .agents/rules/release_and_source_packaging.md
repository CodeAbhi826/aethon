---
description: Rule to enforce automatic source code zip packaging (~/Downloads/aethon_latest.zip and aethon-v<version>-source.zip) upon every update and release
alwaysApply: true
---

# Source Code Packaging & Update Synchronization Rule

Whenever an update, bugfix, or release is completed for Aethon:

1. **Mandatory Source Archive Packaging**:
   - Immediately export the latest committed code (`HEAD`) to:
     - `/home/abhin/Downloads/aethon-v<version>-source.zip`
     - `/home/abhin/Downloads/aethon_latest.zip`
   - Command:
     ```bash
     git archive --format=zip HEAD -o /home/abhin/Downloads/aethon-v<version>-source.zip
     cp /home/abhin/Downloads/aethon-v<version>-source.zip /home/abhin/Downloads/aethon_latest.zip
     ```

2. **Full Repository & Version Synchronization**:
   - Ensure all changes are committed and pushed to `origin/main` on GitHub (`https://github.com/CodeAbhi826/aethon`).
   - Ensure `CHANGELOG.md`, `Cargo.toml`, `tauri.conf.json`, and `package.json` reflect the exact updated version number.

3. **Explicit Reporting**:
   - Always specify the exact file paths to both source zip files and the updated AppImage in the final summary response to the user.
